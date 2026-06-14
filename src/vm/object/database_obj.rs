use std::sync::Arc;
use parking_lot::{RwLock, Mutex};
use std::collections::HashMap;
use crate::vm::value::Value;

// Database connection object representation.
pub struct DatabaseObj {
    pub conn: Arc<Mutex<rusqlite::Connection>>,
    pub engine: String,
    pub path: String,
    pub tables: Arc<RwLock<HashMap<String, Value>>>,
}

impl Drop for DatabaseObj {
    fn drop(&mut self) {
        let tables = self.tables.read();
        for (_, val) in tables.iter() {
            unsafe { val.dec_ref(); }
        }
    }
}
