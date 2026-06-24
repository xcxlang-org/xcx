use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

use crate::vm::opcode::{OpCode, Chunk, TypeTag};
use crate::vm::value::Value as VMValue;

use super::codegen_ctx::CodegenCtx;
use super::type_inference::analyze_chunk_types;
use super::analysis::*;
use super::emit_arith::*;
use super::emit_control::*;
use super::emit_load_store::*;
use super::emit_call::*;
use super::jit::JIT;

impl JIT {
    pub fn compile_fiber_segment(&mut self, func_id_idx: usize, start_ip: usize, chunk: &Chunk, constants: &[VMValue]) -> Result<*const u8, String> {
        self.module.clear_context(&mut self.ctx);

        let mut sig = self.module.make_signature();
        let ptr_type = self.module.target_config().pointer_type();
        sig.params.push(AbiParam::new(ptr_type)); // out_ptr
        sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
        sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
        sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
        sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
        sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
        sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
        let func_id = self.module.declare_function(
            &format!("fiber_seg_{}_{}", func_id_idx, start_ip),
            Linkage::Export,
            &sig,
        ).map_err(|e: cranelift_module::ModuleError| e.to_string())?;

        self.ctx.func.signature = sig;
        
        let symbols = self.symbols.import_in_func(&mut self.module, &mut self.ctx.func);

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let mut blocks: HashMap<usize, Block> = HashMap::new();
        
        let entry_block = b.create_block();
        blocks.insert(start_ip, entry_block);

        for (_ip, op) in chunk.bytecode.iter().enumerate().skip(start_ip) {
            match op {
                OpCode::Jump { target } => { blocks.entry(*target as usize).or_insert_with(|| b.create_block()); }
                OpCode::JumpIfFalse { target, .. } | OpCode::JumpIfTrue { target, .. } => {
                    blocks.entry(*target as usize).or_insert_with(|| b.create_block());
                }
                OpCode::Yield {..} | OpCode::YieldWithTarget {..} | OpCode::YieldVoid | OpCode::Return {..} | OpCode::ReturnVoid => {
                    break;
                }
                _ => {}
            }
        }

        b.append_block_params_for_function_params(entry_block);
        b.switch_to_block(entry_block);

        let out_ptr     = b.block_params(entry_block)[0];
        let locals_ptr  = b.block_params(entry_block)[1];
        let globals_ptr = b.block_params(entry_block)[2];
        let consts_ptr  = b.block_params(entry_block)[3];
        let vm_ptr      = b.block_params(entry_block)[4];
        let exec_ptr     = b.block_params(entry_block)[5];
        let shutdown_ptr = b.block_params(entry_block)[6];

        {
            let (call_depth_offset, stack_ptr_offset) = super::codegen_ctx::executor_field_offsets();

            let mut ctx = CodegenCtx::new(&mut b, out_ptr, locals_ptr, globals_ptr, consts_ptr, vm_ptr, exec_ptr, shutdown_ptr, start_ip, chunk.max_locals, blocks.clone(), u32::MAX, None, call_depth_offset, stack_ptr_offset);
            
            // Phase 4: Analyze used locals and preload them
            let used_locals = analyze_chunk_locals(&chunk.bytecode);
            let bool_array_hints_loop = analyze_bool_array_regs(&chunk.bytecode, constants);
            let (inferred_types, uses_heap) = analyze_chunk_types(&chunk.bytecode, constants, None, chunk.arity, u32::MAX, &bool_array_hints_loop);
            let global_ints = analyze_global_int_regs(&chunk.bytecode, constants);
            ctx.set_global_int_regs(global_ints.clone());
            let non_ptr_regs = analyze_non_ptr_regs(&chunk.bytecode, chunk.arity, &global_ints, constants);
            ctx.set_non_ptr_regs(non_ptr_regs);
            let may_contain_ptr = analyze_maybe_ptr_regs(&chunk.bytecode, &global_ints, constants);
            ctx.set_may_contain_ptr(may_contain_ptr);
            ctx.set_reg_types_per_ip(inferred_types);
            ctx.uses_heap = uses_heap;

            // Jump from entry_block to block_0
            let block_0 = blocks.get(&start_ip).unwrap();
            ctx.b.ins().jump(*block_0, &[]);
            // ctx.b.seal_block(entry_block); // Seal at the end instead
            ctx.b.switch_to_block(*block_0);

            ctx.preload_locals(&used_locals);

            let mut terminated = false;

            for (ip, op) in chunk.bytecode.iter().enumerate().skip(start_ip) {
                if let Some(&block) = blocks.get(&ip) {
                    if ip > start_ip {
                        if !terminated { ctx.b.ins().jump(block, &[]); }
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
                    }
                    OpCode::Move { dst, src } => {
                        emit_move(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::Add { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_add_int(&mut ctx, &symbols, dst, src1, src2);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                    OpCode::GetVar { dst, idx } => {
                        emit_get_var(&mut ctx, &symbols, dst, idx);
                    }
                    OpCode::SetVar { idx, src } => {
                        emit_set_var(&mut ctx, &symbols, idx, src);
                    }
                    OpCode::Equal { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 0);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
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
                    OpCode::Jump { target } => {
                        let target_blk = blocks[&(target as usize)];
                        ctx.b.ins().jump(target_blk, &[]);
                        terminated = true;
                    }
                    OpCode::JumpIfFalse { src, target } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                        let is_bool = ctx.b.ins().icmp(IntCC::Equal, sv_tag, bool_tag);
                        let is_zero = ctx.b.ins().icmp_imm(IntCC::Equal, sv_bits, 0);
                        let should_jump = ctx.b.ins().band(is_bool, is_zero);

                        let target_blk = blocks[&(target as usize)];
                        let next_blk = ctx.create_block();
                        ctx.spill_all();
                        ctx.b.ins().brif(should_jump, target_blk, &[], next_blk, &[]);
                        ctx.b.switch_to_block(next_blk);
                        ctx.clear_block_state(false);
                    }
                    OpCode::JumpIfTrue { src, target } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                        let is_bool = ctx.b.ins().icmp(IntCC::Equal, sv_tag, bool_tag);
                        let is_nonzero = ctx.b.ins().icmp_imm(IntCC::NotEqual, sv_bits, 0);
                        let should_jump = ctx.b.ins().band(is_bool, is_nonzero);

                        let target_blk = blocks[&(target as usize)];
                        let next_blk = ctx.create_block();
                        ctx.spill_all();
                        ctx.b.ins().brif(should_jump, target_blk, &[], next_blk, &[]);
                        ctx.b.switch_to_block(next_blk);
                        ctx.clear_block_state(false);
                    }
                    OpCode::Call { dst, func_idx, base, arg_count } => {
                        emit_call(&mut ctx, &symbols, dst, func_idx, base, arg_count);
                    }
                    OpCode::Yield { src } => {
                        emit_yield(&mut ctx, &symbols, src, ip + 1);
                        terminated = true;
                    }
                    OpCode::YieldWithTarget { dst, src } => {
                        emit_move(&mut ctx, &symbols, dst, src);
                        emit_yield(&mut ctx, &symbols, src, ip + 1);
                        terminated = true;
                    }
                    OpCode::YieldVoid => {
                        emit_yield_void(&mut ctx, &symbols, ip + 1);
                        terminated = true;
                    }
                    OpCode::Return { src } => {
                        emit_return_fiber(&mut ctx, &symbols, Some(src));
                        terminated = true;
                    }
                    OpCode::ReturnVoid => {
                        emit_return_fiber(&mut ctx, &symbols, None);
                        terminated = true;
                    }
                    OpCode::Wait { src } => {
                        let (v_bits, _v_tag) = ctx.use_local(src);
                        ctx.b.ins().call(symbols.xcx_jit_wait, &[v_bits]);
                    }
                    OpCode::TerminalWrite { dst, src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        ctx.b.ins().call(symbols.xcx_jit_terminal_write, &[v_bits, v_tag]);
                        let true_bits = ctx.b.ins().iconst(types::I64, 1);
                        let true_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            ctx.b.ins().call(symbols.xcx_jit_dec_ref, &[old_bits, old_tag]);
                        }
                        ctx.def_local(dst, true_bits, true_tag);
                    }
                    OpCode::MethodCall { dst, kind, base, arg_count } => {
                        emit_method_call(&mut ctx, &symbols, dst, kind, base, arg_count);
                    }
                    OpCode::MethodCallNamed { dst, kind, base, arg_count, names_idx } => {
                        emit_method_call_named(&mut ctx, &symbols, dst, kind, base, arg_count, names_idx, constants);
                    }
                    _ => {
                        return Err(format!("Unsupported opcode for compile_fiber_segment: {:?}", op));
                    }
                }
            }
            if !terminated {
                ctx.cleanup_all(&symbols, None);
                ctx.spill_all();
                let false_bits = ctx.b.ins().iconst(types::I64, 0);
                let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                ctx.b.ins().store(MemFlags::trusted(), false_bits, out_ptr, 0);
                ctx.b.ins().store(MemFlags::trusted(), false_tag, out_ptr, 8);
                ctx.b.ins().return_(&[]);
            }

            ctx.b.seal_block(entry_block);
            for (_, &block) in &blocks { ctx.b.seal_block(block); }
            for &block in &ctx.created_blocks { ctx.b.seal_block(block); }
        }
        b.finalize();

        self.module.define_function(func_id, &mut self.ctx).map_err(|e| format!("{:#?}", e))?;
        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().map_err(|e: cranelift_module::ModuleError| e.to_string())?;
        Ok(self.module.get_finalized_function(func_id))
    }
}
