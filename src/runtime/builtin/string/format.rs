use std::sync::Arc;
use crate::vm::object::StringObj;
use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;

impl Executor {
    pub fn handle_string_format<'a>(
        &mut self,
        dst: u8,
        s: &[u8],
        kind: MethodKind,
        locals: &mut [Value],
    ) -> OpResult {
        match kind {
            MethodKind::Upper => {
                let s_str = String::from_utf8_lossy(s);
                let res = Value::from_string(Arc::new(StringObj::new(s_str.to_uppercase().into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Lower => {
                let s_str = String::from_utf8_lossy(s);
                let res = Value::from_string(Arc::new(StringObj::new(s_str.to_lowercase().into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Trim => {
                let s_str = String::from_utf8_lossy(s);
                let res = Value::from_string(Arc::new(StringObj::new(s_str.trim().to_string().into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => return OpResult::Halt,
        }
        OpResult::Continue
    }
}
