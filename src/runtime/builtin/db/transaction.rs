use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;
use crate::vm::object::DatabaseObj;
use crate::vm::opcode::MethodKind;
use crate::vm::Executor;

impl Executor {
    pub fn handle_database_transaction(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, kind: MethodKind, ip: usize, locals: &mut [Value]) -> OpResult {
        let sql = match kind {
            MethodKind::Begin => "BEGIN TRANSACTION",
            MethodKind::Commit => "COMMIT",
            MethodKind::Rollback => "ROLLBACK",
            _ => {
                eprintln!("Method {:?} not supported for Database transaction{}", kind, self.current_span_info(ip));
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt;
            }
        };
        let ok = match db_rc.conn.lock().as_ref() {
            Some(conn) => conn.execute(sql, []).is_ok(),
            None => {
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                eprintln!("R401: database connection is closed{}", self.current_span_info(ip));
                false
            }
        };
        let res = Value::from_bool(ok);
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = res;
        OpResult::Continue
    }
}
