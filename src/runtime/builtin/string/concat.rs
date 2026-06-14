use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::object::{StringObj, ArrayObj};
use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;

impl Executor {
    pub fn handle_string_concat<'a>(
        &mut self,
        dst: u8,
        s: &[u8],
        kind: MethodKind,
        args: &[Value],
        ip: usize,
        locals: &mut [Value],
    ) -> OpResult {
        match kind {
            MethodKind::Slice => {
                if args.len() != 2 { return OpResult::Halt; }
                if !args[0].is_int() || !args[1].is_int() { return OpResult::Halt; }
                let start = args[0].as_i64();
                let end   = args[1].as_i64();
                
                let s_str = String::from_utf8_lossy(&s);
                let chars: Vec<char> = s_str.chars().collect();
                let len = chars.len() as i64;
                if start < 0 || end > len || start > end {
                    eprintln!("R303: String.slice out of bounds [{}, {}] for len {}{}", start, end, len, self.current_span_info(ip));
                    self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    return OpResult::Halt;
                }
                let res_str: String = chars[start as usize..end as usize].iter().collect();
                let res = Value::from_string(Arc::new(StringObj::new(res_str.into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                OpResult::Continue
            }
            MethodKind::Split => {
                if args.is_empty() { return OpResult::Halt; }
                let sep_bytes = args[0].as_string();
                let s_str = String::from_utf8_lossy(&s);
                let sep = String::from_utf8_lossy(&sep_bytes);
                let parts: Vec<Value> = s_str.split(sep.as_ref()).map(|p| Value::from_string(Arc::new(StringObj::new(p.to_string().into_bytes())))).collect();
                let res = Value::from_array(Arc::new(RwLock::new(ArrayObj { elements: parts })));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                OpResult::Continue
            }
            _ => {
                eprintln!("Method {:?} not supported in concat/slice{}", kind, self.current_span_info(ip));
                OpResult::Halt
            }
        }
    }
}
