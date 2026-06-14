use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::OpResult;
use crate::vm::object::StringObj;

pub fn handle(op: OpCode, locals: &mut [Value]) -> Option<OpResult> {
    match op {
        OpCode::CastInt { dst, src } => {
            let v = locals[src as usize];
            let res = v.cast_int();
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_i64(res);
        }
        OpCode::CastFloat { dst, src } => {
            let v = locals[src as usize];
            let res = v.cast_float();
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_f64(res);
        }
        OpCode::CastString { dst, src } => {
            let v = locals[src as usize];
            let s = v.as_string_lossy();
            let res = Value::from_string(Arc::new(StringObj::new(s.into_bytes())));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::CastBool { dst, src } => {
            let v = locals[src as usize];
            let res = !v.is_bool_false();
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(res);
        }
        OpCode::Typeof { dst, src } => {
            let v = locals[src as usize];
            let tname = v.type_name();
            let res = Value::from_string(Arc::new(StringObj::new(tname.as_bytes().to_vec())));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        _ => return None,
    }
    Some(OpResult::Continue)
}
