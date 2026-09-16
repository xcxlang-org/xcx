use std::sync::Arc;

use crate::vm::core::vm::{VM, OpResult};
use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::executor::Executor;

pub mod arith;
pub mod compare;
pub mod logic;
pub mod collection;
pub mod member;
pub mod cast;
pub mod module;
pub mod control;

impl Executor {
    pub(crate) fn execute_step(
        &mut self,
        op: OpCode,
        locals: &mut [Value],
        vm_arc: &Arc<VM>,
        ip: &mut usize,
    ) -> Option<OpResult> {
        match op {
            // Variable access
            OpCode::LoadConst { dst, idx } => {
                let val = self.ctx.constants[idx as usize];
                unsafe { val.assign_to(&mut locals[dst as usize]); }
                Some(OpResult::Continue)
            }
            OpCode::Move { dst, src } => {
                let val = locals[src as usize];
                unsafe { val.assign_to(&mut locals[dst as usize]); }
                Some(OpResult::Continue)
            }
            OpCode::GetVar { .. } | OpCode::SetVar { .. } | OpCode::IncVar { .. } | OpCode::DecVar { .. } | OpCode::StrAppendVar { .. } | OpCode::StrAppendLocal { .. } => {
                self.handle_var_op(op, locals)
            }

            // Categories
            OpCode::Add {..} | OpCode::Sub {..} | OpCode::Mul {..} | OpCode::Div {..} | 
            OpCode::Mod {..} | OpCode::Pow {..} | OpCode::Neg {..} | OpCode::IncLocal {..} | OpCode::DecLocal {..} => {
                arith::handle(op, locals, vm_arc)
            }
            
            OpCode::Equal {..} | OpCode::NotEqual {..} | OpCode::Greater {..} | 
            OpCode::Less {..} | OpCode::GreaterEqual {..} | OpCode::LessEqual {..} => {
                compare::handle(op, locals)
            }

            OpCode::And {..} | OpCode::Or {..} | OpCode::Not {..} => {
                logic::handle(op, locals)
            }

            OpCode::ArrayInit {..} | OpCode::BoolArrayInit {..} | OpCode::SetInit {..} | OpCode::MapInit {..} | 
            OpCode::Has {..} | OpCode::GetIndex {..} | OpCode::SetIndex {..} | OpCode::StrAppendElement {..} |
            OpCode::SetUnion {..} | OpCode::SetIntersection {..} | 
            OpCode::SetDifference {..} | OpCode::SetSymDifference {..} |
            OpCode::RandomChoice {..} | OpCode::IntConcat {..} | 
            OpCode::SetRange {..} | OpCode::RandomInt {..} | OpCode::RandomFloat {..} |
            OpCode::RowGet {..} | OpCode::TablePushRow {..} | OpCode::TableCloneSkeleton {..} |
            OpCode::TableInitRow {..} => {
                collection::handle(self, op, locals)
            }

            OpCode::TableInit {..} | OpCode::TableBegin {..} => {
                collection::handle_table_init(self, op, locals)
            }

            OpCode::GetMember {..} | OpCode::SetMember {..} | OpCode::StrAppendMember {..} => {
                member::handle(self, op, locals, vm_arc)
            }

            OpCode::CastInt {..} | OpCode::CastFloat {..} | OpCode::CastString {..} | 
            OpCode::CastBool {..} | OpCode::Typeof {..} => {
                cast::handle(op, locals)
            }

            OpCode::Print {..} | OpCode::Input {..} | OpCode::TerminalWrite {..} |
            OpCode::JsonParse {..} | OpCode::DateNow {..} |
            OpCode::PerfMs {..} | OpCode::PerfUs {..} | OpCode::PerfNs {..} |
            OpCode::JsonBind {..} | OpCode::JsonBindLocal {..} | OpCode::JsonInject {..} | OpCode::JsonInjectLocal {..} | OpCode::JsonFastGetPush {..} |
            OpCode::HttpCall {..} | OpCode::HttpRequest {..} | OpCode::HttpRespond {..} | OpCode::HttpServe {..} |
            OpCode::StoreRead {..} | OpCode::StoreWrite {..} | OpCode::StoreAppend {..} |
            OpCode::StoreExists {..} | OpCode::StoreDelete {..} | OpCode::StoreList {..} |
            OpCode::StoreIsDir {..} | OpCode::StoreSize {..} | OpCode::StoreMkdir {..} |
            OpCode::StoreGlob {..} | OpCode::StoreZip {..} | OpCode::StoreUnzip {..} |
            OpCode::CryptoHash {..} | OpCode::CryptoVerify {..} | OpCode::CryptoToken {..} |
            OpCode::EnvGet {..} | OpCode::EnvArgs {..} |
            OpCode::TerminalClear {..} | OpCode::TerminalRaw {..} | OpCode::TerminalNormal {..} |
            OpCode::TerminalCursor {..} | OpCode::TerminalMove {..} |
            OpCode::TerminalRun {..} | OpCode::TerminalExit {..} |
            OpCode::InputKey {..} | OpCode::InputKeyWait {..} | OpCode::InputReady {..} |
            OpCode::DatabaseInit {..} | OpCode::HaltAlert {..} | OpCode::HaltError {..} | OpCode::HaltFatal {..} => {
                module::handle(self, op, locals, vm_arc, *ip)
            }

            OpCode::Jump {..} | OpCode::JumpIfFalse {..} | OpCode::JumpIfTrue {..} |
            OpCode::LoopNext {..} | OpCode::LoopPrev {..} |
            OpCode::IncLocalLoopNext {..} | OpCode::DecLocalLoopPrev {..} |
            OpCode::IncVarLoopNext {..} | OpCode::DecVarLoopPrev {..} |
            OpCode::ArrayLoopNext {..} | OpCode::TableIter {..} |
            OpCode::Call {..} | OpCode::Return {..} | OpCode::ReturnVoid | 
            OpCode::FiberCreate {..} | OpCode::Yield {..} | OpCode::YieldWithTarget {..} | 
            OpCode::YieldVoid | OpCode::Wait {..} | OpCode::Halt => {
                control::handle(self, op, locals, vm_arc, ip)
            }

            OpCode::MethodCall { dst, kind, base, arg_count } => {
                let receiver = locals[base as usize];
                let (args_buf, n) = extract_method_args(locals, base, arg_count);
                let args = &args_buf[..n];
                Some(self.handle_method_call(dst, receiver, kind, args, None, *ip, locals, vm_arc))
            }
            OpCode::MethodCallCustom { dst, method_name_idx, base, arg_count } => {
                let receiver = locals[base as usize];
                let (args_buf, n) = extract_method_args(locals, base, arg_count);
                let args = &args_buf[..n];
                let name = self.const_string(method_name_idx);
                Some(self.handle_method_call_custom(dst, receiver, name.as_str(), args, *ip, locals, vm_arc))
            }
            OpCode::MethodCallNamed { dst, kind, base, arg_count, names_idx } => {
                let receiver = locals[base as usize];
                let (args_buf, n) = extract_method_args(locals, base, arg_count);
                let args = &args_buf[..n];
                let names_arc = self.const_name_list(names_idx);
                let names = names_arc.as_ref().map(|v| v.as_slice());
                Some(self.handle_method_call(dst, receiver, kind, args, names, *ip, locals, vm_arc))
            }

            OpCode::SetName { src, name_idx } => {
                let name = self.const_string(name_idx);
                let val = locals[src as usize];

                let idx = {
                    let mut names = vm_arc.global_names.write();
                    if let Some(&i) = names.get(name.as_str()) {
                        i
                    } else {
                        let next_idx = names.len();
                        names.insert((*name).clone(), next_idx);
                        next_idx
                    }
                };

                vm_arc.set_global(idx, val);
                Some(OpResult::Continue)
            }
        }
    }

