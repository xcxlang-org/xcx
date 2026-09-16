use crate::vm::value::{Value, tag::TAG_INT};
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::OpResult;

pub fn handle(op: OpCode, locals: &mut [Value]) -> Option<OpResult> {
    match op {
        OpCode::Equal { dst, src1, src2 } => {
            let v1 = locals[src1 as usize];
            let v2 = locals[src2 as usize];
            let res = Value::from_bool(if v1.tag == TAG_INT && v2.tag == TAG_INT {
                v1.bits == v2.bits
            } else {
                v1 == v2
            });
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::NotEqual { dst, src1, src2 } => {
            let v1 = locals[src1 as usize];
            let v2 = locals[src2 as usize];
            let res = Value::from_bool(if v1.tag == TAG_INT && v2.tag == TAG_INT {
                v1.bits != v2.bits
            } else {
                v1 != v2
            });
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Greater { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(match int_pair(a, b) {
                Some((x, y)) => x > y,
                None => a > b,
            });
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Less { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(match int_pair(a, b) {
                Some((x, y)) => x < y,
                None => a < b,
            });
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::GreaterEqual { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(match int_pair(a, b) {
                Some((x, y)) => x >= y,
                None => a >= b,
            });
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::LessEqual { dst, src1, src2 } => {
            let a = locals[src1 as usize];
            let b = locals[src2 as usize];
            let res = Value::from_bool(match int_pair(a, b) {
                Some((x, y)) => x <= y,
                None => a <= b,
            });
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        _ => return None,
    }
    Some(OpResult::Continue)
}

/// Ordered/quality comparisons on two ints need no type ranking. The generic
/// `Value` route goes through `Ord::cmp`, which executes `variant_rank()` for
/// both operands before reaching the i64 compare — measurable in loop
/// conditions that run billions of times.
#[inline(always)]
fn int_pair(a: Value, b: Value) -> Option<(i64, i64)> {
    if a.tag == TAG_INT && b.tag == TAG_INT {
        Some((a.bits as i64, b.bits as i64))
    } else {
        None
    }
}
