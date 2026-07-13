use std::sync::Arc;

use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;

pub fn handle(
    exec: &mut Executor,
    op: OpCode,
    locals: &mut [Value],
    _vm_arc: &Arc<VM>,
) -> Option<OpResult> {
    match op {
        OpCode::GetMember { dst, container, name_idx } => {
            let c = locals[container as usize];
            let name = exec.ctx.constants[name_idx as usize].to_string();
            let res = crate::vm::core::runtime_ops::RuntimeOps::get_member(c, &name);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::SetMember { container, name_idx, src } => {
            let c = locals[container as usize];
            let name = exec.ctx.constants[name_idx as usize].to_string();
            let val = locals[src as usize];
            crate::vm::core::runtime_ops::RuntimeOps::set_member(c, &name, val);
        }
        OpCode::StrAppendMember { container, name_idx, src } => {
            let c = locals[container as usize];
            let name = exec.ctx.constants[name_idx as usize].to_string();
            let val = locals[src as usize];
            crate::vm::core::runtime_ops::RuntimeOps::str_append_member(c, &name, val);
        }
        _ => return None,
    }
    Some(OpResult::Continue)
}
