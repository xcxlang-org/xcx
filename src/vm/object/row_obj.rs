use std::sync::Arc;
use parking_lot::RwLock;
use super::table_obj::TableObj;

// Row object (reference) representation.
pub struct RowObj {
    pub table: Arc<RwLock<TableObj>>,
    pub row_idx: u32,
}
