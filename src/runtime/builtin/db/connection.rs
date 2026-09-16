use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;
use parking_lot::{RwLock, Mutex};
use crate::vm::object::DatabaseObj;
use crate::vm::opcode::MethodKind;
use crate::vm::Executor;

impl Executor {
    pub fn handle_database_init(&mut self, dst: u8, engine: String, path: String, tables: &[String], ip: usize, locals: &mut [Value]) -> OpResult {
        if path.contains("..") || path.starts_with('/') || (path.len() > 1 && path.as_bytes()[1] == b':') {
            panic!("halt.fatal: Security violation - illegal DB path access: {}", path);
        }
        match rusqlite::Connection::open(&path) {
            Ok(conn) => {
                let db_data = Arc::new(DatabaseObj {
                    conn: Arc::new(Mutex::new(Some(conn))),
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

    pub fn handle_database_maintenance(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, kind: MethodKind, _ip: usize, locals: &mut [Value]) -> OpResult {
        let mut conn_guard = db_rc.conn.lock();
        match kind {
            MethodKind::IsOpen => {
                let open = conn_guard.is_some();
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(open);
            }
            MethodKind::Sync => {
                // Bare sync with no table: checkpoint only, connection stays open.
                if let Some(conn) = conn_guard.as_ref() {
                    let _ = conn.execute("PRAGMA wal_checkpoint(PASSIVE);", []);
                }
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(true);
            }
            _ => {
                // Close: checkpoint, then drop the connection so the database
                // file is released (store.delete can remove it afterwards).
                // Idempotent — closing a closed database stays true.
                if let Some(conn) = conn_guard.as_ref() {
                    let _ = conn.execute("PRAGMA wal_checkpoint(TRUNCATE);", []);
                }
                *conn_guard = None;
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(true);
            }
        }
        OpResult::Continue
    }
}
