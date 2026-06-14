use std::sync::Arc;
use crate::vm::object::StringObj;
use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::{Value, TAG_STR};

impl Executor {
    pub fn handle_string_search<'a>(
        &mut self,
        dst: u8,
        s: &[u8],
        kind: MethodKind,
        args: &[Value],
        ip: usize,
        locals: &mut [Value],
    ) -> OpResult {
        match kind {
            MethodKind::IndexOf => {
                let s_lossy = String::from_utf8_lossy(&s);
                let res = if let Some(v) = args.first() {
                    if v.is_ptr() && v.tag == TAG_STR {
                        let sub_bytes = v.as_string();
                        let sub = String::from_utf8_lossy(&sub_bytes);
                        let idx = s_lossy.find(sub.as_ref()).map(|i| i as i64).unwrap_or(-1);
                        Value::from_i64(idx)
                    } else { Value::from_i64(-1) }
                } else { Value::from_i64(-1) };
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::LastIndexOf => {
                let s_lossy = String::from_utf8_lossy(&s);
                let res = if let Some(v) = args.first() {
                    if v.is_ptr() && v.tag == TAG_STR {
                        let sub_bytes = v.as_string();
                        let sub = String::from_utf8_lossy(&sub_bytes);
                        let idx = s_lossy.rfind(sub.as_ref()).map(|i| i as i64).unwrap_or(-1);
                        Value::from_i64(idx)
                    } else { Value::from_i64(-1) }
                } else { Value::from_i64(-1) };
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Replace => {
                if args.len() != 2 { return OpResult::Halt; }
                let from = args[0].as_string();
                let to   = args[1].as_string();
                if from.is_empty() { 
                    eprintln!("R307: .replace() called with empty 'from'{}", self.current_span_info(ip)); 
                    return OpResult::Halt; 
                }
                let s_str = String::from_utf8_lossy(&s);
                let from_str = String::from_utf8_lossy(&from);
                let to_str = String::from_utf8_lossy(&to);
                let res_str = s_str.replace(from_str.as_ref(), to_str.as_ref());
                let res = Value::from_string(Arc::new(StringObj::new(res_str.into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::StartsWith => {
                if args.is_empty() { return OpResult::Halt; }
                let prefix = args[0].as_string();
                let res = Value::from_bool(s.starts_with(&prefix));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::EndsWith => {
                if args.is_empty() { return OpResult::Halt; }
                let suffix = args[0].as_string();
                let res = Value::from_bool(s.ends_with(&suffix));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => return OpResult::Halt,
        }
        OpResult::Continue
    }
}
