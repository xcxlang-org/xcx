use crate::vm::core::executor::Executor;
use crate::vm::opcode::{OpCode, MethodKind};
use crate::vm::trace::TraceOp;
use crate::vm::value::Value;

impl Executor {
    pub fn record_op(&mut self, op: OpCode, locals: &[Value], ip: usize) {
        if !self.recorder.is_recording { return; }
        match op {
            OpCode::LoadConst { dst, idx } => {
                let val = self.ctx.constants[idx as usize];
                self.recorder.record(TraceOp::LoadConst { dst, val });
            }
            OpCode::Move { dst, src } => {
                self.recorder.record(TraceOp::Move { dst, src });
            }
            OpCode::Add { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::AddInt { dst, src1, src2 });
                } else if v1.is_float() && v2.is_float() {
                    self.recorder.record(TraceOp::GuardFloat { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardFloat { reg: src2, ip });
                    self.recorder.record(TraceOp::AddFloat { dst, src1, src2 });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Mul { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::MulInt { dst, src1, src2 });
                } else if v1.is_float() && v2.is_float() {
                    self.recorder.record(TraceOp::GuardFloat { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardFloat { reg: src2, ip });
                    self.recorder.record(TraceOp::MulFloat { dst, src1, src2 });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Div { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::DivInt { dst, src1, src2, fail_ip: ip });
                } else if v1.is_float() && v2.is_float() {
                    self.recorder.record(TraceOp::GuardFloat { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardFloat { reg: src2, ip });
                    self.recorder.record(TraceOp::DivFloat { dst, src1, src2, fail_ip: ip });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Sub { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::SubInt { dst, src1, src2 });
                } else if v1.is_float() && v2.is_float() {
                    self.recorder.record(TraceOp::GuardFloat { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardFloat { reg: src2, ip });
                    self.recorder.record(TraceOp::SubFloat { dst, src1, src2 });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Mod { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::ModInt { dst, src1, src2, fail_ip: ip });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Neg { dst, src } => {
                let v = locals[src as usize];
                if v.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src, ip });
                    self.recorder.record(TraceOp::NegInt { dst, src });
                } else if v.is_float() {
                    self.recorder.record(TraceOp::GuardFloat { reg: src, ip });
                    self.recorder.record(TraceOp::NegFloat { dst, src });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Equal { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::CmpInt { dst, src1, src2, cc: 0 }); // IntCC::Equal
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Greater { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::CmpInt { dst, src1, src2, cc: 2 }); // IntCC::SignedGreaterThan
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Less { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::CmpInt { dst, src1, src2, cc: 3 }); // IntCC::SignedLessThan
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::GreaterEqual { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::CmpInt { dst, src1, src2, cc: 4 }); // IntCC::SignedGreaterThanOrEqual
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::LessEqual { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::CmpInt { dst, src1, src2, cc: 5 }); // IntCC::SignedLessThanOrEqual
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::CastFloat { dst, src } => {
                let v = locals[src as usize];
                if v.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src, ip });
                    self.recorder.record(TraceOp::CastIntToFloat { dst, src });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::CastInt { dst, src } => {
                let v = locals[src as usize];
                if v.is_float() {
                    self.recorder.record(TraceOp::GuardFloat { reg: src, ip });
                    self.recorder.record(TraceOp::CastFloatToInt { dst, src });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::CastBool { dst, src } => {
                self.recorder.record(TraceOp::CastBool { dst, src });
            }
            OpCode::JsonParse { dst, src } => {
                self.recorder.record(TraceOp::JsonParse { dst, src });
            }
            OpCode::DateNow { dst } => {
                self.recorder.record(TraceOp::DateNow { dst });
            }
            OpCode::GetIndex { dst, container, index } => {
                let recv = locals[container as usize];
                if recv.is_array() {
                    self.recorder.record(TraceOp::ArrayGetIndex { dst, arr_reg: container, idx_reg: index, fail_ip: ip });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::SetIndex { container, index, src } => {
                let recv = locals[container as usize];
                if recv.is_array() {
                    self.recorder.record(TraceOp::ArraySetIndex { arr_reg: container, idx_reg: index, val_reg: src, fail_ip: ip });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::Call { dst, func_idx, base, arg_count } => {
                self.recorder.record(TraceOp::Call { dst, func_idx, base, arg_count });
            }
            OpCode::GetVar { dst, idx } => {
                self.recorder.record(TraceOp::GetVar { dst, idx });
            }
            OpCode::SetVar { idx, src } => {
                self.recorder.record(TraceOp::SetVar { idx, src });
            }
            OpCode::Jump { target } => {
                let target_ip = target as usize;
                if target_ip < ip {
                    // Backward jump: a bare unconditional backward jump means the loop uses
                    // Add+Jump rather than a dedicated LoopNext opcode (i.e. a @step loop).
                    // The JIT would emit an infinite native loop with no conditional exit for
                    // the loop bound — blacklist to keep it interpreted.
                    let start = self.recorder.start_ip;
                    self.recorder.stop();
                    if let Some(s) = start {
                        self.hotspot.blacklist(s);
                    }
                } else {
                    self.recorder.record(TraceOp::Jump { target_ip });
                }
            }


            OpCode::JumpIfFalse { src, target } => {
                let val = locals[src as usize];
                let target_ip = target as usize;
                if val.is_bool_false() {
                    if target_ip < ip {
                         if let Some(start) = self.recorder.start_ip {
                             if target_ip == start {
                                 self.recorder.record(TraceOp::GuardFalse { reg: src, fail_ip: ip });
                                 self.recorder.record(TraceOp::Jump { target_ip });
                             } else { self.recorder.stop(); }
                         } else { self.recorder.stop(); }
                    } else {
                        self.recorder.record(TraceOp::GuardFalse { reg: src, fail_ip: ip });
                        self.recorder.record(TraceOp::Jump { target_ip });
                    }
                } else {
                    self.recorder.record(TraceOp::GuardTrue { reg: src, fail_ip: target_ip });
                }
            }
            OpCode::JumpIfTrue { src, target } => {
                let val = locals[src as usize];
                let target_ip = target as usize;
                if !val.is_bool_false() {
                    if target_ip < ip {
                        if let Some(start) = self.recorder.start_ip {
                            if target_ip == start {
                                self.recorder.record(TraceOp::GuardTrue { reg: src, fail_ip: ip });
                                self.recorder.record(TraceOp::Jump { target_ip });
                            } else { self.recorder.stop(); }
                        } else { self.recorder.stop(); }
                    } else {
                        self.recorder.record(TraceOp::GuardTrue { reg: src, fail_ip: ip });
                        self.recorder.record(TraceOp::Jump { target_ip });
                    }
                } else {
                    self.recorder.record(TraceOp::GuardFalse { reg: src, fail_ip: target_ip });
                }
            }
            OpCode::LoopNext { reg, limit_reg, target } => {
                let target_ip = target as usize;
                if let Some(start) = self.recorder.start_ip {
                    if target_ip == start {
                        self.recorder.record(TraceOp::LoopNextInt { 
                            reg, limit_reg, target, exit_ip: ip + 1
                        });
                    } else {
                        self.recorder.stop();
                    }
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::IncLocal { reg } => {
                let v = locals[reg as usize];
                if v.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg, ip });
                    self.recorder.record(TraceOp::IncLocal { reg });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::IncLocalLoopNext { inc_reg, reg, limit_reg, target } => {
                let target_ip = target as usize;
                if let Some(start) = self.recorder.start_ip {
                    if target_ip == start {
                        self.recorder.record(TraceOp::IncLocalLoopNext { 
                            inc_reg, reg, limit_reg, target, exit_ip: ip + 1
                        });
                    } else {
                        self.recorder.stop();
                    }
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::ArrayLoopNext { idx_reg, size_reg, target } => {
                let target_ip = target as usize;
                if let Some(start) = self.recorder.start_ip {
                    if target_ip == start {
                        self.recorder.record(TraceOp::ArrayLoopNext { 
                            idx_reg, size_reg, target, exit_ip: ip + 1
                        });
                    } else {
                        self.recorder.stop();
                    }
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::IncVar { idx } => {
                self.recorder.record(TraceOp::IncVar { g_idx: idx });
            }
            OpCode::IntConcat { dst, src1, src2 } => {
                let v1 = locals[src1 as usize];
                let v2 = locals[src2 as usize];
                if v1.is_int() && v2.is_int() {
                    self.recorder.record(TraceOp::GuardInt { reg: src1, ip });
                    self.recorder.record(TraceOp::GuardInt { reg: src2, ip });
                    self.recorder.record(TraceOp::IntConcat { dst, src1, src2 });
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::IncVarLoopNext { g_idx, reg, limit_reg, target } => {
                let target_ip = target as usize;
                if let Some(start) = self.recorder.start_ip {
                    if target_ip == start {
                        self.recorder.record(TraceOp::IncVarLoopNext { 
                            g_idx, reg, limit_reg, target, exit_ip: ip + 1
                        });
                    } else {
                        self.recorder.stop();
                    }
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::MethodCall { dst, kind, base, arg_count } => {
                let recv = locals[base as usize];
                if recv.is_array() {
                    match kind {
                        MethodKind::Len | MethodKind::Count | MethodKind::Size => {
                            self.recorder.record(TraceOp::ArraySize { dst, src: base });
                        }
                        MethodKind::Get if arg_count == 1 => {
                            let idx_reg = base + 1;
                            self.recorder.record(TraceOp::ArrayGet { dst, arr_reg: base, idx_reg, fail_ip: ip });
                        }
                        MethodKind::Push if arg_count == 1 => {
                            let val_reg = base + 1;
                            self.recorder.record(TraceOp::ArrayPush { arr_reg: base, val_reg });
                        }
                        MethodKind::Update | MethodKind::Set if arg_count == 2 => {
                            let idx_reg = base + 1;
                            let val_reg = base + 2;
                            self.recorder.record(TraceOp::ArrayUpdate { arr_reg: base, idx_reg, val_reg, fail_ip: ip });
                        }
                        _ => { self.recorder.stop(); }
                    }
                } else if recv.is_set() {
                    match kind {
                        MethodKind::Len | MethodKind::Count | MethodKind::Size => {
                            self.recorder.record(TraceOp::SetSize { dst, src: base });
                        }
                        MethodKind::Contains | MethodKind::Has if arg_count == 1 => {
                            let val_reg = base + 1;
                            self.recorder.record(TraceOp::SetContains { dst, set_reg: base, val_reg });
                        }
                        _ => { self.recorder.stop(); }
                    }
                } else if recv.is_fiber() {
                    match kind {
                        MethodKind::IsDone => {
                            self.recorder.record(TraceOp::FiberIsDone { dst, src: base });
                        }
                        MethodKind::Next => {
                            self.recorder.record(TraceOp::FiberNext { dst, src: base });
                        }
                        _ => { self.recorder.stop(); }
                    }
                } else if recv.is_table() && kind == MethodKind::Where && arg_count >= 1 {
                    // Record table where as a loop
                    let pred = locals[base as usize + 1];
                    if pred.is_func() {
                        let fid = pred.as_function_idx() as usize;
                        let res_tbl_reg = dst;
                        let tbl_reg = base;
                        let limit_reg = 254; // Use temp regs
                        let idx_reg = 253;
                        let row_reg = 252;
                        
                        self.recorder.record(TraceOp::TableCloneSkeleton { dst: res_tbl_reg, src: tbl_reg });
                        self.recorder.record(TraceOp::TableSize { dst: limit_reg, src: tbl_reg });
                        self.recorder.record(TraceOp::LoadConst { dst: idx_reg, val: Value::from_i64(-1) });
                        
                        let loop_start_idx = if let Some(ref lock) = self.recorder.recording_trace {
                            lock.read().ops.len()
                        } else { 0 };
                        self.recorder.record(TraceOp::TableIter { 
                            tbl_reg, idx_reg, row_reg, limit_reg, target: loop_start_idx as u32, exit_ip: ip + 1 
                        });
                        
                        // Record predicate inline
                        let _pred_chunk = self.ctx.functions[fid].clone();
                        // This is tricky: we need to map pred's locals to our trace's virtual locals
                        // For now, let's just record a Call TraceOp which the JIT can inline
                        self.recorder.record(TraceOp::Call { dst: 251, func_idx: fid as u32, base: row_reg, arg_count: 1 });
                        
                        self.recorder.record(TraceOp::GuardTrue { reg: 251, fail_ip: 0/*skip push*/ });
                        self.recorder.record(TraceOp::TablePushRow { tbl_reg: res_tbl_reg, row_reg });
                        
                        self.recorder.record(TraceOp::Jump { target_ip: loop_start_idx });
                    } else {
                        self.recorder.stop();
                    }
                } else if recv.is_string() {
                    match kind {
                        MethodKind::Length => {
                            self.recorder.record(TraceOp::StringLength { dst, src: base });
                        }
                        _ => { self.recorder.stop(); }
                    }
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::RowGet { dst, row_reg, col_idx } => {
                self.recorder.record(TraceOp::RowGet { dst, row_reg, col_idx });
            }
            OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, target } => {
                self.recorder.record(TraceOp::TableIter { 
                    tbl_reg, idx_reg, row_reg, limit_reg, target, exit_ip: ip + 1 
                });
            }
            OpCode::MethodCallCustom { dst, method_name_idx, base, arg_count } => {
                let recv = locals[base as usize];
                if recv.is_row() && arg_count == 0 {
                    let row = recv.as_row();
                    let name = self.ctx.constants[method_name_idx as usize].to_string();
                    let t = row.table.read();
                    if let Some(ci) = t.columns.iter().position(|c| c.name == name) {
                        self.recorder.record(TraceOp::RowGet { dst, row_reg: base, col_idx: ci as u16 });
                    } else {
                        self.recorder.stop();
                    }
                } else {
                    self.recorder.stop();
                }
            }
            OpCode::TablePushRow { tbl_reg, row_reg } => {
                self.recorder.record(TraceOp::TablePushRow { tbl_reg, row_reg });
            }
            OpCode::TableCloneSkeleton { dst, src } => {
                self.recorder.record(TraceOp::TableCloneSkeleton { dst, src });
            }
            OpCode::JsonBindLocal { dst, json_src, path_src } => {
                // Try to find if path_src was a constant in the current trace
                let mut path_val: Option<String> = None;
                if let Some(ref lock) = self.recorder.recording_trace {
                    let trace = lock.read();
                    for op in trace.ops.iter().rev() {
                        if let TraceOp::LoadConst { dst: d, val } = op {
                            if *d == path_src && val.is_string() {
                                path_val = Some(val.to_string());
                                break;
                            }
                        }
                    }
                }

                if let Some(path) = path_val {
                    self.recorder.record(TraceOp::JsonBindLocalConst { dst, json_reg: json_src, path });
                } else {
                    self.recorder.record(TraceOp::JsonBindLocal { dst, json_reg: json_src, path_reg: path_src });
                }
            }
            OpCode::JsonBind { idx, json_src, path_src } => {
                let mut path_val: Option<String> = None;
                if let Some(ref lock) = self.recorder.recording_trace {
                    let trace = lock.read();
                    for op in trace.ops.iter().rev() {
                        if let TraceOp::LoadConst { dst: d, val } = op {
                            if *d == path_src && val.is_string() {
                                path_val = Some(val.to_string());
                                break;
                            }
                        }
                    }
                }

                if let Some(path) = path_val {
                    self.recorder.record(TraceOp::JsonBindGlobalConst { idx, json_reg: json_src, path });
                } else {
                    self.recorder.record(TraceOp::JsonBindGlobal { idx, json_reg: json_src, path_reg: path_src });
                }
            }
            OpCode::GetMember { dst, container, name_idx } => {
                let name = self.ctx.constants[name_idx as usize].to_string();
                self.recorder.record(TraceOp::GetMember { dst, obj_reg: container, name });
            }
            OpCode::JsonFastGetPush { json_src, path_src, val_src } => {
                self.recorder.record(TraceOp::JsonFastGetPush { json_src, path_src, val_src });
            }
            _ => {
                // eprintln!("[JIT] Unsupported opcode for recording: {:?}", op);
                self.recorder.stop();
            }
        }
    }
}
