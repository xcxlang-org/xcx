use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::OpResult;

pub fn handle(op: OpCode, locals: &mut [Value]) -> Option<OpResult> {
    match op {
        OpCode::And { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(!a.is_bool_false() && !b.is_bool_false());
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Or { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(!a.is_bool_false() || !b.is_bool_false());
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Not { dst, src } => {
            let val = locals[src as usize];
            let res = Value::from_bool(val.is_bool_false());
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        _ => return None,
    }
    Some(OpResult::Continue)
}
