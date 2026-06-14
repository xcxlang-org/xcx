use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;

impl Executor {
    pub fn handle_string_convert<'a>(
        &mut self,
        dst: u8,
        s: &[u8],
        kind: MethodKind,
        ip: usize,
        locals: &mut [Value],
    ) -> OpResult {
        let s_str = String::from_utf8_lossy(s);
        match kind {
            MethodKind::ToInt => {
                match s_str.trim().parse::<i64>() {
                    Ok(n) => {
                        let res = Value::from_i64(n);
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                    }
                    Err(_) => {
                        eprintln!("halt.error: Cannot convert \"{}\" to Integer{}", s_str, self.current_span_info(ip));
                        return OpResult::Halt;
                    }
                }
            }
            MethodKind::ToFloat => {
                match s_str.trim().parse::<f64>() {
                    Ok(f) => {
                        let res = Value::from_f64(f);
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                    }
                    Err(_) => {
                        eprintln!("halt.error: Cannot convert \"{}\" to Float{}", s_str, self.current_span_info(ip));
                        return OpResult::Halt;
                    }
                }
            }
            _ => return OpResult::Halt,
        }
        OpResult::Continue
    }
}
