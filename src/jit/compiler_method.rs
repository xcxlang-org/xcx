use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

use crate::vm::opcode::{OpCode, Chunk, TypeTag};
use crate::vm::value::Value as VMValue;
use crate::vm::value::TAG_BOOL;

use super::codegen_ctx::CodegenCtx;
use super::symbols::SymbolRegistry;
use super::type_inference::analyze_chunk_types;
use super::analysis::*;
use super::emit_arith::*;
use super::emit_control::*;
use super::emit_load_store::*;
use super::emit_call::*;
use super::emit_object::*;
use super::emit_misc::*;
use super::nan_ops::*;
use super::jit::JIT;

impl JIT {
    pub fn compile_method_impl(
        &mut self,
        func_id_idx: usize,
        self_func_idx: u32,
        chunk: &Chunk,
        constants: &[VMValue],
        functions: &[std::sync::Arc<Chunk>],
        _name: &str,
        is_inner_func: bool,
        inner_func_id: Option<cranelift_module::FuncId>,
    ) -> Result<*const std::ffi::c_void, String> {
        self.module.clear_context(&mut self.ctx);
        
        let mut sig = self.module.make_signature();
        let ptr_type = self.module.target_config().pointer_type();
        if is_inner_func {
            for _ in 0..chunk.arity {
                sig.params.push(AbiParam::new(types::I64)); // bits
                sig.params.push(AbiParam::new(types::I64)); // tag
            }
            sig.params.push(AbiParam::new(ptr_type)); // out_ptr
            sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
            sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
            sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
            sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
            sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
            sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
            sig.returns.push(AbiParam::new(types::I32)); // status
        } else {
            sig.params.push(AbiParam::new(ptr_type)); // out_ptr
            sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
            sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
            sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
            sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
            sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
            sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
            sig.returns.push(AbiParam::new(types::I32)); // status
        }

        let func_id = if is_inner_func {
            inner_func_id.unwrap()
        } else {
            self.module.declare_function(
                &format!("method_{}", func_id_idx),
                Linkage::Export,
                &sig,
            ).map_err(|e: cranelift_module::ModuleError| e.to_string())?
        };

        self.ctx.func.signature = sig;
        
        let local_callee = if let Some(fid) = inner_func_id {
            Some(self.module.declare_func_in_func(fid, &mut self.ctx.func))
        } else if self_func_idx != u32::MAX {
            Some(self.module.declare_func_in_func(func_id, &mut self.ctx.func))
        } else {
            None
        };
        let registry = SymbolRegistry::new(&mut self.module);
        let symbols = registry.import_in_func(&mut self.module, &mut self.ctx.func);

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let mut blocks: HashMap<usize, Block> = HashMap::new();
        
        let entry_block = b.create_block();
        blocks.insert(0, entry_block);

        for (ip, op) in chunk.bytecode.iter().enumerate() {
            let _ = ip;
            match op {
                OpCode::Jump { target } => { blocks.entry(*target as usize).or_insert_with(|| b.create_block()); }
                OpCode::JumpIfFalse { target, .. } | OpCode::JumpIfTrue { target, .. } => {
                    blocks.entry(*target as usize).or_insert_with(|| b.create_block());
                }
                OpCode::LoopNext { target, .. } | OpCode::LoopPrev { target, .. } | OpCode::IncLocalLoopNext { target, .. } | 
                OpCode::DecLocalLoopPrev { target, .. } |
                OpCode::IncVarLoopNext { target, .. } | OpCode::DecVarLoopPrev { target, .. } | OpCode::ArrayLoopNext { target, .. } |
                OpCode::TableIter { target, .. } => {
                    blocks.entry(*target as usize).or_insert_with(|| b.create_block());
                }
                _ => {}
            }
        }

        b.append_block_params_for_function_params(entry_block);
        b.switch_to_block(entry_block);

        let arg_vals = b.block_params(entry_block).to_vec();
        let out_ptr = if is_inner_func { arg_vals[chunk.arity * 2] } else { arg_vals[0] };
        let locals_ptr = if is_inner_func { arg_vals[chunk.arity * 2 + 1] } else { arg_vals[1] };
        let globals_ptr = if is_inner_func { arg_vals[chunk.arity * 2 + 2] } else { arg_vals[2] };
        let consts_ptr = if is_inner_func { arg_vals[chunk.arity * 2 + 3] } else { arg_vals[3] };
        let vm_ptr = if is_inner_func { arg_vals[chunk.arity * 2 + 4] } else { arg_vals[4] };
        let executor_ptr = if is_inner_func { arg_vals[chunk.arity * 2 + 5] } else { arg_vals[5] };
        let shutdown_ptr = if is_inner_func { arg_vals[chunk.arity * 2 + 6] } else { arg_vals[6] };

        {
            let (call_depth_offset, stack_ptr_offset) = super::codegen_ctx::executor_field_offsets();

            let mut ctx = CodegenCtx::new(
                &mut b, out_ptr, locals_ptr, globals_ptr, consts_ptr, 
                vm_ptr, executor_ptr, shutdown_ptr,
                0, chunk.max_locals, blocks.clone(),
                self_func_idx,
                local_callee,
                call_depth_offset,
                stack_ptr_offset,
            );
            ctx.set_functions(functions);
            ctx.is_inner_func = is_inner_func;

            let used_locals = analyze_chunk_locals(&chunk.bytecode);


            let bool_array_hints = analyze_bool_array_regs(&chunk.bytecode, constants);
            let (inferred_types, uses_heap) = analyze_chunk_types(&chunk.bytecode, constants, None, chunk.arity, self_func_idx, &bool_array_hints);



            
            // Heuristic: if it's a pure math function, we can elide heap tracking.
            // A function is pure math if it never assigns a non-primitive type.
            if !uses_heap {
                // Optimization: if no heap is used, we don't need any GC cleanup.
            }
            chunk.uses_heap.store(uses_heap, std::sync::atomic::Ordering::Release);
            let global_ints = analyze_global_int_regs(&chunk.bytecode, constants);
            ctx.set_global_int_regs(global_ints.clone());
            let non_ptr_regs = analyze_non_ptr_regs(&chunk.bytecode, chunk.arity, &global_ints, constants);
            ctx.set_non_ptr_regs(non_ptr_regs);
            let may_contain_ptr = analyze_maybe_ptr_regs(&chunk.bytecode, &global_ints, constants);
            ctx.set_may_contain_ptr(may_contain_ptr);
            
            ctx.set_reg_types_per_ip(inferred_types);
            ctx.uses_heap = uses_heap;

            // XCX_JIT_DEBUG removed
            
            if is_inner_func {
                for i in 0..chunk.arity {
                    let bits = arg_vals[i * 2];
                    let tag = arg_vals[i * 2 + 1];
                    ctx.def_local(i as u8, bits, tag);
                }
            }
            
            let mut filtered_locals = Vec::new();
            for &reg in &used_locals {
                if !is_inner_func || (reg as usize) >= chunk.arity {
                    filtered_locals.push(reg);
                }
            }
            let needs_init_vec = analyze_chunk_locals_init(&chunk.bytecode, chunk.arity as u8);
            let mut needs_init: std::collections::HashSet<u8> = needs_init_vec.into_iter().collect();
            if !is_inner_func {
                for i in 0..chunk.arity as u8 {
                    needs_init.insert(i);
                }
            }
            ctx.preload_locals(&filtered_locals, &needs_init);

            let used_globals = analyze_chunk_globals(&chunk.bytecode);
            ctx.preload_globals(&used_globals);



            // Use entry_block as block_0 (it's already switched-to and parameters are loaded)
            let mut terminated = false;

            for (ip, op) in chunk.bytecode.iter().enumerate() {
                // XCX_JIT_DEBUG removed
                if let Some(&block) = blocks.get(&ip) {
                    if ip > 0 {
                        if !terminated { 
                            ctx.sync_for_jump();
                            ctx.b.ins().jump(block, &[]); 
                        }
                        ctx.b.switch_to_block(block);
                        ctx.clear_block_state(false);
                        terminated = false;
                    }
                }
                if terminated { continue; }
                ctx.update_current_reg_types(ip);

                match *op {
                    OpCode::LoadConst { dst, idx } => {
                        emit_load_const(&mut ctx, &symbols, dst, idx, constants);
                        let val = constants[idx as usize];
                        if val.is_int() {
                            ctx.register_const[dst as usize] = Some(val.as_i64());
                        } else {
                            ctx.register_const[dst as usize] = None;
                        }
                    }
                    OpCode::Move { dst, src } => {
                        emit_move(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::Add { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_add_int(&mut ctx, &symbols, dst, src1, src2);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_add_float(&mut ctx, &symbols, dst, src1, src2);
                        } else {
                            emit_add_poly(&mut ctx, &symbols, dst, src1, src2);
                        }
                    }
                    OpCode::Sub { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_sub_int(&mut ctx, &symbols, dst, src1, src2);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_sub_float(&mut ctx, &symbols, dst, src1, src2);
                        } else {
                            emit_sub_poly(&mut ctx, &symbols, dst, src1, src2);
                        }
                    }
                    OpCode::Mul { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_mul_int(&mut ctx, &symbols, dst, src1, src2);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_mul_float(&mut ctx, &symbols, dst, src1, src2);
                        } else {
                            emit_mul_poly(&mut ctx, &symbols, dst, src1, src2);
                        }
                    }
                    OpCode::Div { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_poly_div_mod_fast_path(&mut ctx, &symbols, dst, src1, src2, symbols.xcx_jit_div, false);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_div_float(&mut ctx, &symbols, dst, src1, src2);
                        } else {
                            emit_poly_div_mod_fast_path(&mut ctx, &symbols, dst, src1, src2, symbols.xcx_jit_div, false);
                        }
                    }
                    OpCode::Mod { dst, src1, src2 } => {
                        emit_poly_div_mod_fast_path(&mut ctx, &symbols, dst, src1, src2, symbols.xcx_jit_mod, true);
                    }
                    OpCode::Neg { dst, src } => {
                        let t = ctx.get_reg_type(src as usize);
                        if t == TypeTag::Int {
                            emit_neg_int(&mut ctx, &symbols, dst, src);
                        } else if t == TypeTag::Float {
                            emit_neg_float(&mut ctx, &symbols, dst, src);
                        } else {
                            emit_neg_poly(&mut ctx, &symbols, dst, src);
                        }
                    }
                    OpCode::IncLocal { reg } => {
                        emit_inc_local(&mut ctx, reg);
                    }
                    OpCode::DecLocal { reg } => {
                        emit_dec_local(&mut ctx, reg);
                    }
                     OpCode::GetVar { dst, idx } => {
                        emit_get_var(&mut ctx, &symbols, dst, idx);
                    }
                    OpCode::SetVar { idx, src } => {
                        emit_set_var(&mut ctx, &symbols, idx, src);
                    }
                    OpCode::StrAppendVar { var_idx, src } => {
                        ctx.spill_all();
                        let idx_val = ctx.b.ins().iconst(types::I32, var_idx as i64);
                        let (s_bits, s_tag) = ctx.use_local(src);
                        ctx.b.ins().call(symbols.xcx_jit_str_append_var, &[
                            ctx.vm_ptr,
                            idx_val,
                            s_bits,
                            s_tag,
                        ]);
                        ctx.reload_globals();
                    }
                    OpCode::StrAppendLocal { local_idx, src } => {
                        ctx.spill_all();
                        let idx_val = ctx.b.ins().iconst(types::I32, local_idx as i64);
                        let (s_bits, s_tag) = ctx.use_local(src);
                        ctx.b.ins().call(symbols.xcx_jit_str_append_local, &[
                            ctx.locals_ptr,
                            idx_val,
                            s_bits,
                            s_tag,
                        ]);
                        ctx.reload_local(local_idx);
                    }
                    OpCode::Equal { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 0);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 0);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 0);
                        }
                    }
                    OpCode::NotEqual { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 1);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 1);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 1);
                        }
                    }
                    OpCode::Greater { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 2);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 2);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 2);
                        }
                    }
                    OpCode::Less { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 3);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 3);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 3);
                        }
                    }
                    OpCode::GreaterEqual { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 4);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 4);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 4);
                        }
                    }
                    OpCode::LessEqual { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 5);
                        } else if t1 == TypeTag::Float && t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 5);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 5);
                        }
                    }
                    OpCode::IntConcat { dst, src1, src2 } => {
                        emit_int_concat(&mut ctx, &symbols, dst, src1, src2);
                    }
                    OpCode::CastInt { dst, src } => {
                        emit_cast_to_int(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::CastFloat { dst, src } => {
                        emit_cast_to_float(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::CastString { dst, src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_cast_string, &[v_bits, v_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::Print { src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        ctx.b.ins().call(symbols.xcx_jit_print, &[v_bits, v_tag]);
                    }
                    OpCode::Jump { target } => {
                        emit_jump(&mut ctx, &blocks, target, &mut terminated);
                    }
                    OpCode::JumpIfFalse { src, target } => {
                        emit_jump_if(&mut ctx, &symbols, &blocks, src, target, false);
                    }
                    OpCode::JumpIfTrue { src, target } => {
                        emit_jump_if(&mut ctx, &symbols, &blocks, src, target, true);
                    }
                    OpCode::Call { dst, func_idx, base, arg_count } => {
                        emit_call(&mut ctx, &symbols, dst, func_idx, base, arg_count);
                    }
                    OpCode::Return { src } => {
                        emit_return(&mut ctx, &symbols, Some(src), &mut terminated);
                    }
                    OpCode::ReturnVoid => {
                        emit_return(&mut ctx, &symbols, None, &mut terminated);
                    }
                    OpCode::Yield { src } => {
                        let _ = ctx.use_local(src);
                    }
                    OpCode::YieldWithTarget { dst, src } => {
                        emit_move(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::YieldVoid => {
                        emit_method_yield_void(&mut ctx, &symbols, ip + 1, &mut terminated);
                    }
                    OpCode::Wait { src } => {
                        let (v_bits, _v_tag) = ctx.use_local(src);
                        let ms = ctx.b.ins().band_imm(v_bits, 0x0000_FFFF_FFFF_FFFF_i64);
                        ctx.b.ins().call(symbols.xcx_jit_wait, &[ms]);
                    }
                    OpCode::MethodCall { dst, kind, base, arg_count } => {
                        emit_method_call(&mut ctx, &symbols, dst, kind, base, arg_count);
                    }
                    OpCode::MethodCallNamed { dst, kind, base, arg_count, names_idx } => {
                        emit_method_call_named(&mut ctx, &symbols, dst, kind, base, arg_count, names_idx, constants);
                    }
                    OpCode::GetIndex { dst, container, index } => {
                        let (cv_bits, cv_tag) = ctx.use_local(container);
                        let (iv_bits, _iv_tag) = ctx.use_local(index);
                        let container_ty = ctx.get_reg_type(container as usize);
                        if container_ty == TypeTag::BoolArray {
                            let elements_ptr = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), cv_bits, 16);
                            let len = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), cv_bits, 24);
                            let is_in_bounds = ctx.b.ins().icmp(IntCC::UnsignedLessThan, iv_bits, len);
                            
                            let fast_blk = ctx.create_block();
                            let slow_blk = ctx.create_block();
                            let join_blk = ctx.create_block();
                            let res_val = ctx.b.declare_var(types::I64);
                            
                            ctx.b.ins().brif(is_in_bounds, fast_blk, &[], slow_blk, &[]);
                            
                            ctx.b.switch_to_block(fast_blk);
                            let elem_addr = ctx.b.ins().iadd(elements_ptr, iv_bits);
                            let raw_val_i8 = ctx.b.ins().load(types::I8, cranelift_codegen::ir::MemFlags::trusted(), elem_addr, 0);
                            let raw_val = ctx.b.ins().uextend(types::I64, raw_val_i8);
                            let val_bool = ctx.b.ins().band_imm(raw_val, 1);
                            ctx.b.def_var(res_val, val_bool);
                            ctx.b.ins().jump(join_blk, &[]);
                            
                            ctx.b.switch_to_block(slow_blk);
                            let call = ctx.b.ins().call(symbols.xcx_jit_array_get_bool, &[cv_bits, cv_tag, iv_bits]);
                            let r_bits = ctx.b.inst_results(call)[0];
                            ctx.b.def_var(res_val, r_bits);
                            ctx.b.ins().jump(join_blk, &[]);
                            
                            ctx.b.switch_to_block(join_blk);
                            ctx.clear_block_state(false);
                            let final_bits = ctx.b.use_var(res_val);
                            if !ctx.should_skip_dec_ref(dst) {
                                let (old_bits, old_tag) = ctx.use_local(dst);
                                emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                            }
                            let bool_tag = ctx.b.ins().iconst(types::I64, TAG_BOOL as i64);
                            ctx.def_local(dst, final_bits, bool_tag);
                        } else if container_ty == TypeTag::Table {
                            let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_table_get_row, &[cv_bits, cv_tag, iv_bits]);
                            if !ctx.should_skip_dec_ref(dst) {
                                let (old_bits, old_tag) = ctx.use_local(dst);
                                emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                            }
                            ctx.def_local(dst, res_bits, res_tag);
                        } else {
                            let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_get, &[cv_bits, cv_tag, iv_bits]);
                            if !ctx.should_skip_dec_ref(dst) {
                                let (old_bits, old_tag) = ctx.use_local(dst);
                                emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                            }
                            ctx.def_local(dst, res_bits, res_tag);
                        }
                    }
                    OpCode::SetIndex { container, index, src } => {
                        let (cv_bits, cv_tag) = ctx.use_local(container);
                        let (iv_bits, _iv_tag) = ctx.use_local(index);
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        if ctx.get_reg_type(container as usize) == TypeTag::BoolArray {
                            let unpacked_bool = if ctx.get_reg_type(src as usize) == TypeTag::Bool {
                                sv_bits
                            } else {
                                super::nan_ops::unpack_bool(ctx.b, sv_bits)
                            };
                            
                            let elements_ptr = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), cv_bits, 16);
                            let len = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), cv_bits, 24);
                            let is_in_bounds = ctx.b.ins().icmp(IntCC::UnsignedLessThan, iv_bits, len);
                            
                            let fast_blk = ctx.create_block();
                            let slow_blk = ctx.create_block();
                            let join_blk = ctx.create_block();
                            
                            ctx.b.ins().brif(is_in_bounds, fast_blk, &[], slow_blk, &[]);
                            
                            ctx.b.switch_to_block(fast_blk);
                            let elem_addr = ctx.b.ins().iadd(elements_ptr, iv_bits);
                            let val_norm = ctx.b.ins().band_imm(unpacked_bool, 1);
                            let val_bool_i8 = ctx.b.ins().ireduce(types::I8, val_norm);
                            ctx.b.ins().store(cranelift_codegen::ir::MemFlags::trusted(), val_bool_i8, elem_addr, 0);
                            ctx.b.ins().jump(join_blk, &[]);
                            
                            ctx.b.switch_to_block(slow_blk);
                            let sv_bits_i8 = ctx.b.ins().ireduce(types::I8, unpacked_bool);
                            ctx.b.ins().call(symbols.xcx_jit_array_set_bool, &[cv_bits, cv_tag, iv_bits, sv_bits_i8]);
                            ctx.b.ins().jump(join_blk, &[]);
                            
                            ctx.b.switch_to_block(join_blk);
                            ctx.clear_block_state(false);
                        } else {
                            ctx.b.ins().call(symbols.xcx_jit_array_update, &[cv_bits, cv_tag, iv_bits, sv_bits, sv_tag]);
                        }
                    }
                    OpCode::JsonParse { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_json_parse, &[sv_bits, sv_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::JsonFastGetPush { json_src, path_src, val_src } => {
                        let (json_bits, json_tag) = ctx.use_local(json_src);
                        let (path_bits, path_tag) = ctx.use_local(path_src);
                        let (val_bits, val_tag) = ctx.use_local(val_src);
                        ctx.b.ins().call(symbols.xcx_jit_json_get_push, &[json_bits, json_tag, path_bits, path_tag, val_bits, val_tag]);
                    }
                    OpCode::JsonBind { idx, json_src, path_src } => {
                        emit_json_bind_global(&mut ctx, &symbols, idx, json_src, path_src);
                    }
                    OpCode::JsonBindLocal { dst, json_src, path_src } => {
                        emit_json_bind_local(&mut ctx, &symbols, dst, json_src, path_src);
                    }
                    OpCode::DateNow { dst } => {
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_date_now, &[]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::PerfMs { dst } => {
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_perf_ms, &[ctx.vm_ptr]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::PerfUs { dst } => {
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_perf_us, &[ctx.vm_ptr]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::PerfNs { dst } => {
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_perf_ns, &[ctx.vm_ptr]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::CastBool { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_cast_int, &[sv_bits, sv_tag]);
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::LoopNext { reg, limit_reg, target } => {
                        emit_loop_next(&mut ctx, &symbols, &blocks, reg, limit_reg, target);
                    }
                    OpCode::LoopPrev { reg, limit_reg, target } => {
                        emit_loop_prev(&mut ctx, &symbols, &blocks, reg, limit_reg, target);
                    }
                    OpCode::IncLocalLoopNext { inc_reg, reg, limit_reg, target } => {
                        emit_inc_local_loop_next_opcode(&mut ctx, &symbols, &blocks, inc_reg, reg, limit_reg, target);
                    }
                    OpCode::DecLocalLoopPrev { dec_reg, reg, limit_reg, target } => {
                        emit_dec_local_loop_prev_opcode(&mut ctx, &symbols, &blocks, dec_reg, reg, limit_reg, target);
                    }
                    OpCode::IncVar { idx } => {
                        emit_inc_var(&mut ctx, &symbols, idx);
                    }
                    OpCode::DecVar { idx } => {
                        emit_dec_var(&mut ctx, &symbols, idx);
                    }
                    OpCode::IncVarLoopNext { g_idx, reg, limit_reg, target } => {
                        emit_inc_var_loop_next_opcode(&mut ctx, &symbols, &blocks, g_idx, reg, limit_reg, target);
                    }
                    OpCode::DecVarLoopPrev { g_idx, reg, limit_reg, target } => {
                        emit_dec_var_loop_prev_opcode(&mut ctx, &symbols, &blocks, g_idx, reg, limit_reg, target);
                    }
                    OpCode::ArrayLoopNext { idx_reg, size_reg, target } => {
                        emit_array_loop_next_opcode(&mut ctx, &symbols, &blocks, idx_reg, size_reg, target);
                    }
                    OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, target } => {
                        emit_table_iter_opcode(&mut ctx, &symbols, &blocks, tbl_reg, idx_reg, row_reg, limit_reg, target);
                    }
                    OpCode::HaltAlert { src } => {
                        emit_halt_alert(&mut ctx, &symbols, src);
                    }
                    OpCode::HaltError { src } => {
                        emit_halt_error(&mut ctx, &symbols, src, &mut terminated);
                    }
                    OpCode::HaltFatal { src } => {
                        emit_halt_fatal(&mut ctx, &symbols, src, &mut terminated);
                    }
                    OpCode::Typeof { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_typeof, &[sv_bits, sv_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::StoreRead { dst, base } => {
                        emit_store_read(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::StoreWrite { dst, base } => {
                        emit_store_write(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::StoreAppend { dst, base } => {
                        emit_store_append(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::StoreExists { dst, base } => {
                        emit_store_exists(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::StoreDelete { dst, base } => {
                        emit_store_delete(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::DatabaseInit { dst, engine_src, path_src, tables_base_reg, table_count } => {
                        emit_database_init(&mut ctx, &symbols, dst, engine_src, path_src, tables_base_reg, table_count);
                    }
                    OpCode::GetMember { dst, container, name_idx } => {
                        emit_get_member(&mut ctx, &symbols, dst, container, name_idx, constants);
                    }
                    OpCode::SetMember { container, name_idx, src } => {
                        emit_set_member(&mut ctx, &symbols, container, name_idx, src, constants);
                    }
                    OpCode::StrAppendMember { container, name_idx, src } => {
                        emit_str_append_member(&mut ctx, &symbols, container, name_idx, src, constants);
                    }
                    OpCode::StrAppendElement { container, index, src } => {
                        emit_str_append_element(&mut ctx, &symbols, container, index, src);
                    }
                    OpCode::TablePushRow { tbl_reg, row_reg } => {
                        emit_table_push_row(&mut ctx, &symbols, tbl_reg, row_reg);
                    }
                    OpCode::TableCloneSkeleton { dst, src } => {
                        emit_table_clone_skeleton(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::RowGet { dst, row_reg, col_idx } => {
                        emit_row_get(&mut ctx, &symbols, dst, row_reg, col_idx);
                    }
                    OpCode::EnvGet { dst, src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_env_get, &[v_bits, v_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::EnvArgs { dst } => {
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_env_args, &[]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::CryptoHash { dst, pass_src, alg_src } => {
                        let (p_bits, p_tag) = ctx.use_local(pass_src);
                        let (a_bits, a_tag) = ctx.use_local(alg_src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_crypto_hash, &[p_bits, p_tag, a_bits, a_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::CryptoVerify { dst, pass_src, hash_src, alg_src } => {
                         let (p_bits, p_tag) = ctx.use_local(pass_src);
                         let (h_bits, h_tag) = ctx.use_local(hash_src);
                         let (a_bits, a_tag) = ctx.use_local(alg_src);
                         let call = ctx.b.ins().call(symbols.xcx_jit_crypto_verify, &[p_bits, p_tag, h_bits, h_tag, a_bits, a_tag]);
                         let res_i32 = ctx.b.inst_results(call)[0];
                         let res_bits = ctx.b.ins().uextend(types::I64, res_i32);
                         let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                         if !ctx.should_skip_dec_ref(dst) {
                             let (old_bits, old_tag) = ctx.use_local(dst);
                             emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                         }
                         ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::CryptoToken { dst, len_src } => {
                        let (l_bits, l_tag) = ctx.use_local(len_src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_crypto_token, &[l_bits, l_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::RandomChoice { dst, src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_random_choice, &[v_bits, v_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::RandomInt { dst, min, max, step, has_step } => {
                        let (min_bits, min_tag) = ctx.use_local(min);
                        let (max_bits, max_tag) = ctx.use_local(max);
                        let (step_bits, step_tag) = ctx.use_local(step);
                        let (hs_bits, _hs_tag) = ctx.use_local(has_step);
                        let hs = ctx.b.ins().ireduce(types::I8, hs_bits);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_random_int, &[min_bits, min_tag, max_bits, max_tag, step_bits, step_tag, hs]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::RandomFloat { dst, min, max, step, has_step } => {
                        let (min_bits, min_tag) = ctx.use_local(min);
                        let (max_bits, max_tag) = ctx.use_local(max);
                        let (step_bits, step_tag) = ctx.use_local(step);
                        let (hs_bits, _hs_tag) = ctx.use_local(has_step);
                        let hs = ctx.b.ins().ireduce(types::I8, hs_bits);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_random_float, &[min_bits, min_tag, max_bits, max_tag, step_bits, step_tag, hs]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetRange { dst, start, end, step, has_step } => {
                        let (start_bits, start_tag) = ctx.use_local(start);
                        let (end_bits, end_tag) = ctx.use_local(end);
                        let (step_bits, step_tag) = ctx.use_local(step);
                        let (hs_bits, _hs_tag) = ctx.use_local(has_step);
                        let hs = ctx.b.ins().ireduce(types::I8, hs_bits);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_range, &[start_bits, start_tag, end_bits, end_tag, step_bits, step_tag, hs]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::And { dst, src1, src2 } => {
                        let (b1, _t1) = ctx.use_local(src1);
                        let (b2, _t2) = ctx.use_local(src2);
                        let res_bits = ctx.b.ins().band(b1, b2);
                        let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::Or { dst, src1, src2 } => {
                        let (b1, _t1) = ctx.use_local(src1);
                        let (b2, _t2) = ctx.use_local(src2);
                        let res_bits = ctx.b.ins().bor(b1, b2);
                        let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::Not { dst, src } => {
                        let (b, _t) = ctx.use_local(src);
                        let res_bits = ctx.b.ins().bxor_imm(b, 1);
                        let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::Has { dst, src1, src2 } => {
                        let (b1, t1) = ctx.use_local(src1);
                        let (b2, t2) = ctx.use_local(src2);
                        let call = ctx.b.ins().call(symbols.xcx_jit_has, &[b1, t1, b2, t2]);
                        let res_i8 = ctx.b.inst_results(call)[0];
                        let res_bits = ctx.b.ins().uextend(types::I64, res_i8);
                        let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::ArrayInit { dst, base, count } => {
                        ctx.spill_all();
                        let offset = (base as i64) * 16;
                        let elem_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);
                        let count_val = ctx.b.ins().iconst(types::I32, count as i64);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_init, &[ctx.executor_ptr, elem_ptr, count_val]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::BoolArrayInit { dst } => {
                        ctx.spill_all();
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_bool_array_init, &[ctx.executor_ptr]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::FiberCreate { dst, func_idx, base, arg_count } => {
                        ctx.spill_all();
                        let func_idx_val = ctx.b.ins().iconst(types::I64, func_idx as i64);
                        let base_val = ctx.b.ins().iconst(types::I8, base as i64);
                        let arg_count_val = ctx.b.ins().iconst(types::I8, arg_count as i64);
                        
                        let out_slot = ctx.b.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            16,
                            4,
                        ));
                        let out_ptr = ctx.b.ins().stack_addr(types::I64, out_slot, 0);

                        ctx.b.ins().call(symbols.xcx_jit_fiber_create, &[
                            out_ptr,
                            func_idx_val,
                            base_val,
                            arg_count_val,
                            ctx.executor_ptr,
                            ctx.locals_ptr,
                        ]);

                        let res_bits = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 0);
                        let res_tag  = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 8);

                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                        ctx.reload_globals();
                    }
                    OpCode::SetInit { dst, base, count } => {
                        ctx.spill_all();
                        let offset = (base as i64) * 16;
                        let elem_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);
                        let count_val = ctx.b.ins().iconst(types::I32, count as i64);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_init, &[ctx.executor_ptr, elem_ptr, count_val]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetUnion { dst, src1, src2 } => {
                        let (s1_bits, s1_tag) = ctx.use_local(src1);
                        let (s2_bits, s2_tag) = ctx.use_local(src2);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_union, &[s1_bits, s1_tag, s2_bits, s2_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetIntersection { dst, src1, src2 } => {
                        let (s1_bits, s1_tag) = ctx.use_local(src1);
                        let (s2_bits, s2_tag) = ctx.use_local(src2);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_intersection, &[s1_bits, s1_tag, s2_bits, s2_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetDifference { dst, src1, src2 } => {
                        let (s1_bits, s1_tag) = ctx.use_local(src1);
                        let (s2_bits, s2_tag) = ctx.use_local(src2);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_difference, &[s1_bits, s1_tag, s2_bits, s2_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetSymDifference { dst, src1, src2 } => {
                        let (s1_bits, s1_tag) = ctx.use_local(src1);
                        let (s2_bits, s2_tag) = ctx.use_local(src2);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_sym_difference, &[s1_bits, s1_tag, s2_bits, s2_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::MapInit { dst, base, count } => {
                        ctx.spill_all();
                        let offset = (base as i64) * 16;
                        let elem_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);
                        let count_val = ctx.b.ins().iconst(types::I32, count as i64);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_map_init, &[ctx.executor_ptr, elem_ptr, count_val]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::TableInit { dst, skeleton_idx, base, row_count, col_count } => {
                        ctx.spill_all();
                        let skel_val = ctx.b.ins().iconst(types::I32, skeleton_idx as i64);
                        let base_val = ctx.b.ins().iconst(types::I32, base as i64);
                        let rows_val = ctx.b.ins().iconst(types::I32, row_count as i64);
                        let cols_val = ctx.b.ins().iconst(types::I32, col_count as i64);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_table_init, &[
                            skel_val, base_val, rows_val, cols_val, ctx.locals_ptr, ctx.consts_ptr
                        ]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::MethodCallCustom { dst, method_name_idx, base, arg_count } => {
                        emit_method_call_custom(&mut ctx, &symbols, dst, method_name_idx, base, arg_count, constants);
                    }
                    OpCode::HttpCall { dst, method_idx, url_src, body_src } => {
                        ctx.spill_all();
                        let m_idx = ctx.b.ins().iconst(types::I64, method_idx as i64);
                        let (u_bits, u_tag) = ctx.use_local(url_src);
                        let (b_bits, b_tag) = ctx.use_local(body_src);
                        
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_net_call, &[
                            m_idx, u_bits, u_tag, b_bits, b_tag, ctx.consts_ptr
                        ]);
                        
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::HttpRequest { dst, arg_src } => {
                        ctx.spill_all();
                        let (a_bits, a_tag) = ctx.use_local(arg_src);
                        
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_net_request, &[
                            a_bits, a_tag
                        ]);
                        
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::HttpServe { func_idx, port_src, host_src, workers_src, routes_src } => {
                        emit_http_serve(&mut ctx, &symbols, func_idx, port_src, host_src, workers_src, routes_src);
                    }
                    OpCode::HttpRespond { dst, status_src, body_src, headers_src } => {
                        emit_http_respond(&mut ctx, &symbols, dst, status_src, body_src, headers_src);
                    }
                    OpCode::TerminalWrite { dst, src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        ctx.b.ins().call(symbols.xcx_jit_terminal_write, &[v_bits, v_tag]);
                        let true_bits = ctx.b.ins().iconst(types::I64, 1);
                        let true_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, true_bits, true_tag);
                    }
                    OpCode::TerminalClear { dst } => {
                        ctx.spill_all();
                        ctx.b.ins().call(symbols.xcx_jit_terminal_clear, &[]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        let b_val = ctx.b.ins().iconst(types::I64, 1);
                        let t_val = ctx.b.ins().iconst(types::I64, 2);
                        ctx.def_local(dst, b_val, t_val);
                    }
                    OpCode::TerminalRaw { dst } => {
                        ctx.spill_all();
                        ctx.b.ins().call(symbols.xcx_jit_terminal_raw, &[ctx.executor_ptr]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        let b_val = ctx.b.ins().iconst(types::I64, 1);
                        let t_val = ctx.b.ins().iconst(types::I64, 2);
                        ctx.def_local(dst, b_val, t_val);
                    }
                    OpCode::TerminalNormal { dst } => {
                        ctx.spill_all();
                        ctx.b.ins().call(symbols.xcx_jit_terminal_normal, &[ctx.executor_ptr]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        let b_val = ctx.b.ins().iconst(types::I64, 1);
                        let t_val = ctx.b.ins().iconst(types::I64, 2);
                        ctx.def_local(dst, b_val, t_val);
                    }
                    OpCode::TerminalCursor { dst, on } => {
                        ctx.spill_all();
                        let on_val = ctx.b.ins().iconst(types::I8, on as i64);
                        ctx.b.ins().call(symbols.xcx_jit_terminal_cursor, &[on_val]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        let b_val = ctx.b.ins().iconst(types::I64, 1);
                        let t_val = ctx.b.ins().iconst(types::I64, 2);
                        ctx.def_local(dst, b_val, t_val);
                    }
                    OpCode::TerminalMove { dst, x_src, y_src } => {
                        ctx.spill_all();
                        let (x_bits, x_tag) = ctx.use_local(x_src);
                        let (y_bits, y_tag) = ctx.use_local(y_src);
                        ctx.b.ins().call(symbols.xcx_jit_terminal_move, &[x_bits, x_tag, y_bits, y_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        let b_val = ctx.b.ins().iconst(types::I64, 1);
                        let t_val = ctx.b.ins().iconst(types::I64, 2);
                        ctx.def_local(dst, b_val, t_val);
                    }
                    OpCode::TerminalExit { dst } => {
                        ctx.spill_all();
                        ctx.b.ins().call(symbols.xcx_jit_terminal_exit, &[]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        let b_val = ctx.b.ins().iconst(types::I64, 1);
                        let t_val = ctx.b.ins().iconst(types::I64, 2);
                        ctx.def_local(dst, b_val, t_val);
                    }
                    OpCode::TerminalRun { dst, cmd_src } => {
                        ctx.spill_all();
                        let (c_bits, c_tag) = ctx.use_local(cmd_src);
                        let out_slot = ctx.b.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                            16,
                            4,
                        ));
                        let out_ptr = ctx.b.ins().stack_addr(types::I64, out_slot, 0);

                        ctx.b.ins().call(symbols.xcx_jit_terminal_run, &[
                            out_ptr,
                            c_bits,
                            c_tag,
                        ]);

                        let res_bits = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 0);
                        let res_tag  = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 8);

                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    _ => {
                        // For Phase 3, we abort if we hit unsupported opcode in method JIT
                        return Err(format!("Unsupported opcode for compile_method: {:?}", op));
                    }
                }
            }

            let end_ip = chunk.bytecode.len();
            if let Some(&block) = blocks.get(&end_ip) {
                if !terminated {
                    ctx.sync_for_jump();
                    ctx.b.ins().jump(block, &[]);
                }
                ctx.b.switch_to_block(block);
                ctx.clear_block_state(false);
                terminated = false;
            }

            if !terminated {
                ctx.spill_all();
                if is_inner_func {
                    let false_bits = ctx.b.ins().iconst(types::I64, 0);
                    let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                    let status = ctx.b.ins().iconst(types::I32, 0);
                    ctx.b.ins().store(super::abi::trusted(), false_bits, out_ptr, 0);
                    ctx.b.ins().store(super::abi::trusted(), false_tag, out_ptr, 8);
                    ctx.b.ins().return_(&[status]);
                } else {
                    let res_bits = ctx.b.ins().iconst(types::I64, 0);
                    let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL (false)
                    ctx.b.ins().store(super::abi::trusted(), res_bits, out_ptr, 0);
                    ctx.b.ins().store(super::abi::trusted(), res_tag, out_ptr, 8);
                    let rv = ctx.b.ins().iconst(types::I32, 0);
                    ctx.b.ins().return_(&[rv]);
                }
            }

            // Seal all blocks at the end
            for (_, &block) in &blocks {
                ctx.b.seal_block(block);
            }
            for &block in &ctx.created_blocks {
                ctx.b.seal_block(block);
            }
        }
        b.finalize();
        // XCX_JIT_DEBUG removed

        if let Err(errors) = self.module.define_function(func_id, &mut self.ctx) {
            let _ = errors;
            return Err(format!("{:#?}", errors));
        }
        self.module.clear_context(&mut self.ctx);

        if is_inner_func {
            Ok(std::ptr::null())
        } else {
            if let Err(e) = self.module.finalize_definitions() {
                eprintln!("[JIT] Error finalizing definitions: {:?}", e);
                return Err(e.to_string());
            }
            
            
            
            Ok(self.module.get_finalized_function(func_id) as *const std::ffi::c_void)
        }
    }

    fn compile_outer_wrapper(
        &mut self,
        func_id_idx: usize,
        _self_func_idx: u32,
        chunk: &Chunk,
        inner_func_id: cranelift_module::FuncId,
    ) -> Result<*const std::ffi::c_void, String> {
        self.module.clear_context(&mut self.ctx);
        
        let arity = chunk.arity;
        let mut sig = self.module.make_signature();
        let ptr_type = self.module.target_config().pointer_type();
        sig.params.push(AbiParam::new(ptr_type)); // out_ptr
        sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
        sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
        sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
        sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
        sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
        sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
        sig.returns.push(AbiParam::new(types::I32)); // status

        let func_id = self.module.declare_function(
            &format!("method_{}", func_id_idx),
            Linkage::Export,
            &sig,
        ).map_err(|e| e.to_string())?;

        self.ctx.func.signature = sig;
        
        let inner_ref = self.module.declare_func_in_func(inner_func_id, &mut self.ctx.func);

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = b.create_block();
        b.append_block_params_for_function_params(entry_block);
        b.switch_to_block(entry_block);

        let arg_vals = b.block_params(entry_block).to_vec();
        let out_ptr      = arg_vals[0];
        let locals_ptr   = arg_vals[1];
        let globals_ptr  = arg_vals[2];
        let consts_ptr   = arg_vals[3];
        let vm_ptr       = arg_vals[4];
        let executor_ptr = arg_vals[5];
        let shutdown_ptr = arg_vals[6];

        let mut call_args = Vec::new();
        for i in 0..arity {
            let offset = (i as i32) * 16;
            let bits = b.ins().load(types::I64, super::abi::trusted(), locals_ptr, offset);
            let tag  = b.ins().load(types::I64, super::abi::trusted(), locals_ptr, offset + 8);
            call_args.push(bits);
            call_args.push(tag);
        }

        call_args.push(out_ptr);
        call_args.push(locals_ptr);
        call_args.push(globals_ptr);
        call_args.push(consts_ptr);
        call_args.push(vm_ptr);
        call_args.push(executor_ptr);
        call_args.push(shutdown_ptr);

        let inst = b.ins().call(inner_ref, &call_args);
        let status = b.func.dfg.inst_results(inst)[0];

        b.ins().return_(&[status]);

        b.seal_block(entry_block);
        b.finalize();

        if let Err(e) = self.module.define_function(func_id, &mut self.ctx) {
            eprintln!("[JIT] Error defining wrapper function: {:?}", e);
            return Err(e.to_string());
        }
        self.module.clear_context(&mut self.ctx);
        if let Err(e) = self.module.finalize_definitions() {
            eprintln!("[JIT] Error finalizing wrapper definitions: {:?}", e);
            return Err(e.to_string());
        }
        

        
        Ok(self.module.get_finalized_function(func_id) as *const std::ffi::c_void)
    }

    fn precompile_callees(
        &mut self,
        caller_func_idx: u32,
        chunk: &Chunk,
        constants: &[VMValue],
        functions: &[std::sync::Arc<Chunk>],
    ) {
        for op in chunk.bytecode.iter() {
            if let OpCode::Call { func_idx, .. } = *op {
                let fi = func_idx as usize;
                if fi < functions.len() {
                    let callee = &functions[fi];
                    if callee.jit_ptr.load(std::sync::atomic::Ordering::Acquire).is_null() && fi != caller_func_idx as usize {
                        let callee_id_idx = callee.bytecode.as_ptr() as usize;
                        let name = callee.name.clone();
                        match self.compile_method(callee_id_idx, fi as u32, callee, constants, functions, &name) {
                            Ok(ptr) if !ptr.is_null() => {
                                callee.jit_ptr.store(ptr as *mut std::ffi::c_void, std::sync::atomic::Ordering::Release);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    pub fn compile_method(
        &mut self,
        func_id_idx: usize,
        self_func_idx: u32,
        chunk: &Chunk,
        constants: &[VMValue],
        functions: &[std::sync::Arc<Chunk>],
        name: &str,
    ) -> Result<*const std::ffi::c_void, String> {
        if !self.in_progress.insert(func_id_idx) {
            return Ok(std::ptr::null());
        }

        self.precompile_callees(self_func_idx, chunk, constants, functions);

        let arity = chunk.arity;
        let res = if self_func_idx == u32::MAX {
            self.compile_method_impl(func_id_idx, self_func_idx, chunk, constants, functions, name, false, None)
        } else {
            let ptr_type = self.module.target_config().pointer_type();
            let mut inner_sig = self.module.make_signature();
            for _ in 0..arity {
                inner_sig.params.push(AbiParam::new(types::I64)); // bits
                inner_sig.params.push(AbiParam::new(types::I64)); // tag
            }
            inner_sig.params.push(AbiParam::new(ptr_type)); // out_ptr
            inner_sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
            inner_sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
            inner_sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
            inner_sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
            inner_sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
            inner_sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
            inner_sig.returns.push(AbiParam::new(types::I32)); // status

            let inner_func_id = self.module.declare_function(
                &format!("method_inner_{}", func_id_idx),
                Linkage::Local,
                &inner_sig,
            ).map_err(|e| e.to_string())?;

            self.compile_method_impl(func_id_idx, self_func_idx, chunk, constants, functions, name, true, Some(inner_func_id))?;
            self.compile_outer_wrapper(func_id_idx, self_func_idx, chunk, inner_func_id)
        };

        self.in_progress.remove(&func_id_idx);
        res
    }
}
