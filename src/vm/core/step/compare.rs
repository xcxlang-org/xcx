use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::OpResult;

pub fn handle(op: OpCode, locals: &mut [Value]) -> Option<OpResult> {
    match op {
        OpCode::Equal { dst, src1, src2 } => {
            let v1 = locals[src1 as usize];
            let v2 = locals[src2 as usize];
            let res = Value::from_bool(v1 == v2);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::NotEqual { dst, src1, src2 } => {
            let res = Value::from_bool(locals[src1 as usize] != locals[src2 as usize]);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Greater { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(a > b);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Less { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(a < b);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::GreaterEqual { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(a >= b);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::LessEqual { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(a <= b);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        _ => return None,
    }
    Some(OpResult::Continue)
}
