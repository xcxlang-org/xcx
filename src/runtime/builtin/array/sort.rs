use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;

impl Executor {
    pub fn handle_array_sort(
        &mut self,
        dst: u8,
        arr_rc: Arc<RwLock<crate::vm::object::ArrayObj>>,
        kind: MethodKind,
        locals: &mut [Value],
    ) -> OpResult {
        match kind {
            MethodKind::Sort => {
                arr_rc.write().elements.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                OpResult::Continue
            }
            MethodKind::Reverse => {
                arr_rc.write().elements.reverse();
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                OpResult::Continue
            }
            _ => OpResult::Halt,
        }
    }
}
