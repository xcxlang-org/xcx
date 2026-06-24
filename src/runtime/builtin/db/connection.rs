use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;
use parking_lot::{RwLock, Mutex};
use crate::vm::object::DatabaseObj;
use crate::vm::Executor;

impl Executor {
    pub fn handle_database_init(&mut self, dst: u8, engine: String, path: String, tables: &[String], ip: usize, locals: &mut [Value]) -> OpResult {
        if path.contains("..") || path.starts_with('/') || (path.len() > 1 && path.as_bytes()[1] == b':') {
            panic!("halt.fatal: Security violation - illegal DB path access: {}", path);
        }
        match rusqlite::Connection::open(&path) {
            Ok(conn) => {
                let db_data = Arc::new(DatabaseObj {
                    conn: Arc::new(Mutex::new(conn)),
                    engine: engine,
                    path: path.clone(),
                    tables: Arc::new(RwLock::new(std::collections::HashMap::new())),
                });
                
                // Pre-initialize tables
                for tname in tables {
                    let _ = self.handle_database_ddl(dst, db_data.clone(), tname.clone(), ip, locals);
                }

                let res = Value::from_database(db_data);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                OpResult::Continue
            }
            Err(e) => {
                eprintln!("R401: Failed to open database {}: {}{}", path, e, self.current_span_info(ip));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(false);
                OpResult::Continue
            }
        }
    }

    pub fn handle_database_maintenance(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, _ip: usize, locals: &mut [Value]) -> OpResult {
        let conn = db_rc.conn.lock();
        // WAL checkpoint
        let _ = conn.execute("PRAGMA wal_checkpoint(PASSIVE);", []);
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = Value::from_bool(true);
        OpResult::Continue
    }
}