    fn handle_var_op(
        &mut self,
        op: OpCode,
        locals: &mut [Value],
    ) -> Option<OpResult> {
        match op {
            OpCode::GetVar { dst, idx } => {
                let val = unsafe { self.get_global_raw(idx as usize) };
                unsafe { val.assign_to(&mut locals[dst as usize]); }
            }
            OpCode::SetVar { idx, src } => {
                let val = locals[src as usize];
                unsafe { self.set_global_raw(idx as usize, val); }
            }
            OpCode::IncVar { idx } => {
                let idx = idx as usize;
                let val = unsafe { self.get_global_raw(idx) };
                let res = val.add(Value::from_i64(1));
                unsafe { self.set_global_raw(idx, res); }
            }
            OpCode::DecVar { idx } => {
                let idx = idx as usize;
                let val = unsafe { self.get_global_raw(idx) };
                let res = val.sub(Value::from_i64(1));
                unsafe { self.set_global_raw(idx, res); }
            }
            OpCode::StrAppendVar { var_idx, src } => {
                let rhs_val = locals[src as usize];
                let raw = unsafe { self.get_global_raw(var_idx as usize) };

                // Fast sub-path: target string under the usual in-place
                // threshold and a distinct string rhs — append the rhs bytes
                // straight from their allocation instead of copying them into
                // an intermediate Vec first. Falls through to the general
                // path for self-append, shared strings and non-string rhs.
                if raw.tag == crate::vm::value::nan_boxing::TAG_STR
                    && rhs_val.tag == crate::vm::value::nan_boxing::TAG_STR
                    && rhs_val.bits != raw.bits
                {
                    let ptr = raw.bits as *const crate::vm::object::StringObj;
                    let arc = unsafe { Arc::from_raw(ptr) };
                    // In-place append requires the globals slot to be the sole
                    // owner of the buffer. A string literal loaded from the
                    // chunk's constant pool (or an alias assigned to another
                    // variable) is a real, observable co-owner: mutating a
                    // buffer with sc == 2 corrupts the constant pool itself,
                    // so every later load of the same literal inherits the
                    // appended content.
                    if Arc::strong_count(&arc) <= 1 && Arc::weak_count(&arc) == 0 {
                        unsafe {
                            let obj_ptr = ptr as *mut crate::vm::object::StringObj;
                            (*obj_ptr).hash = None;
                            (*obj_ptr).data.extend_from_slice(
                                &(*(rhs_val.bits as *const crate::vm::object::StringObj)).data,
                            );
                        }
                        let bits = Arc::into_raw(arc) as u64;
                        let new_val = Value { bits, tag: crate::vm::value::nan_boxing::TAG_STR };
                        unsafe { self.set_global_raw(var_idx as usize, new_val); }
                        return Some(OpResult::Continue);
                    }
                    std::mem::forget(arc);
                }

                let rhs_bytes = if rhs_val.tag == crate::vm::value::nan_boxing::TAG_STR {
                    let arc = crate::vm::value::heap_object::as_string(&rhs_val);
                    arc.data.clone()
                } else {
                    rhs_val.to_string().into_bytes()
                };

                if raw.tag == crate::vm::value::nan_boxing::TAG_STR {
                    let ptr = raw.bits as *const crate::vm::object::StringObj;
                    let arc = unsafe { Arc::from_raw(ptr) };

                    // Same sole-owner rule as the fast sub-path above: the
                    // globals slot must be the only reference. A second
                    // reference is a real co-owner (constant pool, aliased
                    // variable), never a dead register copy, so shared
                    // buffers always take the re-allocation path.
                    let sc = Arc::strong_count(&arc);
                    let wc = Arc::weak_count(&arc);
                    if sc <= 1 && wc == 0 {
                        let obj_ptr = ptr as *mut crate::vm::object::StringObj;
                        unsafe {
                            (*obj_ptr).hash = None;
                            (*obj_ptr).data.extend_from_slice(&rhs_bytes);
                        }
                        let bits = Arc::into_raw(arc) as u64;
                        let new_val = Value { bits, tag: crate::vm::value::nan_boxing::TAG_STR };
                        unsafe { self.set_global_raw(var_idx as usize, new_val); }
                    } else {
                        let mut combined = Vec::with_capacity(arc.data.len() + rhs_bytes.len());
                        combined.extend_from_slice(&arc.data);
                        combined.extend_from_slice(&rhs_bytes);
                        std::mem::forget(arc);
                        let new_arc = Arc::new(crate::vm::object::StringObj::new(combined));
                        let new_val = crate::vm::value::heap_object::from_string(new_arc);
                        unsafe { self.set_global_raw(var_idx as usize, new_val); }
                        // set_global_raw takes its own reference for the slot;
                        // release the construction reference so the stored
                        // count reflects the slot alone. The sole-owner check
                        // above reads exactly this count on the next append —
                        // a surviving construction reference would masquerade
                        // as a co-owner and force a re-allocation (and leak
                        // every intermediate buffer) on each iteration.
                        unsafe { new_val.dec_ref(); }
                    }
                } else {
                    let s1 = raw.to_string();
                    let suffix = String::from_utf8_lossy(&rhs_bytes).into_owned();
                    let combined = s1 + &suffix;
                    let new_val = Value::from_string(Arc::new(crate::vm::object::StringObj::new(combined.into_bytes())));
                    unsafe { self.set_global_raw(var_idx as usize, new_val); }
                    unsafe { new_val.dec_ref(); }
                }
            }
            OpCode::StrAppendLocal { local_idx, src } => {
                let rhs_val = locals[src as usize];
                let raw = locals[local_idx as usize];

                // Fast sub-path, same rationale as StrAppendVar above: skip
                // the rhs Vec copy when the target is uniquely owned and the
                // rhs is a different string object.
                if raw.tag == crate::vm::value::nan_boxing::TAG_STR
                    && rhs_val.tag == crate::vm::value::nan_boxing::TAG_STR
                    && rhs_val.bits != raw.bits
                {
                    let ptr = raw.bits as *const crate::vm::object::StringObj;
                    let arc = unsafe { Arc::from_raw(ptr) };
                    if Arc::strong_count(&arc) <= 1 && Arc::weak_count(&arc) == 0 {
                        unsafe {
                            let obj_ptr = ptr as *mut crate::vm::object::StringObj;
                            (*obj_ptr).hash = None;
                            (*obj_ptr).data.extend_from_slice(
                                &(*(rhs_val.bits as *const crate::vm::object::StringObj)).data,
                            );
                        }
                        let bits = Arc::into_raw(arc) as u64;
                        locals[local_idx as usize] = Value { bits, tag: crate::vm::value::nan_boxing::TAG_STR };
                        return Some(OpResult::Continue);
                    }
                    std::mem::forget(arc);
                }

                let rhs_bytes = if rhs_val.tag == crate::vm::value::nan_boxing::TAG_STR {
                    let arc = crate::vm::value::heap_object::as_string(&rhs_val);
                    arc.data.clone()
                } else {
                    rhs_val.to_string().into_bytes()
                };

                let raw = locals[local_idx as usize];
                if raw.tag == crate::vm::value::nan_boxing::TAG_STR {
                    let ptr = raw.bits as *const crate::vm::object::StringObj;
                    let arc = unsafe { Arc::from_raw(ptr) };

                    // Exclusive ownership through the register: mutating in
                    // place cannot be observed through any other reference.
                    let sc = Arc::strong_count(&arc);
                    let wc = Arc::weak_count(&arc);
                    if sc <= 1 && wc == 0 {
                        let obj_ptr = ptr as *mut crate::vm::object::StringObj;
                        unsafe {
                            (*obj_ptr).hash = None;
                            (*obj_ptr).data.extend_from_slice(&rhs_bytes);
                        }
                        let bits = Arc::into_raw(arc) as u64;
                        let new_val = Value { bits, tag: crate::vm::value::nan_boxing::TAG_STR };
                        locals[local_idx as usize] = new_val;
                    } else {
                        let mut combined = Vec::with_capacity(arc.data.len() + rhs_bytes.len());
                        combined.extend_from_slice(&arc.data);
                        combined.extend_from_slice(&rhs_bytes);
                        std::mem::forget(arc);
                        let new_arc = Arc::new(crate::vm::object::StringObj::new(combined));
                        let new_val = crate::vm::value::heap_object::from_string(new_arc);
                        unsafe { new_val.replace_at(&mut locals[local_idx as usize]); }
                    }
                } else {
                    let s1 = raw.to_string();
                    let suffix = String::from_utf8_lossy(&rhs_bytes).into_owned();
                    let combined = s1 + &suffix;
                    let new_val = Value::from_string(Arc::new(crate::vm::object::StringObj::new(combined.into_bytes())));
                    unsafe { new_val.replace_at(&mut locals[local_idx as usize]); }
                }
            }
            _ => return None,
        }
        Some(OpResult::Continue)
    }}

#[inline(always)]
fn extract_method_args(locals: &[Value], base: u8, arg_count: u8) -> ([Value; 16], usize) {
    let mut args_buf = [Value::from_bool(false); 16];
    let args_start = base as usize + 1;
    let n = arg_count as usize;
    args_buf[..n].copy_from_slice(&locals[args_start..args_start + n]);
    (args_buf, n)
}

