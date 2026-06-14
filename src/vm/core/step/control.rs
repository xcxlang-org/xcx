use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::object::{FiberObj, FiberStatus};

pub fn handle(
    exec: &mut Executor,
    op: OpCode,
    locals: &mut [Value],
    vm_arc: &Arc<VM>,
    ip: &mut usize,
) -> Option<OpResult> {
    match op {
        OpCode::Jump { target } => {
            // Backward jump — potential hot path

            *ip = target as usize;
            Some(OpResult::Continue)
        }
        OpCode::JumpIfFalse { src, target } => {
            if locals[src as usize].is_bool_false() {
                *ip = target as usize;
                return Some(OpResult::Continue);
            }
            Some(OpResult::Continue)
        }
        OpCode::JumpIfTrue { src, target } => {
            if !locals[src as usize].is_bool_false() {
                *ip = target as usize;
                return Some(OpResult::Continue);
            }
            Some(OpResult::Continue)
        }
        OpCode::LoopNext { reg, limit_reg, target } => {
            let val = locals[reg as usize];
            let limit = locals[limit_reg as usize];
            if val.is_int() && limit.is_int() {
                let v = val.as_i64().wrapping_add(1);
                locals[reg as usize] = Value::from_i64(v);
                if v <= limit.as_i64() {
                    *ip = target as usize;
                    return Some(OpResult::Continue);
                }
            }
            Some(OpResult::Continue)
        }
        OpCode::LoopPrev { reg, limit_reg, target } => {
            let val = locals[reg as usize];
            let limit = locals[limit_reg as usize];
            if val.is_int() && limit.is_int() {
                let v = val.as_i64().wrapping_sub(1);
                locals[reg as usize] = Value::from_i64(v);
                if v >= limit.as_i64() {
                    *ip = target as usize;
                    return Some(OpResult::Continue);
                }
            }
            Some(OpResult::Continue)
        }
        OpCode::IncLocalLoopNext { inc_reg, reg, limit_reg, target } => {
            let r_idx = reg as usize;
            let l_idx = limit_reg as usize;
            let limit_val = locals[l_idx];
            let val = locals[r_idx];

            if val.is_int() && limit_val.is_int() {
                let next = val.as_i64().wrapping_add(1);
                locals[r_idx] = Value::from_i64(next);

                let i_idx = inc_reg as usize;
                let old_i_val = locals[i_idx];
                locals[i_idx] = Value::from_i64(old_i_val.as_i64().wrapping_add(1));
                if old_i_val.is_ptr() { unsafe { old_i_val.dec_ref(); } }

                if next <= limit_val.as_i64() {
                    *ip = target as usize;
                    return Some(OpResult::Continue);
                }
            }
            Some(OpResult::Continue)
        }
        OpCode::DecLocalLoopPrev { dec_reg, reg, limit_reg, target } => {
            let r_idx = reg as usize;
            let l_idx = limit_reg as usize;
            let limit_val = locals[l_idx];
            let val = locals[r_idx];

            if val.is_int() && limit_val.is_int() {
                let next = val.as_i64().wrapping_sub(1);
                locals[r_idx] = Value::from_i64(next);

                let i_idx = dec_reg as usize;
                let old_i_val = locals[i_idx];
                locals[i_idx] = Value::from_i64(old_i_val.as_i64().wrapping_sub(1));
                if old_i_val.is_ptr() { unsafe { old_i_val.dec_ref(); } }

                if next >= limit_val.as_i64() {
                    *ip = target as usize;
                    return Some(OpResult::Continue);
                }
            }
            Some(OpResult::Continue)
        }
        OpCode::IncVarLoopNext { g_idx, reg, limit_reg, target } => {
            let idx = g_idx as usize;
            let val = vm_arc.get_global(idx);
            let res = val.add(Value::from_i64(1));
            vm_arc.set_global(idx, res);

            let r_idx = reg as usize;
            let l_idx = limit_reg as usize;
            let limit_val = locals[l_idx];
            let val = locals[r_idx];

            if val.is_int() && limit_val.is_int() {
                let next = val.as_i64().wrapping_add(1);
                locals[r_idx] = Value::from_i64(next);

                if next <= limit_val.as_i64() {
                    *ip = target as usize;
                    return Some(OpResult::Continue);
                }
            }
            Some(OpResult::Continue)
        }
        OpCode::DecVarLoopPrev { g_idx, reg, limit_reg, target } => {
            let idx = g_idx as usize;
            let val = vm_arc.get_global(idx);
            let res = val.sub(Value::from_i64(1));
            vm_arc.set_global(idx, res);

            let r_idx = reg as usize;
            let l_idx = limit_reg as usize;
            let limit_val = locals[l_idx];
            let val = locals[r_idx];

            if val.is_int() && limit_val.is_int() {
                let next = val.as_i64().wrapping_sub(1);
                locals[r_idx] = Value::from_i64(next);

                if next >= limit_val.as_i64() {
                    *ip = target as usize;
                    return Some(OpResult::Continue);
                }
            }
            Some(OpResult::Continue)
        }
        OpCode::Call { dst, func_idx, base, arg_count } => {
            let func_chunk = exec.ctx.functions[func_idx as usize].clone();
            let args = &locals[base as usize..(base as usize + arg_count as usize)];
            let ores = exec.handle_call(dst, func_idx, func_chunk, args, vm_arc);
            match ores {
                OpResult::Return(v) => {
                    if let Some(val) = v {
                        unsafe { val.replace_at(&mut locals[dst as usize]); }
                    }
                    Some(OpResult::Continue)
                }
                OpResult::Halt => {
                    // halt.error aborts the callee frame. If the caller is also a function,
                    // propagate the abort up. If the caller is the top-level main block, continue.
                    if exec.call_depth == 0 {
                        Some(OpResult::Continue)
                    } else {
                        Some(OpResult::Halt)
                    }
                }
                _ => Some(ores),
            }
        }
        OpCode::Return { src } => {
            let val = locals[src as usize];
            unsafe { val.inc_ref(); }
            Some(OpResult::Return(Some(val)))
        }
        OpCode::ReturnVoid => Some(OpResult::Return(None)),
        OpCode::ArrayLoopNext { idx_reg, size_reg, target } => {
            let i_idx = idx_reg as usize;
            let s_idx = size_reg as usize;
            let idx_val = locals[i_idx].as_i64();
            let size_val = locals[s_idx].as_i64();

            let next_idx = idx_val + 1;
            locals[i_idx] = Value::from_i64(next_idx);

            if next_idx < size_val {
                *ip = target as usize;
                return Some(OpResult::Continue);
            }
            Some(OpResult::Continue)
        }
        OpCode::Halt => Some(OpResult::Halt),

        OpCode::FiberCreate { dst, func_idx, base, arg_count } => {
            let chunk = exec.ctx.functions[func_idx as usize].clone();
            let mut f_locals = vec![Value::from_bool(false); chunk.max_locals];
            for i in 0..arg_count {
                let v = locals[(base + i) as usize];
                unsafe { v.inc_ref(); }
                if (i as usize) < f_locals.len() {
                    f_locals[i as usize] = v;
                }
            }
            let fiber = FiberObj {
                func_id: func_idx as usize,
                ip: 0,
                locals: f_locals,
                status: FiberStatus::Suspended,
                is_done: false,
                yielded_value: None,
                trace_revision: 0,
            };
            let res = Value::from_fiber(Arc::new(RwLock::new(fiber)));
            unsafe { res.replace_at(&mut locals[dst as usize]); }
            Some(OpResult::Continue)
        }
        OpCode::Yield { src } => {
            if exec.in_fiber {
                let val = locals[src as usize];
                unsafe { val.inc_ref(); }
                Some(OpResult::Yield(Some(val)))
            } else {
                Some(OpResult::Continue)
            }
        }
        OpCode::YieldWithTarget { dst, src } => {
            let val = locals[src as usize];
            unsafe { val.assign_to(&mut locals[dst as usize]); }
            if exec.in_fiber {
                unsafe { val.inc_ref(); }
                Some(OpResult::YieldWithTarget(dst, Some(val)))
            } else {
                Some(OpResult::Continue)
            }
        }
        OpCode::YieldVoid => {
            if exec.in_fiber {
                Some(OpResult::Yield(None))
            } else {
                Some(OpResult::Continue)
            }
        }
        OpCode::Wait { src } => {
            let ms = locals[src as usize].as_i64();
            crate::runtime::builtin::io::flush_buffered();
            if ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
            }
            Some(OpResult::Continue)
        }
        OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, target } => {
            let i_idx = idx_reg as usize;
            let l_idx = limit_reg as usize;
            let idx_val = locals[i_idx].as_i64();
            let limit_val = locals[l_idx].as_i64();

            let next_idx = idx_val + 1;
            locals[i_idx] = Value::from_i64(next_idx);

            if next_idx < limit_val {
                let tbl_val = locals[tbl_reg as usize];
                if tbl_val.is_table() {
                    let t_rc = tbl_val.as_table();
                    let row_obj = Arc::new(crate::vm::object::RowObj {
                        table: t_rc.clone(),
                        row_idx: next_idx as u32,
                    });
                    let new_row = Value::from_row(row_obj);
                    unsafe { locals[row_reg as usize].dec_ref(); }
                    locals[row_reg as usize] = new_row;

                    *ip = target as usize;
                    return Some(OpResult::Continue);
                }
            }
            Some(OpResult::Continue)
        }
        _ => None,
    }
}
