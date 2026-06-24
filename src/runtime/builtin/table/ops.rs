use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;
use crate::vm::object::TableObj;

impl Executor {
    pub fn handle_table_method(
        &mut self,
        dst: u8,
        t_rc: Arc<RwLock<TableObj>>,
        kind: MethodKind,
        args: &[Value],
        names: Option<&[String]>,
        ip: usize,
        locals: &mut [Value],
        vm_arc: &Arc<VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Count | MethodKind::Len | MethodKind::Size | MethodKind::Show | MethodKind::Where | MethodKind::Join | MethodKind::Find => {
                self.handle_table_select(dst, t_rc, kind, args, ip, locals, vm_arc)
            }
            MethodKind::Insert | MethodKind::Add | MethodKind::Save => {
                self.handle_table_insert(dst, t_rc, kind, args, names, locals)
            }
            MethodKind::Update => {
                self.handle_table_update(dst, t_rc, args, locals)
            }
            MethodKind::Delete => {
                self.handle_table_delete(dst, t_rc, args, locals)
            }
            MethodKind::Get | MethodKind::First => {
                self.handle_table_index(dst, t_rc, args, ip, locals, kind == MethodKind::First)
            }
            MethodKind::Fetch | MethodKind::Query => {
                self.handle_table_query(dst, t_rc, kind, args, ip, locals)
            }
            MethodKind::Clear => {
                self.handle_table_clear(dst, t_rc, locals)
            }
            MethodKind::ToJson => {
                self.handle_table_to_json(dst, t_rc, locals)
            }
            _ => { 
                eprintln!("Method {:?} not supported for Table{}", kind, self.current_span_info(ip)); 
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                OpResult::Halt 
            }
        }
    }
}
