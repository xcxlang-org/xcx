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
            OpCode::GetVar { .. } | OpCode::SetVar { .. } | OpCode::IncVar { .. } | OpCode::DecVar { .. } => {
                self.handle_var_op(op, locals, vm_arc)
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

            OpCode::ArrayInit {..} | OpCode::SetInit {..} | OpCode::MapInit {..} | 
            OpCode::Has {..} | OpCode::GetIndex {..} | OpCode::SetIndex {..} |
            OpCode::SetUnion {..} | OpCode::SetIntersection {..} | 
            OpCode::SetDifference {..} | OpCode::SetSymDifference {..} |
            OpCode::RandomChoice {..} | OpCode::IntConcat {..} | 
            OpCode::SetRange {..} | OpCode::RandomInt {..} | OpCode::RandomFloat {..} |
            OpCode::RowGet {..} | OpCode::TablePushRow {..} | OpCode::TableCloneSkeleton {..} |
            OpCode::TableInitRow {..} => {
                collection::handle(op, locals)
            }

            OpCode::TableInit {..} | OpCode::TableBegin {..} => {
                collection::handle_table_init(self, op, locals)
            }

            OpCode::GetMember {..} | OpCode::SetMember {..} => {
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
                module::handle(self, op, locals, vm_arc)
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
                let args_start = base as usize + 1;
                let n = arg_count as usize;
                let mut args_buf = [Value::from_bool(false); 16];
                for i in 0..n { args_buf[i] = locals[args_start + i]; }
                let args = &args_buf[..n];
                Some(self.handle_method_call(dst, receiver, kind, args, None, *ip, locals, vm_arc))
            }
            OpCode::MethodCallCustom { dst, method_name_idx, base, arg_count } => {
                let receiver = locals[base as usize];
                let args_start = base as usize + 1;
                let n = arg_count as usize;
                let mut args_buf = [Value::from_bool(false); 16];
                for i in 0..n { args_buf[i] = locals[args_start + i]; }
                let args = &args_buf[..n];
                let name = self.ctx.constants[method_name_idx as usize].to_string();
                Some(self.handle_method_call_custom(dst, receiver, &name, args, *ip, locals, vm_arc, base))
            }
            OpCode::MethodCallNamed { dst, kind, base, arg_count, names_idx } => {
                let receiver = locals[base as usize];
                let args_start = base as usize + 1;
                let n = arg_count as usize;
                let mut args_buf = [Value::from_bool(false); 16];
                for i in 0..n { args_buf[i] = locals[args_start + i]; }
                let args = &args_buf[..n];
                let names_val = self.ctx.constants[names_idx as usize];
                let mut names_vec = Vec::new();
                if names_val.is_array() {
                    let arr = names_val.as_array();
                    let arr_rd = arr.read();
                    for v in arr_rd.elements.iter() {
                        names_vec.push(v.to_string());
                    }
                }
                let names = if names_vec.is_empty() { None } else { Some(names_vec.as_slice()) };
                Some(self.handle_method_call(dst, receiver, kind, args, names, *ip, locals, vm_arc))
            }

            OpCode::SetName { src, name_idx } => {
                let name_val = self.ctx.constants[name_idx as usize];
                let name = name_val.to_string();
                let val = locals[src as usize];

                let idx = {
                    let mut names = vm_arc.global_names.write();
                    if let Some(&i) = names.get(&name) {
                        i
                    } else {
                        let next_idx = names.len();
                        names.insert(name, next_idx);
                        next_idx
                    }
                };

                vm_arc.set_global(idx, val);
                Some(OpResult::Continue)
            }


            _ => None,
        }
    }

    fn handle_var_op(
        &mut self,
        op: OpCode,
        locals: &mut [Value],
        vm_arc: &Arc<VM>,
    ) -> Option<OpResult> {
        match op {
            OpCode::GetVar { dst, idx } => {
                let val = vm_arc.get_global(idx as usize);
                unsafe { val.assign_to(&mut locals[dst as usize]); }
            }
            OpCode::SetVar { idx, src } => {
                let val = locals[src as usize];
                vm_arc.set_global(idx as usize, val);
            }
            OpCode::IncVar { idx } => {
                let idx = idx as usize;
                let val = vm_arc.get_global(idx);
                let res = val.add(Value::from_i64(1));
                vm_arc.set_global(idx, res);
            }
            OpCode::DecVar { idx } => {
                let idx = idx as usize;
                let val = vm_arc.get_global(idx);
                let res = val.sub(Value::from_i64(1));
                vm_arc.set_global(idx, res);
            }
            _ => return None,
        }
        Some(OpResult::Continue)
    }
}

