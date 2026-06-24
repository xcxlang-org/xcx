use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_jit::JITModule;
use cranelift_module::{Linkage, Module};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

use crate::vm::value::{TAG_INT, TAG_BOOL};
use crate::vm::trace::{Trace, TraceOp};

use super::codegen_ctx::CodegenCtx;
use super::abi::trusted;
use super::symbols::SymbolRegistry;
use super::analysis::{analyze_trace_locals, analyze_trace_globals, analyze_trace_global_ints, analyze_trace_non_ptr_regs};

use super::emit_arith::*;
use super::emit_control::*;
use super::emit_load_store::*;
use super::emit_call::*;
use super::nan_ops::*;
use super::builder::create_jit_builder;

pub struct JIT {
    pub(crate) module: JITModule,
    pub(crate) ctx: codegen::Context,
    pub(crate) ptr_type: types::Type,
    pub(crate) symbols: SymbolRegistry,
    pub(crate) in_progress: std::collections::HashSet<usize>,
}

impl JIT {
    pub fn new() -> Self {
        let builder = create_jit_builder();
        let mut module = JITModule::new(builder);
        let ctx = module.make_context();
        let ptr_type = module.target_config().pointer_type();
        let symbols = SymbolRegistry::new(&mut module);
        
        Self {
            module,
            ctx,
            ptr_type,
            symbols,
            in_progress: std::collections::HashSet::new(),
        }
    }

    pub fn compile(&mut self, trace: Arc<RwLock<Trace>>) -> Result<*const u8, String> {
        let trace_read = trace.read();
        self.module.clear_context(&mut self.ctx);

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(self.ptr_type)); // out_ptr
        sig.params.push(AbiParam::new(self.ptr_type)); // locals_ptr
        sig.params.push(AbiParam::new(self.ptr_type)); // globals_ptr
        sig.params.push(AbiParam::new(self.ptr_type)); // consts_ptr
        sig.params.push(AbiParam::new(self.ptr_type)); // vm_ptr
        sig.params.push(AbiParam::new(self.ptr_type)); // exec_ptr
        sig.params.push(AbiParam::new(self.ptr_type)); // shutdown_ptr
        sig.returns.push(AbiParam::new(types::I32)); // status

        // If a function with this name was already declared, reuse its id.
        let func_name = format!("trace_{}", trace_read.start_ip);
        let func_id = self.module
            .declare_function(&func_name, Linkage::Export, &sig)
            .map_err(|e: cranelift_module::ModuleError| e.to_string())?;

        self.ctx.func.signature = sig;

