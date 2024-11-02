use std::cell::UnsafeCell;

use crate::{
    component::{
        ComponentId,
        ComponentInfo,
    },
    entity::Entity,
    storage::column::Column,
    util::sparse_map::{
        ImmutableSparseMap,
        SparseMap,
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
}

#[derive(Debug)]
pub struct Table {
    columns: ImmutableSparseMap<ComponentId, Column>,
    entities: Vec<Entity>,
}

impl Table {
    unsafe fn swap_remove_unchecked(&mut self, row: TableRow) -> Option<Entity> {
        let row_index = row.0 as usize;
        let is_last = row_index == self.entities.len();
        for column in self.columns.values_mut() {
            column.swap_remove_unchecked(row_index);
        }
        self.entities.swap_remove(row_index);
        (!is_last).then(|| self.entities[row_index])
    }

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
        InsertIntoTable {
            table: self,
            index,
            entity,
        }
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
}

pub struct InsertIntoTable<'a> {
    table: &'a mut Table,
    index: usize,
    entity: Entity,
}

impl<'a> InsertIntoTable<'a> {
    pub unsafe fn write_column<T>(&mut self, component_id: ComponentId, value: T) {
        let column = self.table.get_column_mut(component_id).unwrap();
        assert_eq!(column.len(), self.index);
        column.push(value);
    }

    pub unsafe fn finish(self) -> TableRow {
        self.table.entities.push(self.entity);
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
    //table_ids: HashMap<Box<[ComponentId]>, TableId>,
}

impl Default for Tables {
    fn default() -> Self {
        Self {
            tables: vec![TableBuilder::new(0, 0).build()],
        }
    }
}

impl Tables {
    pub fn insert(&mut self, table: Table) -> TableId {
        let table_id = TableId(self.tables.len().try_into().expect("TableId overflow"));

        //self.table_ids.insert(table.component_ids().collect(), table_id);
        self.tables.push(table);

        table_id
    }

    pub fn get(&self, table_id: TableId) -> &Table {
        &self.tables[table_id.index()]
    }

    pub fn get_mut(&mut self, table_id: TableId) -> &mut Table {
        &mut self.tables[table_id.index()]
    }

    pub fn clear(&mut self) {
        self.tables.clear();
    }
}
