pub mod format;
pub mod search;
pub mod convert;
pub mod concat;

use std::sync::Arc;

use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;

impl Executor {
    pub fn handle_string_method(
        &mut self,
        dst: u8,
        s: &[u8],
        kind: MethodKind,
        args: &[Value],
        _names: Option<&[String]>,
        ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Length | MethodKind::Size => {
                let res = Value::from_i64(s.len() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                OpResult::Continue
            }
            MethodKind::Upper | MethodKind::Lower | MethodKind::Trim => {
                self.handle_string_format(dst, s, kind, locals)
            }
            MethodKind::IndexOf | MethodKind::LastIndexOf | MethodKind::Replace | MethodKind::StartsWith | MethodKind::EndsWith => {
                self.handle_string_search(dst, s, kind, args, ip, locals)
            }
            MethodKind::ToInt | MethodKind::ToFloat => {
                self.handle_string_convert(dst, s, kind, ip, locals)
            }
            MethodKind::Slice | MethodKind::Split => {
                self.handle_string_concat(dst, s, kind, args, ip, locals)
            }
            MethodKind::ToStr => {
                let res = Value::from_string(Arc::new(crate::vm::object::StringObj::new(s.to_vec())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                OpResult::Continue
            }
            _ => {
                eprintln!("Method {:?} not found on String{}", kind, self.current_span_info(ip));
                OpResult::Halt
            }
        }
    }
}
