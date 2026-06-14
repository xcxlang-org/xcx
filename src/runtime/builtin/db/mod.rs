pub mod connection;
pub mod ddl;
pub mod read;
pub mod write;
pub mod delete;
pub mod transaction;

use std::sync::Arc;

use crate::vm::core::vm::{VM, OpResult};
use crate::vm::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;
use crate::vm::object::{DatabaseObj};

impl Executor {
    pub fn handle_database_method(
        &mut self,
        dst: u8,
        db_rc: Arc<DatabaseObj>,
        kind: MethodKind,
        args: &[Value],
        _names: Option<&[String]>,
        ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Table => {
                let name = args[0].to_string();
                self.handle_database_ddl(dst, db_rc, name, ip, locals)
            }
            MethodKind::Execute | MethodKind::Exec | MethodKind::Insert | MethodKind::Truncate | MethodKind::Push | MethodKind::Save => {
                self.handle_database_write(dst, db_rc, kind, args, ip, locals)
            }
            MethodKind::Query | MethodKind::Fetch | MethodKind::QueryRaw => {
                self.handle_database_read(dst, db_rc, args, ip, locals)
            }
            MethodKind::Delete => {
                self.handle_database_delete(dst, db_rc, args, ip, locals)
            }
            MethodKind::Drop | MethodKind::Has => {
                self.handle_database_maintenance_ddl(dst, db_rc, kind, args, ip, locals)
            }
            MethodKind::Begin | MethodKind::Commit | MethodKind::Rollback => {
                self.handle_database_transaction(dst, db_rc, kind, ip, locals)
            }
            MethodKind::Sync => {
                if args.is_empty() {
                    self.handle_database_maintenance(dst, db_rc, ip, locals)
                } else {
                    self.handle_database_sync(dst, db_rc, args, ip, locals)
                }
            }
            MethodKind::Close | MethodKind::IsOpen => {
                // To be implemented in connection.rs if needed for more than maintenance
                self.handle_database_maintenance(dst, db_rc, ip, locals)
            }
            _ => { 
                eprintln!("Method {:?} not supported for Database{}", kind, self.current_span_info(ip)); 
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                OpResult::Halt 
            }
        }
    }
}
