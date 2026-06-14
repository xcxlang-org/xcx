use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::OpResult;
use crate::vm::VM;
use std::sync::Arc;

pub fn handle(op: OpCode, locals: &mut [Value], vm_arc: &Arc<VM>) -> Option<OpResult> {
    match op {
        OpCode::Add { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = a.add(b);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Sub { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = a.sub(b);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Mul { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = a.mul(b);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Div { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            match a.div(b) {
                Ok(res) => {
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = res;
                }
                Err(_) => {
                    crate::runtime::builtin::io::eprint_buffered("ERROR halt: division by zero\n");
                    vm_arc.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    return Some(OpResult::Halt);
                }
            }
        }
        OpCode::Mod { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            match a.rem(b) {
                Ok(res) => {
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = res;
                }
                Err(_) => {
                    crate::runtime::builtin::io::eprint_buffered("ERROR halt: modulo by zero\n");
                    vm_arc.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    return Some(OpResult::Halt);
                }
            }
        }
        OpCode::Pow { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = a.pow(b);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Neg { dst, src } => {
            let val = locals[src as usize];
            let res = val.neg();
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::IncLocal { reg } => {
            let idx = reg as usize;
            let val = locals[idx];
            locals[idx] = Value::from_i64(val.as_i64().wrapping_add(1));
        }
        OpCode::DecLocal { reg } => {
            let idx = reg as usize;
            let val = locals[idx];
            locals[idx] = Value::from_i64(val.as_i64().wrapping_sub(1));
        }
        _ => return None,
    }
    Some(OpResult::Continue)
}
