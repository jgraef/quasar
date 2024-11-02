pub mod column;
pub mod table;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StorageType {
    Table,
    SparseSet,
    BitSet,
}
