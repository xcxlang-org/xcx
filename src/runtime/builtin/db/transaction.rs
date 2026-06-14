use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;
use crate::vm::object::DatabaseObj;
use crate::vm::opcode::MethodKind;
use crate::vm::Executor;

impl Executor {
    pub fn handle_database_transaction(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, kind: MethodKind, ip: usize, locals: &mut [Value]) -> OpResult {
        match kind {
            MethodKind::Begin => {
                let ok = db_rc.conn.lock().execute("BEGIN TRANSACTION", []).is_ok();
                let res = Value::from_bool(ok);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Commit => {
                let ok = db_rc.conn.lock().execute("COMMIT", []).is_ok();
                let res = Value::from_bool(ok);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Rollback => {
                let ok = db_rc.conn.lock().execute("ROLLBACK", []).is_ok();
                let res = Value::from_bool(ok);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => { 
                eprintln!("Method {:?} not supported for Database transaction{}", kind, self.current_span_info(ip));
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt; 
            }
        }
        OpResult::Continue
    }
}
