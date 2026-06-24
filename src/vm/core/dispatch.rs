use std::sync::Arc;
use crate::vm::core::vm::{OpResult, VM};
use crate::vm::value::{Value, TAG_DB, TAG_TBL, TAG_ARR, TAG_MAP, TAG_SET, TAG_STR, TAG_DATE, TAG_JSON, TAG_FIB, TAG_ROW};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;

impl Executor {
    pub fn handle_method_call(
        &mut self,
        dst: u8,
        receiver: Value,
        kind: MethodKind,
        args: &[Value],
        names: Option<&[String]>,
        ip: usize,
        locals: &mut [Value],
        vm_arc: &Arc<VM>,
    ) -> OpResult {
        if matches!(kind, MethodKind::ToStr | MethodKind::ToJson) {
            let res = match kind {
                MethodKind::ToStr => {
                    let s = receiver.to_string();
                    Value::from_string(Arc::new(crate::vm::object::StringObj::new(s.into_bytes())))
                }
                MethodKind::ToJson => {
                    let json = crate::vm::utils::json::value_to_json(&receiver);
                    Value::from_json(Arc::new(crate::vm::object::JsonObj::new(json)))
                }
                _ => unreachable!(),
            };
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            return OpResult::Continue;
        }

        if !receiver.is_ptr() && !receiver.is_date() {
            self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return OpResult::Halt;
        }

        let tag = receiver.tag;
        match tag {
            TAG_DB   => self.handle_database_method(dst, receiver.as_database(), kind, args, names, ip, locals, vm_arc),
            TAG_TBL  => self.handle_table_method(dst, receiver.as_table(), kind, args, names, ip, locals, vm_arc),
            TAG_ARR  => self.handle_array_method(dst, receiver.as_array(), kind, args, names, ip, locals, vm_arc),
            TAG_MAP  => self.handle_map_method(dst, receiver.as_map(), kind, args, names, ip, locals, vm_arc),
            TAG_SET  => self.handle_set_method(dst, receiver.as_set(), kind, args, names, ip, locals, vm_arc),
            TAG_STR  => {
                let s_arc = receiver.as_string();
                self.handle_string_method(dst, &(*s_arc), kind, args, names, ip, locals, vm_arc)
            },
            TAG_DATE => {
                self.handle_date_method(dst, receiver.as_date(), kind, args, names, ip, locals, vm_arc)
            },
            TAG_JSON => self.handle_json_method(dst, receiver.as_json(), kind, args, names, ip, locals, vm_arc),
            TAG_FIB  => self.handle_fiber_method(dst, receiver.as_fiber(), kind, args, names, ip, locals, vm_arc),
            TAG_ROW  => self.handle_row_method(dst, receiver.as_row(), kind, args, names, ip, locals, vm_arc),
            _ => {
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                OpResult::Halt
            }
        }
    }

    pub fn handle_method_call_custom(
        &mut self,
        dst: u8,
        receiver: Value,
        method_name: &str,
        args: &[Value],
        ip: usize,
        locals: &mut [Value],
        vm_arc: &Arc<VM>,
    ) -> OpResult {
        if !receiver.is_ptr() {
            self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            return OpResult::Halt;
        }

        let tag = receiver.tag;
        let n_bytes = method_name.as_bytes();
        match tag {
            TAG_ROW  => self.handle_row_custom(dst, receiver.as_row(), n_bytes, ip, locals, vm_arc),
            TAG_JSON => self.handle_json_custom(dst, receiver.as_json(), n_bytes, args, ip, locals, vm_arc),
            TAG_DB   => {
                let res = crate::vm::core::runtime_ops::RuntimeOps::get_member(receiver, method_name);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                OpResult::Continue
            }
            _ => {
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                OpResult::Halt
            }
        }
    }
}