        // Import symbols into this specific function's IR.
        let symbols = self.symbols.import_in_func(&mut self.module, &mut self.ctx.func);

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        {
            let entry_block = b.create_block();
            let loop_head = b.create_block();
            b.append_block_params_for_function_params(entry_block);
            b.switch_to_block(entry_block);
            // b.seal_block(entry_block); // Seal at the end instead

            let out_ptr      = b.block_params(entry_block)[0];
            let locals_ptr   = b.block_params(entry_block)[1];
            let globals_ptr  = b.block_params(entry_block)[2];
            let consts_ptr   = b.block_params(entry_block)[3];
            let vm_ptr       = b.block_params(entry_block)[4];
            let exec_ptr     = b.block_params(entry_block)[5];
            let shutdown_ptr = b.block_params(entry_block)[6];

            let start_ip = trace_read.start_ip;
            
            let (call_depth_offset, stack_ptr_offset) = super::codegen_ctx::executor_field_offsets();

            let mut ctx = CodegenCtx::new(&mut b, out_ptr, locals_ptr, globals_ptr, consts_ptr, vm_ptr, exec_ptr, shutdown_ptr, start_ip, trace_read.min_locals, HashMap::new(), u32::MAX, None, call_depth_offset, stack_ptr_offset);
            
            // Phase 4: Analyze used locals and preload them
            let used_locals = analyze_trace_locals(&trace_read.ops);
            ctx.preload_locals(&used_locals);
            
            let used_globals = analyze_trace_globals(&trace_read.ops);
            ctx.preload_globals(&used_globals);

            let trace_global_ints = analyze_trace_global_ints(&trace_read.ops);
            ctx.set_global_int_regs(trace_global_ints.clone());
            let trace_non_ptr = analyze_trace_non_ptr_regs(&trace_read.ops, &trace_global_ints);
            ctx.set_non_ptr_regs(trace_non_ptr);
            ctx.uses_heap = false;
            for op in &trace_read.ops {
                if let crate::vm::trace::TraceOp::ArrayGet { .. }
                    | crate::vm::trace::TraceOp::ArrayGetIndex { .. }
                    | crate::vm::trace::TraceOp::GetMember { .. }
                    | crate::vm::trace::TraceOp::JsonBindLocal { .. }
                    | crate::vm::trace::TraceOp::JsonBindLocalConst { .. }
                    | crate::vm::trace::TraceOp::JsonParse { .. }
                    | crate::vm::trace::TraceOp::FiberNext { .. }
                    | crate::vm::trace::TraceOp::Call { .. }
                    | crate::vm::trace::TraceOp::TableCloneSkeleton { .. }
                    | crate::vm::trace::TraceOp::RowGet { .. } = op {
                    ctx.uses_heap = true;
                    break;
                }
            }

            ctx.b.ins().jump(loop_head, &[]);
            ctx.b.switch_to_block(loop_head);
            ctx.clear_block_state(true);

            // Shutdown check
            let shutdown_val = ctx.b.ins().load(types::I8, trusted(), shutdown_ptr, 0);
            let is_shutdown = ctx.b.ins().icmp_imm(IntCC::NotEqual, shutdown_val, 0);
            let continue_blk = ctx.create_block();
            let exit_blk     = ctx.create_block();
            ctx.b.ins().brif(is_shutdown, exit_blk, &[], continue_blk, &[]);

            ctx.b.switch_to_block(exit_blk);
            // ctx.b.seal_block(exit_blk); // Seal at the end
            ctx.spill_all();
            let rv_zero = ctx.b.ins().iconst(types::I32, 0); // Return 0 to signal fallback/end
            ctx.b.ins().return_(&[rv_zero]);

            ctx.b.switch_to_block(continue_blk);
            // ctx.b.seal_block(continue_blk); // Seal at the end

            let mut terminated = false;

            for op in &trace_read.ops {
                if terminated { break; }
                match *op {
                    TraceOp::LoadConst { dst, val } => {
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        let bits = ctx.b.ins().iconst(types::I64, val.bits as i64);
                        let tag  = ctx.b.ins().iconst(types::I64, val.tag as i64);
                        emit_conditional_inc_ref(&mut ctx, &symbols, bits, tag);
                        ctx.def_local(dst, bits, tag);
                        ctx.known_types[dst as usize] = if val.is_int() {
                            crate::vm::opcode::TypeTag::Int
                        } else if val.is_float() {
                            crate::vm::opcode::TypeTag::Float
                        } else if val.is_bool() {
                            crate::vm::opcode::TypeTag::Bool
                        } else if val.is_string() {
                            crate::vm::opcode::TypeTag::String
                        } else {
                            crate::vm::opcode::TypeTag::Unknown
                        };
                    }
                    TraceOp::Move { dst, src } => {
                        emit_move(&mut ctx, &symbols, dst, src);
                        ctx.known_types[dst as usize] = ctx.known_types[src as usize];
                    }
                    TraceOp::GetVar { dst, idx } => {
                        emit_get_var(&mut ctx, &symbols, dst, idx);
                    }
                    TraceOp::SetVar { idx, src } => {
                        emit_set_var(&mut ctx, &symbols, idx, src);
                    }
                    TraceOp::AddInt { dst, src1, src2 } => {
                        emit_add_int(&mut ctx, &symbols, dst, src1, src2);
                    }
                    TraceOp::AddFloat { dst, src1, src2 } => {
                        emit_add_float(&mut ctx, &symbols, dst, src1, src2);
                    }
                    TraceOp::SubInt { dst, src1, src2 } => {
                        emit_sub_int(&mut ctx, &symbols, dst, src1, src2);
                    }
                    TraceOp::SubFloat { dst, src1, src2 } => {
                        emit_sub_float(&mut ctx, &symbols, dst, src1, src2);
                    }
                    TraceOp::MulInt { dst, src1, src2 } => {
                        emit_mul_int(&mut ctx, &symbols, dst, src1, src2);
                    }
                    TraceOp::MulFloat { dst, src1, src2 } => {
                        emit_mul_float(&mut ctx, &symbols, dst, src1, src2);
                    }
                    TraceOp::DivInt { dst, src1, src2, fail_ip } => {
                        emit_div_int(&mut ctx, &symbols, dst, src1, src2, fail_ip, false);
                    }
                    TraceOp::DivFloat { dst, src1, src2, fail_ip: _ } => {
                        emit_div_float(&mut ctx, &symbols, dst, src1, src2);
                    }
                    TraceOp::ModInt { dst, src1, src2, fail_ip } => {
                        emit_mod_int(&mut ctx, &symbols, dst, src1, src2, fail_ip);
                    }
                    TraceOp::IntConcat { dst, src1, src2 } => {
                        emit_int_concat(&mut ctx, &symbols, dst, src1, src2);
                    }
                    TraceOp::NegInt { dst, src } => {
                        emit_neg_int(&mut ctx, &symbols, dst, src);
                    }
                    TraceOp::NegFloat { dst, src } => {
                        emit_neg_float(&mut ctx, &symbols, dst, src);
                    }
                    TraceOp::CastIntToFloat { dst, src } => {
                        emit_cast_to_float(&mut ctx, &symbols, dst, src);
                    }
                    TraceOp::CastFloatToInt { dst, src } => {
                        emit_cast_to_int(&mut ctx, &symbols, dst, src);
                    }

                    TraceOp::GuardInt { reg, ip } => {
                        emit_guard_int(&mut ctx, &symbols, reg, ip);
                    }
                    TraceOp::GuardFloat { reg, ip } => {
                        emit_guard_float(&mut ctx, &symbols, reg, ip);
                    }
                    TraceOp::GuardTrue { reg, fail_ip } => {
                        emit_guard_bool(&mut ctx, &symbols, reg, fail_ip, true);
                    }
                    TraceOp::GuardFalse { reg, fail_ip } => {
                        emit_guard_bool(&mut ctx, &symbols, reg, fail_ip, false);
                    }

                    TraceOp::CmpInt { dst, src1, src2, cc } => {
                        emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, cc);
                    }
                    TraceOp::CmpFloat { dst, src1, src2, cc } => {
                        emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, cc);
                    }

                    TraceOp::Jump { target_ip } => {
                        let cond = ctx.b.ins().iconst(types::I32, 1);
                        emit_loop_exit(&mut ctx, cond, loop_head, target_ip, start_ip, target_ip, &mut terminated);
                    }
                    TraceOp::LoopNextInt { reg, limit_reg, target, exit_ip } => {
                        emit_loop_next_int(&mut ctx, &symbols, reg, limit_reg, loop_head, target as usize, start_ip, exit_ip, &mut terminated);
                    }
                    TraceOp::IncLocalLoopNext { inc_reg, reg, limit_reg, target, exit_ip } => {
                        emit_inc_local_loop_next(&mut ctx, &symbols, inc_reg, reg, limit_reg, loop_head, target as usize, start_ip, exit_ip, &mut terminated);
                    }
                    TraceOp::ArrayLoopNext { idx_reg, size_reg, target, exit_ip } => {
                        emit_array_loop_next(&mut ctx, idx_reg, size_reg, loop_head, target, start_ip, exit_ip, &mut terminated);
                    }
                    TraceOp::IncVarLoopNext { g_idx, reg, limit_reg, target, exit_ip } => {
                        emit_inc_var_loop_next(&mut ctx, &symbols, g_idx, reg, limit_reg, loop_head, target as usize, start_ip, exit_ip, &mut terminated);
                    }

                    TraceOp::Call { dst, func_idx, base, arg_count } => {
                        emit_call(&mut ctx, &symbols, dst, func_idx, base, arg_count);
                    }

                    TraceOp::IncLocal { reg } => {
                        emit_inc_local(&mut ctx, reg);
                    }
                    TraceOp::IncVar { g_idx } => {
                        emit_inc_var(&mut ctx, &symbols, g_idx);
                    }

                    TraceOp::ArraySize { dst, src } => {
                        let (av_bits, av_tag) = ctx.use_local(src);
                        let call = ctx.b.ins().call(symbols.xcx_jit_array_size, &[av_bits, av_tag]);
                        let res = ctx.b.inst_results(call)[0];
                        let res_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res, res_tag);
                    }
                    TraceOp::ArrayGet { dst, arr_reg, idx_reg, .. } => {
                        let (av_bits, av_tag) = ctx.use_local(arr_reg);
                        let (iv_bits, _) = ctx.use_local(idx_reg);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_get, &[av_bits, av_tag, iv_bits]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    TraceOp::ArrayPush { arr_reg, val_reg } => {
                        let (av_bits, av_tag) = ctx.use_local(arr_reg);
                        let (vv_bits, vv_tag) = ctx.use_local(val_reg);
                        ctx.b.ins().call(symbols.xcx_jit_array_push, &[av_bits, av_tag, vv_bits, vv_tag]);
                    }
                    TraceOp::ArrayUpdate { arr_reg, idx_reg, val_reg, .. } => {
                        let (av_bits, av_tag) = ctx.use_local(arr_reg);
                        let (iv_bits, _) = ctx.use_local(idx_reg);
                        let (vv_bits, vv_tag) = ctx.use_local(val_reg);
                        ctx.b.ins().call(symbols.xcx_jit_array_update, &[av_bits, av_tag, iv_bits, vv_bits, vv_tag]);
                    }
                    TraceOp::SetSize { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let call = ctx.b.ins().call(symbols.xcx_jit_set_size, &[sv_bits, sv_tag]);
                        let res = ctx.b.inst_results(call)[0];
                        let res_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res, res_tag);
                    }
                    TraceOp::SetContains { dst, set_reg, val_reg } => {
                        let (sv_bits, sv_tag) = ctx.use_local(set_reg);
                        let (vv_bits, vv_tag) = ctx.use_local(val_reg);
                        let call = ctx.b.ins().call(symbols.xcx_jit_set_contains, &[sv_bits, sv_tag, vv_bits, vv_tag]);
                        let res = ctx.b.inst_results(call)[0];
                        let res_v = ctx.b.ins().uextend(types::I64, res);
                        let res_tag = ctx.b.ins().iconst(types::I64, TAG_BOOL as i64);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_v, res_tag);
                    }

                    TraceOp::RowGet { dst, row_reg, col_idx } => {
                        emit_row_get(&mut ctx, &symbols, dst, row_reg, col_idx);
                    }
                    TraceOp::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, target: _, exit_ip } => {
                        emit_table_iter(&mut ctx, &symbols, tbl_reg, idx_reg, row_reg, limit_reg, loop_head, start_ip, exit_ip, &mut terminated);
                    }
                    TraceOp::TablePushRow { tbl_reg, row_reg } => {
                        emit_table_push_row(&mut ctx, &symbols, tbl_reg, row_reg);
                    }
                    TraceOp::TableCloneSkeleton { dst, src } => {
                        emit_table_clone_skeleton(&mut ctx, &symbols, dst, src);
                    }
                    TraceOp::TableSize { dst, src } => {
                        emit_table_size(&mut ctx, &symbols, dst, src);
                    }
                    TraceOp::JsonBindLocal { dst, json_reg, path_reg } => {
                        emit_json_bind_local(&mut ctx, &symbols, dst, json_reg, path_reg);
                    }
                    TraceOp::JsonBindLocalConst { dst, json_reg, ref path } => {
                        emit_json_bind_local_const(&mut ctx, &symbols, dst, json_reg, path);
                    }
                    TraceOp::JsonBindGlobal { idx, json_reg, path_reg } => {
                        emit_json_bind_global(&mut ctx, &symbols, idx, json_reg, path_reg);
                    }
                    TraceOp::JsonBindGlobalConst { idx, json_reg, ref path } => {
                        emit_json_bind_global_const(&mut ctx, &symbols, idx, json_reg, path);
                    }
                    TraceOp::GetMember { dst, obj_reg, ref name } => {
                        let name_ptr = name.as_ptr() as i64;
                        let name_len = name.len() as i64;
                        let np = ctx.b.ins().iconst(types::I64, name_ptr);
                        let nl = ctx.b.ins().iconst(types::I64, name_len);
                        let (cv_bits, cv_tag) = ctx.use_local(obj_reg);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_get_member, &[cv_bits, cv_tag, np, nl]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    TraceOp::StringLength { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let call = ctx.b.ins().call(symbols.xcx_jit_string_length, &[sv_bits, sv_tag]);
                        let res = ctx.b.inst_results(call)[0];
                        let res_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res, res_tag);
                    }
                    TraceOp::CastBool { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_cast_bool, &[sv_bits, sv_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (o_bits, o_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, o_bits, o_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    TraceOp::JsonParse { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_json_parse, &[sv_bits, sv_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    TraceOp::JsonFastGetPush { json_src, path_src, val_src } => {
                        let (json_bits, json_tag) = ctx.use_local(json_src);
                        let (path_bits, path_tag) = ctx.use_local(path_src);
                        let (val_bits, val_tag) = ctx.use_local(val_src);
                        ctx.b.ins().call(symbols.xcx_jit_json_get_push, &[json_bits, json_tag, path_bits, path_tag, val_bits, val_tag]);
                    }
                    TraceOp::DateNow { dst } => {
                        ctx.spill_all();
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_date_now, &[]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    TraceOp::ArrayGetIndex { dst, arr_reg, idx_reg, fail_ip: _ } => {
                        let (av_bits, av_tag) = ctx.use_local(arr_reg);
                        let (iv_bits, _) = ctx.use_local(idx_reg);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_get, &[av_bits, av_tag, iv_bits]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    TraceOp::ArraySetIndex { arr_reg, idx_reg, val_reg, fail_ip: _ } => {
                        let (av_bits, av_tag) = ctx.use_local(arr_reg);
                        let (iv_bits, _) = ctx.use_local(idx_reg);
                        let (vv_bits, vv_tag) = ctx.use_local(val_reg);
                        ctx.b.ins().call(symbols.xcx_jit_array_update, &[av_bits, av_tag, iv_bits, vv_bits, vv_tag]);
                    }
                    TraceOp::FiberIsDone { dst, src } => {
                        let (fv_bits, fv_tag) = ctx.use_local(src);
                        let call = ctx.b.ins().call(symbols.xcx_jit_fiber_is_done, &[fv_bits, fv_tag]);
                        let res = ctx.b.inst_results(call)[0];
                        let res_v = ctx.b.ins().uextend(types::I64, res);
                        let res_tag = ctx.b.ins().iconst(types::I64, TAG_BOOL as i64);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_v, res_tag);
                    }
                    TraceOp::FiberNext { dst, src } => {
                        let (fv_bits, fv_tag) = ctx.use_local(src);
                        ctx.spill_all();
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_fiber_next, &[fv_bits, fv_tag, ctx.executor_ptr]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }

                    _ => {}
                }
            }

            if !terminated {
                ctx.spill_all();
                let rv_zero = ctx.b.ins().iconst(types::I32, 0);
                ctx.b.ins().return_(&[rv_zero]);
            }
            
            ctx.b.seal_block(entry_block);
            ctx.b.seal_block(loop_head);
            for &block in &ctx.created_blocks {
                ctx.b.seal_block(block);
            }
        }
        b.finalize();
        
        self.module.define_function(func_id, &mut self.ctx).map_err(|e| format!("define_function: {:#?}", e))?;
        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().map_err(|e| format!("finalize_definitions: {}", e))?;
        let code = self.module.get_finalized_function(func_id);

        Ok(code)
    }
}
