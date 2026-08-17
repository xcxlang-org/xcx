use std::sync::Arc;
use crate::vm::object::StringObj;
use parking_lot::RwLock;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;
use crate::vm::object::{FiberObj, FiberStatus};

impl Executor {
    pub fn handle_fiber_method(
        &mut self,
        dst: u8,
        fib_rc: Arc<RwLock<FiberObj>>,
        kind: MethodKind,
        _args: &[Value],
        _names: Option<&[String]>,
        ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Status => {
                let status = match fib_rc.read().status {
                    FiberStatus::Suspended => "suspended",
                    FiberStatus::Running => "running",
                    FiberStatus::Done => "done",
                    FiberStatus::Error => "error",
                };
                let res = Value::from_string(Arc::new(StringObj::new(status.to_string().into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::IsDone => {
                let done = fib_rc.read().is_done;
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(done);
            }
            MethodKind::Next | MethodKind::Run => {
                let (func_id, mut fib_ip, mut fib_locals) = {
                    let mut f = fib_rc.write();
                    if f.is_done {
                        eprintln!("R306: Calling .next()/.run() on finished fiber{}", self.current_span_info(ip));
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        return OpResult::Halt;
                    }
                    f.status = FiberStatus::Running;
                    (f.func_id, f.ip, std::mem::take(&mut f.locals))
                };

                let was_in_fiber = self.in_fiber;
                self.in_fiber = true;

                let chunk = self.ctx.functions[func_id].clone();

                // Fibers execute through the interpreter; the fiber-segment JIT was
                // removed as unreachable (its hotspot counter was never populated,
                // so no segment could ever be compiled or cached).
                let mut ores = None;

                if ores.is_none() {
                    let fiber_bptr = chunk.bytecode.as_ptr() as usize;
                    let old_bptr   = std::mem::replace(&mut self.current_bytecode_ptr, fiber_bptr);

                    ores = Some(self.execute_bytecode_inner(&chunk.bytecode, &mut fib_ip, &mut fib_locals, _vm_arc));

                    self.current_bytecode_ptr = old_bptr;
                }

                let ores = ores.unwrap();

                let mut f = fib_rc.write();
                f.ip = fib_ip;
                f.locals = fib_locals;

                match ores {
                    OpResult::Yield(val) | OpResult::YieldWithTarget(_, val) => {
                        f.status = FiberStatus::Suspended;
                        let res = val.unwrap_or(Value::from_bool(false));
                        if kind == MethodKind::Run {
                            // .run() returns true if suspended
                            unsafe { locals[dst as usize].dec_ref(); }
                            locals[dst as usize] = Value::from_bool(true);
                        } else {
                            // .next() returns the yielded value
                            unsafe { locals[dst as usize].dec_ref(); }
                            locals[dst as usize] = res;
                        }
                    }
                    OpResult::Return(_) | OpResult::Continue => {
                        f.status = FiberStatus::Done;
                        f.is_done = true;
                        if kind == MethodKind::Run {
                            // .run() returns false if done
                            unsafe { locals[dst as usize].dec_ref(); }
                            locals[dst as usize] = Value::from_bool(false);
                        } else {
                            // .next() returns the final return value or false
                            let res = if let OpResult::Return(v) = ores { v.unwrap_or(Value::from_bool(false)) } else { Value::from_bool(false) };
                            unsafe { locals[dst as usize].dec_ref(); }
                            locals[dst as usize] = res;
                        }
                    }
                    OpResult::Halt => {
                        self.in_fiber = was_in_fiber;
                        f.status = FiberStatus::Error;
                        return OpResult::Halt;
                    }
                }
                self.in_fiber = was_in_fiber;
            }
            MethodKind::Close => {
                let mut f = fib_rc.write();
                f.is_done = true;
                f.status = FiberStatus::Done;
                for v in std::mem::take(&mut f.locals) {
                    unsafe { v.dec_ref(); }
                }
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(true);
            }
            _ => { 
                eprintln!("Method {:?} not supported for Fiber{}", kind, self.current_span_info(ip)); 
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt; 
            }
        }
        OpResult::Continue
    }
}
