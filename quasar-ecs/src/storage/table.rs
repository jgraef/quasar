use std::{
    cell::UnsafeCell,
    collections::HashMap,
};

use crate::{
    component::{
        self,
        ComponentId,
        ComponentInfo,
    },
    entity::{
        ChangedLocation,
        Entity,
    },
    storage::column::Column,
    util::{
        slice_get_mut_pair,
        sparse_map::{
            ImmutableSparseMap,
            SparseMap,
        },
        Joined,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TableId(u32);

impl TableId {
    pub const EMPTY: Self = Self(0);
    pub const INVALID: Self = Self(u32::MAX);

    fn index(&self) -> usize {
        self.0 as usize
    }

    fn from_index(index: usize) -> Self {
        Self(index.try_into().expect("TableId overflow"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TableRow(u32);

impl TableRow {
    pub const INVALID: Self = Self(u32::MAX);

    fn index(&self) -> usize {
        self.0 as usize
    }

    fn from_index(index: usize) -> Self {
        Self(index.try_into().expect("TableRow overflow"))
    }

    pub fn is_invalid(&self) -> bool {
        *self == Self::INVALID
    }

    pub fn is_valid(&self) -> bool {
        !self.is_invalid()
    }
}

#[derive(Debug)]
pub struct Table {
    columns: ImmutableSparseMap<ComponentId, Column>,
    entities: Vec<Entity>,
}

impl Table {
    pub fn get_column(&self, component_id: ComponentId) -> Option<&Column> {
        self.columns.get(&component_id)
    }

    pub fn get_column_mut(&mut self, component_id: ComponentId) -> Option<&mut Column> {
        self.columns.get_mut(&component_id)
    }

    pub fn has_column(&self, component_id: ComponentId) -> bool {
        self.columns.contains_key(&component_id)
    }

    pub fn reserve(&mut self, additional: usize) {
        for (_, column) in &mut self.columns {
            column.reserve(additional);
        }
        self.entities.reserve(additional);
    }

    pub fn num_entities(&self) -> usize {
        self.entities.len()
    }

    pub fn num_components(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        for column in self.columns.values_mut() {
            column.clear();
        }
    }

    pub fn insert(&mut self, entity: Entity) -> InsertIntoTable {
        let index = self.entities.len();
        self.entities.push(entity);
        InsertIntoTable { table: self, index }
    }

    pub fn component_ids(&self) -> impl Iterator<Item = ComponentId> + use<'_> {
        self.columns.iter().map(|(k, _)| k)
    }

    pub unsafe fn get_component<T>(
        &self,
        component_id: ComponentId,
        table_row: TableRow,
    ) -> Option<&T> {
        let column = self.columns.get(&component_id)?;
        Some(&column.get_slice()[table_row.index()])
    }

    pub unsafe fn get_component_mut<T>(
        &mut self,
        component_id: ComponentId,
        table_row: TableRow,
    ) -> Option<&mut T> {
        let column = self.columns.get_mut(&component_id)?;
        Some(&mut column.get_mut_slice()[table_row.index()])
    }

    pub unsafe fn take_component_and_remove_later<T>(
        &mut self,
        component_id: ComponentId,
        table_row: TableRow,
    ) -> Option<T> {
        let column = self.columns.get_mut(&component_id)?;
        Some(column.take_item_and_remove_later(table_row.index()))
    }

    pub unsafe fn move_row<'t>(
        &mut self,
        from_row: TableRow,
        to_table: &'t mut Self,
        entity: Entity,
        mut handle_unmatched: impl MoveRowHandleUnmatched,
    ) -> MoveRowResult<'t> {
        to_table.entities.push(entity);
        let to_row = TableRow::from_index(self.entities.len());

        let swapped = if from_row.is_valid() {
            let from_row_index = from_row.index();
            assert!(
                from_row_index < self.entities.len(),
                "row_index ({from_row_index}) < self.entities.len() ({})",
                self.entities.len()
            );

            let swapped = from_row_index != self.entities.len() - 1;

            let removed_entity = self.entities.swap_remove(from_row_index);
            assert_eq!(removed_entity, entity);

            for (component_id, from_column) in &mut self.columns {
                if let Some(to_column) = to_table.get_column_mut(component_id) {
                    from_column.move_item(from_row_index, to_column);
                }
                else {
                    handle_unmatched.handle(from_column, from_row_index, component_id)
                }
            }

            swapped.then(|| {
                ChangedLocation {
                    entity: self.entities[from_row_index],
                    changed_value: from_row,
                }
            })
        }
        else {
            None
        };

        MoveRowResult {
            swapped,
            insert: InsertIntoTable {
                table: to_table,
                index: to_row.index(),
            },
        }
    }

    pub unsafe fn remove_row(&mut self, row: TableRow) -> Option<ChangedLocation<TableRow>> {
        if row.is_invalid() {
            return None;
        }

        let row_index = row.0 as usize;
        assert!(
            row_index < self.entities.len(),
            "row_index ({row_index}) < self.entities.len() ({})",
            self.entities.len()
        );

        let swapped = row_index != self.entities.len() - 1;

        self.entities.swap_remove(row_index);

        for column in self.columns.values_mut() {
            column.remove_item(row_index);
        }

        swapped.then(|| {
            ChangedLocation {
                entity: self.entities[row_index],
                changed_value: row,
            }
        })
    }
}

pub trait MoveRowHandleUnmatched {
    unsafe fn handle(&mut self, column: &mut Column, row_index: usize, component_id: ComponentId);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MoveRowDropUnmatched;

impl MoveRowHandleUnmatched for MoveRowDropUnmatched {
    unsafe fn handle(&mut self, column: &mut Column, row_index: usize, _component_id: ComponentId) {
        unsafe {
            column.remove_item(row_index);
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MoveRowForgetUnmatched;

impl MoveRowHandleUnmatched for MoveRowForgetUnmatched {
    unsafe fn handle(&mut self, column: &mut Column, row_index: usize, _component_id: ComponentId) {
        unsafe {
            column.forget_item(row_index);
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MoveRowPanicUnmatched;

impl MoveRowHandleUnmatched for MoveRowPanicUnmatched {
    unsafe fn handle(
        &mut self,
        _column: &mut Column,
        _row_index: usize,
        component_id: ComponentId,
    ) {
        panic!("unexpected unmatched column: {component_id:?}");
    }
}

#[derive(Debug)]
pub struct MoveRowResult<'a> {
    pub swapped: Option<ChangedLocation<TableRow>>,
    pub insert: InsertIntoTable<'a>,
}

impl<'a> MoveRowResult<'a> {
    pub fn to_row(&self) -> TableRow {
        self.insert.table_row()
    }
}

#[derive(Debug)]
pub struct InsertIntoTable<'a> {
    table: &'a mut Table,
    index: usize,
}

impl<'a> InsertIntoTable<'a> {
    pub unsafe fn write_column<T>(&mut self, component_id: ComponentId, value: T) {
        let column = if let Some(column) = self.table.get_column_mut(component_id) {
            column
        }
        else {
            let component_ids = self.table.component_ids().collect::<Box<[ComponentId]>>();
            panic!(
                "trying to write to column {component_id:?} to, but table has only columns [{:?}]",
                Joined::new(", ", &component_ids)
            );
        };

        assert_eq!(column.len(), self.index);
        column.push(value);
    }

    pub fn table_row(&self) -> TableRow {
        TableRow::from_index(self.index)
    }
}

#[derive(Debug, Default)]
pub struct TableBuilder {
    columns: SparseMap<ComponentId, Column>,
    row_capacity: usize,
}

impl TableBuilder {
    pub fn new(row_capacity: usize, column_capacity: usize) -> Self {
        Self {
            columns: SparseMap::with_capacity(column_capacity),
            row_capacity,
        }
    }

    pub fn add_column(&mut self, component_info: &ComponentInfo) {
        self.columns.insert(
            &component_info.id(),
            Column::new(component_info.descriptor(), self.row_capacity),
        );
    }

    pub fn reserve_rows(&mut self, additional: usize) {
        self.row_capacity += additional;
        for column in self.columns.values_mut() {
            column.reserve(additional);
        }
    }

    pub fn reserve_columns(&mut self, additional: usize) {
        self.columns.reserve(additional);
    }

    pub fn build(self) -> Table {
        Table {
            columns: self.columns.into(),
            entities: Vec::with_capacity(self.row_capacity),
        }
    }
}

#[derive(Debug)]
pub struct Tables {
    tables: Vec<Table>,
    by_components: HashMap<Box<[ComponentId]>, TableId>,
}

impl Default for Tables {
    fn default() -> Self {
        let mut by_components = HashMap::with_capacity(1);
        by_components.insert(std::iter::empty().collect(), TableId::EMPTY);

        Self {
            tables: vec![TableBuilder::new(0, 0).build()],
            by_components,
        }
    }
}

impl Tables {
    pub fn insert(&mut self, table: Table) -> TableId {
        let table_id = TableId::from_index(self.tables.len());

        if let Some(replaced_table_id) = self
            .by_components
            .insert(table.component_ids().collect(), table_id)
        {
            panic!("tried to insert a table that already exists: replaced: {replaced_table_id:?}, new: {table_id:?}");
        }

        self.tables.push(table);

        table_id
    }

    pub fn get(&self, table_id: TableId) -> &Table {
        &self.tables[table_id.index()]
    }

    pub fn get_mut(&mut self, table_id: TableId) -> &mut Table {
        &mut self.tables[table_id.index()]
    }

    pub fn get_mut_pair(
        &mut self,
        first: TableId,
        second: TableId,
    ) -> Result<(&mut Table, &mut Table), &mut Table> {
        slice_get_mut_pair(&mut self.tables, first.index(), second.index())
    }

    pub fn get_table_id_by_component_ids(&self, component_ids: &[ComponentId]) -> Option<TableId> {
        self.by_components.get(component_ids).copied()
    }

    pub fn clear(&mut self) {
        self.tables.clear();
    }
}
