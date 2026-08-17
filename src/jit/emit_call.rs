use cranelift::prelude::*;
use super::codegen_ctx::CodegenCtx;
use super::symbols::ImportedSymbols;
use super::nan_ops::*;

use crate::vm::opcode::MethodKind;

pub fn emit_call(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    func_idx: u32,
    base: u8,
    arg_count: u8,
) {
    if func_idx == ctx.self_func_idx && ctx.self_func_ref.is_some() {
        if ctx.is_inner_func {
            let offset = (ctx.max_locals as i64) * 16;
            let new_locals_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);

            if ctx.uses_heap {
                for i in 0..ctx.max_locals {
                    let addr = ctx.b.ins().iadd_imm(new_locals_ptr, (i as i64) * 16);
                    let false_bits = ctx.b.ins().iconst(types::I64, 0);
                    let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                    ctx.b.ins().store(MemFlags::trusted(), false_bits, addr, 0);
                    ctx.b.ins().store(MemFlags::trusted(), false_tag, addr, 8);
                }
            }

            let mut call_args = Vec::new();
            for i in 0..arg_count as usize {
                let arg_reg = base + i as u8;
                let (a_bits, a_tag) = ctx.use_local(arg_reg);
                if ctx.uses_heap {
                    emit_conditional_inc_ref(ctx, symbols, a_bits, a_tag);
                }
                call_args.push(a_bits);
                call_args.push(a_tag);
            }

            let out_slot = ctx.b.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                16,
                4,
            ));
            let out_ptr = ctx.b.ins().stack_addr(cranelift_codegen::ir::types::I64, out_slot, 0);

            call_args.push(out_ptr);
            call_args.push(new_locals_ptr);
            call_args.push(ctx.globals_ptr);
            call_args.push(ctx.consts_ptr);
            call_args.push(ctx.vm_ptr);
            call_args.push(ctx.executor_ptr);
            call_args.push(ctx.shutdown_ptr);

            // --- Recursion guard ---
            let cur_depth = ctx.b.ins().load(types::I64, MemFlags::trusted(), ctx.executor_ptr, ctx.call_depth_offset as i32);
            let limit = ctx.b.ins().iconst(types::I64, 800);
            let is_overflow = ctx.b.ins().icmp(IntCC::SignedGreaterThanOrEqual, cur_depth, limit);
            
            let overflow_blk = ctx.create_block();
            let run_blk = ctx.create_block();
            ctx.b.ins().brif(is_overflow, overflow_blk, &[], run_blk, &[]);

            ctx.b.switch_to_block(overflow_blk);
            let check_call = ctx.b.ins().call(symbols.xcx_jit_check_recursion, &[ctx.executor_ptr]);
            let halt_status = ctx.b.inst_results(check_call)[0];
            let err_val = ctx.b.ins().iconst(types::I64, 0);
            let err_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.b.ins().store(MemFlags::trusted(), err_val, ctx.out_ptr, 0);
            ctx.b.ins().store(MemFlags::trusted(), err_tag, ctx.out_ptr, 8);
            ctx.b.ins().return_(&[halt_status]);

            ctx.b.switch_to_block(run_blk);
            let new_depth = ctx.b.ins().iadd_imm(cur_depth, 1);
            ctx.b.ins().store(MemFlags::trusted(), new_depth, ctx.executor_ptr, ctx.call_depth_offset as i32);

            let self_ref = ctx.self_func_ref.unwrap();
            let inst = ctx.b.ins().call(self_ref, &call_args);
            ctx.b.ins().store(MemFlags::trusted(), cur_depth, ctx.executor_ptr, ctx.call_depth_offset as i32);

            if ctx.uses_heap {
                ctx.reload_globals();
            }

            let status = ctx.b.func.dfg.inst_results(inst)[0];
            let res_bits = ctx.b.ins().load(cranelift_codegen::ir::types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 0);
            let res_tag  = ctx.b.ins().load(cranelift_codegen::ir::types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 8);

            let halt_blk = ctx.create_block();
            let next_blk = ctx.create_block();
            let zero = ctx.b.ins().iconst(types::I32, 0);
            let is_halt = ctx.b.ins().icmp(IntCC::NotEqual, status, zero);
            ctx.b.ins().brif(is_halt, halt_blk, &[], next_blk, &[]);

            ctx.b.switch_to_block(halt_blk);
            ctx.b.ins().store(cranelift_codegen::ir::MemFlags::trusted(), res_bits, ctx.out_ptr, 0);
            ctx.b.ins().store(cranelift_codegen::ir::MemFlags::trusted(), res_tag, ctx.out_ptr, 8);
            ctx.b.ins().return_(&[status]);

            ctx.b.switch_to_block(next_blk);

            if ctx.uses_heap {
                emit_conditional_inc_ref(ctx, symbols, res_bits, res_tag);
            }

            if !ctx.should_skip_dec_ref(dst) {
                let (old_bits, old_tag) = ctx.use_local(dst);
                emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
            }
            ctx.def_local(dst, res_bits, res_tag);
            return;
        }

        ctx.spill_all();

        let offset = (ctx.max_locals as i64) * 16;
        let new_locals_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);

        if ctx.uses_heap {
            for i in 0..ctx.max_locals {
                let addr = ctx.b.ins().iadd_imm(new_locals_ptr, (i as i64) * 16);
                let false_bits = ctx.b.ins().iconst(types::I64, 0);
                let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                ctx.b.ins().store(MemFlags::trusted(), false_bits, addr, 0);
                ctx.b.ins().store(MemFlags::trusted(), false_tag, addr, 8);
            }
        }

        let mut arg_vals = Vec::new();
        for i in 0..arg_count as usize {
            let arg_reg = base + i as u8;
            let (a_bits, a_tag) = ctx.use_local(arg_reg);
            if ctx.uses_heap {
                emit_conditional_inc_ref(ctx, symbols, a_bits, a_tag);
            }
            arg_vals.push((a_bits, a_tag));
        }

        for (i, (bits, tag)) in arg_vals.iter().enumerate() {
            let offset_bytes = (i as i32) * 16;
            ctx.b.ins().store(MemFlags::trusted(), *bits, new_locals_ptr, offset_bytes);
            ctx.b.ins().store(MemFlags::trusted(), *tag,  new_locals_ptr, offset_bytes + 8);
        }

        let out_slot = ctx.b.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            16,
            4,
        ));
        let out_ptr = ctx.b.ins().stack_addr(cranelift_codegen::ir::types::I64, out_slot, 0);

        let mut call_args = Vec::new();
        for (bits, tag) in arg_vals.iter() {
            call_args.push(*bits);
            call_args.push(*tag);
        }
        call_args.push(out_ptr);
        call_args.push(new_locals_ptr);
        call_args.push(ctx.globals_ptr);
        call_args.push(ctx.consts_ptr);
        call_args.push(ctx.vm_ptr);
        call_args.push(ctx.executor_ptr);
        call_args.push(ctx.shutdown_ptr);

        // --- Recursion guard ---
        let cur_depth = ctx.b.ins().load(types::I64, MemFlags::trusted(), ctx.executor_ptr, ctx.call_depth_offset as i32);
        let limit = ctx.b.ins().iconst(types::I64, 800);
        let is_overflow = ctx.b.ins().icmp(IntCC::SignedGreaterThanOrEqual, cur_depth, limit);

        let overflow_blk = ctx.create_block();
        let run_blk = ctx.create_block();
        ctx.b.ins().brif(is_overflow, overflow_blk, &[], run_blk, &[]);

        ctx.b.switch_to_block(overflow_blk);
        let check_call = ctx.b.ins().call(symbols.xcx_jit_check_recursion, &[ctx.executor_ptr]);
        let halt_status = ctx.b.inst_results(check_call)[0];
        let err_val = ctx.b.ins().iconst(types::I64, 0);
        let err_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
        ctx.b.ins().store(MemFlags::trusted(), err_val, ctx.out_ptr, 0);
        ctx.b.ins().store(MemFlags::trusted(), err_tag, ctx.out_ptr, 8);
        ctx.b.ins().return_(&[halt_status]);

        ctx.b.switch_to_block(run_blk);
        let new_depth = ctx.b.ins().iadd_imm(cur_depth, 1);
        ctx.b.ins().store(MemFlags::trusted(), new_depth, ctx.executor_ptr, ctx.call_depth_offset as i32);

        let join_blk = ctx.create_block();
        let self_ref = ctx.self_func_ref.unwrap();
        let inst = ctx.b.ins().call(self_ref, &call_args);
        ctx.b.ins().store(MemFlags::trusted(), cur_depth, ctx.executor_ptr, ctx.call_depth_offset as i32);
        if ctx.uses_heap {
            ctx.reload_globals();
        }

        let _status = ctx.b.func.dfg.inst_results(inst)[0];
        ctx.b.ins().jump(join_blk, &[]);
        
        ctx.b.switch_to_block(join_blk);
        let res_bits = ctx.b.ins().load(cranelift_codegen::ir::types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 0);
        let res_tag  = ctx.b.ins().load(cranelift_codegen::ir::types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 8);

        if ctx.uses_heap {
            emit_conditional_inc_ref(ctx, symbols, res_bits, res_tag);
        }

        if !ctx.should_skip_dec_ref(dst) {
            let (old_bits, old_tag) = ctx.use_local(dst);
            emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
        }
        ctx.def_local(dst, res_bits, res_tag);
    } else if let Some(funcs) = ctx.functions {
        let callee_chunk = &funcs[func_idx as usize];
        let callee_uses_heap = callee_chunk.uses_heap.load(std::sync::atomic::Ordering::Relaxed);
        let callee_jit_ptr_addr = {
            let atom_ref = &*callee_chunk.jit_ptr;
            atom_ref as *const std::sync::atomic::AtomicPtr<std::ffi::c_void> as usize
        };

        let slot_data = cranelift_codegen::ir::StackSlotData::new(cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 16, 8);
        let slot = ctx.b.create_sized_stack_slot(slot_data);
        let out_ptr = ctx.b.ins().stack_addr(types::I64, slot, 0);

        let addr_val = ctx.b.ins().iconst(types::I64, callee_jit_ptr_addr as i64);
        let callee_jit_fn = ctx.b.ins().load(types::I64, MemFlags::trusted(), addr_val, 0);

        let zero_ptr = ctx.b.ins().iconst(types::I64, 0);
        let is_null = ctx.b.ins().icmp(IntCC::Equal, callee_jit_fn, zero_ptr);

        let slow_blk = ctx.create_block();
        let fast_blk = ctx.create_block();
        let join_blk = ctx.create_block();

        let status_var = ctx.b.declare_var(types::I32);
        
        ctx.b.ins().brif(is_null, slow_blk, &[], fast_blk, &[]);

        // --- Slow Block (Fallback to xcx_jit_call_recursive) ---
        ctx.b.switch_to_block(slow_blk);
        let f_idx = ctx.b.ins().iconst(types::I64, func_idx as i64);
        let a_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, (base as i64) * 16);
        let a_cnt = ctx.b.ins().iconst(types::I8, arg_count as i64);

        let mut call_args = Vec::new();
        call_args.push(out_ptr);
        call_args.push(f_idx);
        call_args.push(a_ptr);
        call_args.push(a_cnt);
        call_args.push(ctx.executor_ptr);

        ctx.spill_all();
        let call_inst = ctx.b.ins().call(symbols.xcx_jit_call_recursive, &call_args);
        let slow_status = ctx.b.func.dfg.inst_results(call_inst)[0];
        ctx.b.def_var(status_var, slow_status);
        ctx.b.ins().jump(join_blk, &[]);

        // --- Fast Block (Direct JIT-to-JIT call) ---
        ctx.b.switch_to_block(fast_blk);
        
        let ptr_type = ctx.b.func.dfg.value_type(ctx.locals_ptr);
        let mut inner_sig = cranelift::prelude::Signature::new(ctx.b.func.signature.call_conv);
        inner_sig.params.push(AbiParam::new(ptr_type)); // out_ptr
        inner_sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
        inner_sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
        inner_sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
        inner_sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
        inner_sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
        inner_sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
        inner_sig.returns.push(AbiParam::new(types::I32)); // status
        
        let sig_ref = ctx.b.import_signature(inner_sig);

        let offset = (ctx.max_locals as i64) * 16;
        let new_locals_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);

        let callee_uses_heap = callee_uses_heap;
        if callee_uses_heap {
            for i in 0..callee_chunk.max_locals {
                let addr = ctx.b.ins().iadd_imm(new_locals_ptr, (i as i64) * 16);
                let false_bits = ctx.b.ins().iconst(types::I64, 0);
                let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                ctx.b.ins().store(MemFlags::trusted(), false_bits, addr, 0);
                ctx.b.ins().store(MemFlags::trusted(), false_tag, addr, 8);
            }
        }

        let mut arg_vals = Vec::new();
        for i in 0..arg_count as usize {
            let arg_reg = base + i as u8;
            let (a_bits, a_tag) = ctx.use_local(arg_reg);
            if ctx.uses_heap {
                emit_conditional_inc_ref(ctx, symbols, a_bits, a_tag);
            }
            arg_vals.push((a_bits, a_tag));
        }

        for (i, (bits, tag)) in arg_vals.iter().enumerate() {
            let offset_bytes = (i as i32) * 16;
            ctx.b.ins().store(MemFlags::trusted(), *bits, new_locals_ptr, offset_bytes);
            ctx.b.ins().store(MemFlags::trusted(), *tag,  new_locals_ptr, offset_bytes + 8);
        }

        // --- Recursion check ---
        let cur_depth = ctx.b.ins().load(types::I64, MemFlags::trusted(), ctx.executor_ptr, ctx.call_depth_offset as i32);
        let limit = ctx.b.ins().iconst(types::I64, 800);
        let is_overflow = ctx.b.ins().icmp(IntCC::SignedGreaterThanOrEqual, cur_depth, limit);

        let fast_halt_blk = ctx.create_block();
        let fast_run_blk = ctx.create_block();

        ctx.b.ins().brif(is_overflow, fast_halt_blk, &[], fast_run_blk, &[]);

        // - Fast Halt Block
        ctx.b.switch_to_block(fast_halt_blk);
        let check_call = ctx.b.ins().call(symbols.xcx_jit_check_recursion, &[ctx.executor_ptr]);
        let err_status = ctx.b.inst_results(check_call)[0];
        let err_val = ctx.b.ins().iconst(types::I64, 0);
        let err_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
        ctx.b.ins().store(MemFlags::trusted(), err_val, out_ptr, 0);
        ctx.b.ins().store(MemFlags::trusted(), err_tag, out_ptr, 8);
        ctx.b.def_var(status_var, err_status);
        ctx.b.ins().jump(join_blk, &[]);

        // - Fast Run Block
        ctx.b.switch_to_block(fast_run_blk);
        let new_depth = ctx.b.ins().iadd_imm(cur_depth, 1);
        ctx.b.ins().store(MemFlags::trusted(), new_depth, ctx.executor_ptr, ctx.call_depth_offset as i32);

        let cur_stack_ptr = ctx.b.ins().load(types::I64, MemFlags::trusted(), ctx.executor_ptr, ctx.stack_ptr_offset as i32);
        let new_stack_ptr = ctx.b.ins().iadd_imm(cur_stack_ptr, callee_chunk.max_locals as i64);
        ctx.b.ins().store(MemFlags::trusted(), new_stack_ptr, ctx.executor_ptr, ctx.stack_ptr_offset as i32);

        ctx.spill_all();

        let mut fast_call_args = Vec::new();
        fast_call_args.push(out_ptr);
        fast_call_args.push(new_locals_ptr);
        fast_call_args.push(ctx.globals_ptr);
        fast_call_args.push(ctx.consts_ptr);
        fast_call_args.push(ctx.vm_ptr);
        fast_call_args.push(ctx.executor_ptr);
        fast_call_args.push(ctx.shutdown_ptr);

        let call_inst = ctx.b.ins().call_indirect(sig_ref, callee_jit_fn, &fast_call_args);
        let fast_status = ctx.b.func.dfg.inst_results(call_inst)[0];

        ctx.b.ins().store(MemFlags::trusted(), cur_depth, ctx.executor_ptr, ctx.call_depth_offset as i32);
        ctx.b.ins().store(MemFlags::trusted(), cur_stack_ptr, ctx.executor_ptr, ctx.stack_ptr_offset as i32);

        ctx.b.def_var(status_var, fast_status);
        ctx.b.ins().jump(join_blk, &[]);

        // --- Join Block ---
        ctx.b.switch_to_block(join_blk);
        let status = ctx.b.use_var(status_var);

        let res_bits = ctx.b.ins().load(types::I64, MemFlags::trusted(), out_ptr, 0);
        let res_tag  = ctx.b.ins().load(types::I64, MemFlags::trusted(), out_ptr, 8);
        if callee_uses_heap {
            ctx.reload_globals();
        }

        if ctx.is_inner_func {
            let halt_blk = ctx.create_block();
            let next_blk = ctx.create_block();
            let zero = ctx.b.ins().iconst(types::I32, 0);
            let is_halt = ctx.b.ins().icmp(IntCC::NotEqual, status, zero);
            ctx.b.ins().brif(is_halt, halt_blk, &[], next_blk, &[]);

            ctx.b.switch_to_block(halt_blk);
            let ret_status = ctx.b.ins().iconst(types::I32, 1);
            ctx.b.ins().return_(&[ret_status]);

            ctx.b.switch_to_block(next_blk);
        }

        if !ctx.should_skip_dec_ref(dst) {
            let (old_bits, old_tag) = ctx.use_local(dst);
            emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
        }
        ctx.def_local(dst, res_bits, res_tag);
    } else {
        let f_idx = ctx.b.ins().iconst(types::I64, func_idx as i64);
        let a_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, (base as i64) * 16);
        let a_cnt = ctx.b.ins().iconst(types::I8, arg_count as i64);
        
        let slot_data = cranelift_codegen::ir::StackSlotData::new(cranelift_codegen::ir::StackSlotKind::ExplicitSlot, 16, 8);
        let slot = ctx.b.create_sized_stack_slot(slot_data);
        let out_ptr = ctx.b.ins().stack_addr(types::I64, slot, 0);

        let mut call_args = Vec::new();
        call_args.push(out_ptr);
        call_args.push(f_idx);
        call_args.push(a_ptr);
        call_args.push(a_cnt);
        call_args.push(ctx.executor_ptr);

        ctx.spill_all();
        let call_inst = ctx.b.ins().call(symbols.xcx_jit_call_recursive, &call_args);
        let status = ctx.b.func.dfg.inst_results(call_inst)[0];
        
        let res_bits = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 0);
        let res_tag  = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), out_ptr, 8);
        ctx.reload_globals();

        if ctx.is_inner_func {
            let halt_blk = ctx.create_block();
            let next_blk = ctx.create_block();
            let zero = ctx.b.ins().iconst(types::I32, 0);
            let is_halt = ctx.b.ins().icmp(IntCC::NotEqual, status, zero);
            ctx.b.ins().brif(is_halt, halt_blk, &[], next_blk, &[]);

            ctx.b.switch_to_block(halt_blk);
            let ret_status = ctx.b.ins().iconst(types::I32, 1);
            ctx.b.ins().return_(&[ret_status]);

            ctx.b.switch_to_block(next_blk);
        }
        
        if !ctx.should_skip_dec_ref(dst) {
            let (old_bits, old_tag) = ctx.use_local(dst);
            emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
        }
        ctx.def_local(dst, res_bits, res_tag);
    }
}

pub fn emit_method_call(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    kind: MethodKind,
    base: u8,
    arg_count: u8,
) {
    let (recv_bits, recv_tag) = ctx.use_local(base);

    match kind {
        MethodKind::Get if arg_count == 1 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Json || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map) => {
            let (idx_bits, idx_tag) = ctx.use_local(base + 1);
            let kind_ty = ctx.get_reg_type(base as usize);
            // When dst == base and the receiver holds an inc-elided (un-owned)
            // copy of its global, the old-value dec_ref below would release a
            // reference that was never acquired — skip it.
            let recv_is_unowned = dst == base && ctx.unowned_recv_regs[base as usize];

            if kind_ty == crate::vm::opcode::TypeTag::Array {
                if ctx.get_reg_type(dst as usize) == crate::vm::opcode::TypeTag::Int {
                    let elements_ptr = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 16);
                    let len = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 24);
                    let is_in_bounds = ctx.b.ins().icmp(IntCC::UnsignedLessThan, idx_bits, len);
                    
                    let fast_blk = ctx.create_block();
                    let slow_blk = ctx.create_block();
                    let join_blk = ctx.create_block();
                    let res_val = ctx.b.declare_var(types::I64);
                    
                    ctx.b.ins().brif(is_in_bounds, fast_blk, &[], slow_blk, &[]);
                    
                    ctx.b.switch_to_block(fast_blk);
                    let idx_offset = ctx.b.ins().imul_imm(idx_bits, 16);
                    let elem_addr = ctx.b.ins().iadd(elements_ptr, idx_offset);
                    let raw_val = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), elem_addr, 0);
                    ctx.b.def_var(res_val, raw_val);
                    ctx.b.ins().jump(join_blk, &[]);
                    
                    ctx.b.switch_to_block(slow_blk);
                    let call = ctx.b.ins().call(symbols.xcx_jit_array_get_int, &[recv_bits, recv_tag, idx_bits]);
                    let r_bits = ctx.b.inst_results(call)[0];
                    ctx.b.def_var(res_val, r_bits);
                    ctx.b.ins().jump(join_blk, &[]);
                    
                    ctx.b.switch_to_block(join_blk);
                    ctx.clear_block_state(false);
                    let final_bits = ctx.b.use_var(res_val);
                    let r_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
                    ctx.def_local(dst, final_bits, r_tag);
                } else {
                    let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_get, &[recv_bits, recv_tag, idx_bits]);
                    if !recv_is_unowned && !ctx.should_skip_dec_ref(dst) {
                        let (old_bits, old_tag) = ctx.use_local(dst);
                        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
                    }
                    ctx.def_local(dst, r_bits, r_tag);
                }
            } else if kind_ty == crate::vm::opcode::TypeTag::BoolArray {
                let elements_ptr = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 16);
                let len = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 24);
                let is_in_bounds = ctx.b.ins().icmp(IntCC::UnsignedLessThan, idx_bits, len);
                
                let fast_blk = ctx.create_block();
                let slow_blk = ctx.create_block();
                let join_blk = ctx.create_block();
                let res_val = ctx.b.declare_var(types::I64);
                
                ctx.b.ins().brif(is_in_bounds, fast_blk, &[], slow_blk, &[]);
                
                ctx.b.switch_to_block(fast_blk);
                let elem_addr = ctx.b.ins().iadd(elements_ptr, idx_bits);
                let raw_val_i8 = ctx.b.ins().load(types::I8, cranelift_codegen::ir::MemFlags::trusted(), elem_addr, 0);
                let raw_val = ctx.b.ins().uextend(types::I64, raw_val_i8);
                let val_bool = ctx.b.ins().band_imm(raw_val, 1);
                ctx.b.def_var(res_val, val_bool);
                ctx.b.ins().jump(join_blk, &[]);
                
                ctx.b.switch_to_block(slow_blk);
                let call = ctx.b.ins().call(symbols.xcx_jit_array_get_bool, &[recv_bits, recv_tag, idx_bits]);
                let r_bits = ctx.b.inst_results(call)[0];
                ctx.b.def_var(res_val, r_bits);
                ctx.b.ins().jump(join_blk, &[]);
                
                ctx.b.switch_to_block(join_blk);
                ctx.clear_block_state(false);
                let final_bits = ctx.b.use_var(res_val);
                if !recv_is_unowned && !ctx.should_skip_dec_ref(dst) {
                    let (old_bits, old_tag) = ctx.use_local(dst);
                    emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
                }
                let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                ctx.def_local(dst, final_bits, bool_tag);
            } else if kind_ty == crate::vm::opcode::TypeTag::Json {
                let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_json_get, &[recv_bits, recv_tag, idx_bits, idx_tag]);
                if !recv_is_unowned && !ctx.should_skip_dec_ref(dst) {
                    let (old_bits, old_tag) = ctx.use_local(dst);
                    emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
                }
                ctx.def_local(dst, r_bits, r_tag);
            } else if kind_ty == crate::vm::opcode::TypeTag::Map {
                let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_map_get, &[recv_bits, recv_tag, idx_bits, idx_tag]);
                if !recv_is_unowned && !ctx.should_skip_dec_ref(dst) {
                    let (old_bits, old_tag) = ctx.use_local(dst);
                    emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
                }
                ctx.def_local(dst, r_bits, r_tag);
            } else {

                let k_val = ctx.b.ins().iconst(types::I32, kind as i64);
                let k_i8  = ctx.b.ins().ireduce(types::I8, k_val);
                let args_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, (base as i64 + 1) * 16);
                let a_cnt_i8 = ctx.b.ins().iconst(types::I8, 1);
                let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_method_dispatch, &[
                    recv_bits, recv_tag, k_i8, args_ptr, a_cnt_i8, ctx.executor_ptr
                ]);
                if !ctx.should_skip_dec_ref(dst) {
                    let (old_bits, old_tag) = ctx.use_local(dst);
                    emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
                }
                ctx.def_local(dst, r_bits, r_tag);
                ctx.emit_halt_if_errors(symbols);
            }
            return;
        }
        MethodKind::Update | MethodKind::Set if arg_count == 2 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Json || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map) => {
            let (idx_bits, idx_tag) = ctx.use_local(base + 1);
            let val_reg = base + 2;
            let kind_ty = ctx.get_reg_type(base as usize);

            if kind_ty == crate::vm::opcode::TypeTag::Array {
                if ctx.get_reg_type(val_reg as usize) == crate::vm::opcode::TypeTag::Bool {
                    let (val_bits, _val_tag) = ctx.use_local(val_reg);
                    let elements_ptr = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 16);
                    let len = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 24);
                    let is_in_bounds = ctx.b.ins().icmp(IntCC::UnsignedLessThan, idx_bits, len);
                    
                    let fast_blk = ctx.create_block();
                    let slow_blk = ctx.create_block();
                    let join_blk = ctx.create_block();
                    
                    ctx.b.ins().brif(is_in_bounds, fast_blk, &[], slow_blk, &[]);
                    
                    ctx.b.switch_to_block(fast_blk);
                    let idx_offset = ctx.b.ins().imul_imm(idx_bits, 16);
                    let elem_addr = ctx.b.ins().iadd(elements_ptr, idx_offset);
                    let val_norm = ctx.b.ins().band_imm(val_bits, 1);
                    let tag_val = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                    ctx.b.ins().store(cranelift_codegen::ir::MemFlags::trusted(), val_norm, elem_addr, 0);
                    ctx.b.ins().store(cranelift_codegen::ir::MemFlags::trusted(), tag_val, elem_addr, 8);
                    ctx.b.ins().jump(join_blk, &[]);
                    
                    ctx.b.switch_to_block(slow_blk);
                    let val_bool_i8 = ctx.b.ins().ireduce(types::I8, val_bits); 
                    ctx.b.ins().call(symbols.xcx_jit_array_set_bool, &[recv_bits, recv_tag, idx_bits, val_bool_i8]);
                    ctx.b.ins().jump(join_blk, &[]);
                    
                    ctx.b.switch_to_block(join_blk);
                    ctx.clear_block_state(false);
                } else if ctx.get_reg_type(val_reg as usize) == crate::vm::opcode::TypeTag::Int {
                    let (val_bits, _val_tag) = ctx.use_local(val_reg);
                    let elements_ptr = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 16);
                    let len = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 24);
                    let is_in_bounds = ctx.b.ins().icmp(IntCC::UnsignedLessThan, idx_bits, len);
                    
                    let fast_blk = ctx.create_block();
                    let slow_blk = ctx.create_block();
                    let join_blk = ctx.create_block();
                    
                    ctx.b.ins().brif(is_in_bounds, fast_blk, &[], slow_blk, &[]);
                    
                    ctx.b.switch_to_block(fast_blk);
                    let idx_offset = ctx.b.ins().imul_imm(idx_bits, 16);
                    let elem_addr = ctx.b.ins().iadd(elements_ptr, idx_offset);
                    let tag_val = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
                    ctx.b.ins().store(cranelift_codegen::ir::MemFlags::trusted(), val_bits, elem_addr, 0);
                    ctx.b.ins().store(cranelift_codegen::ir::MemFlags::trusted(), tag_val, elem_addr, 8);
                    ctx.b.ins().jump(join_blk, &[]);
                    
                    ctx.b.switch_to_block(slow_blk);
                    ctx.b.ins().call(symbols.xcx_jit_array_set_int, &[recv_bits, recv_tag, idx_bits, val_bits]);
                    ctx.b.ins().jump(join_blk, &[]);
                    
                    ctx.b.switch_to_block(join_blk);
                    ctx.clear_block_state(false);
                } else {
                    let (val_bits, val_tag) = ctx.use_local(val_reg);
                    ctx.b.ins().call(symbols.xcx_jit_array_update, &[recv_bits, recv_tag, idx_bits, val_bits, val_tag]);
                }
                let t_bits = ctx.b.ins().iconst(types::I64, 1);
                let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                ctx.def_local(dst, t_bits, t_tag);
            } else if kind_ty == crate::vm::opcode::TypeTag::BoolArray {
                let (val_bits, _val_tag) = ctx.use_local(val_reg);
                let unpacked_bool = if ctx.get_reg_type(val_reg as usize) == crate::vm::opcode::TypeTag::Bool {
                    val_bits
                } else {
                    super::nan_ops::unpack_bool(ctx.b, val_bits)
                };
                
                let elements_ptr = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 16);
                let len = ctx.b.ins().load(types::I64, cranelift_codegen::ir::MemFlags::trusted(), recv_bits, 24);
                let is_in_bounds = ctx.b.ins().icmp(IntCC::UnsignedLessThan, idx_bits, len);
                
                let fast_blk = ctx.create_block();
                let slow_blk = ctx.create_block();
                let join_blk = ctx.create_block();
                
                ctx.b.ins().brif(is_in_bounds, fast_blk, &[], slow_blk, &[]);
                
                ctx.b.switch_to_block(fast_blk);
                let elem_addr = ctx.b.ins().iadd(elements_ptr, idx_bits);
                let val_norm = ctx.b.ins().band_imm(unpacked_bool, 1);
                let val_bool_i8 = ctx.b.ins().ireduce(types::I8, val_norm);
                ctx.b.ins().store(cranelift_codegen::ir::MemFlags::trusted(), val_bool_i8, elem_addr, 0);
                ctx.b.ins().jump(join_blk, &[]);
                
                ctx.b.switch_to_block(slow_blk);
                let val_bool_i8_slow = ctx.b.ins().ireduce(types::I8, unpacked_bool);
                ctx.b.ins().call(symbols.xcx_jit_array_set_bool, &[recv_bits, recv_tag, idx_bits, val_bool_i8_slow]);
                ctx.b.ins().jump(join_blk, &[]);
                
                ctx.b.switch_to_block(join_blk);
                ctx.clear_block_state(false);
                
                let t_bits = ctx.b.ins().iconst(types::I64, 1);
                let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                ctx.def_local(dst, t_bits, t_tag);
            } else if kind_ty == crate::vm::opcode::TypeTag::Json {
                let (v_bits, v_tag) = ctx.use_local(base + 2);
                ctx.b.ins().call(symbols.xcx_jit_json_set, &[recv_bits, recv_tag, idx_bits, idx_tag, v_bits, v_tag]);
                
                let t_bits = ctx.b.ins().iconst(types::I64, 1);
                let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                ctx.def_local(dst, t_bits, t_tag);
            } else if kind_ty == crate::vm::opcode::TypeTag::Map {
                let (v_bits, v_tag) = ctx.use_local(base + 2);
                ctx.b.ins().call(symbols.xcx_jit_map_insert, &[recv_bits, recv_tag, idx_bits, idx_tag, v_bits, v_tag]);
                
                let t_bits = ctx.b.ins().iconst(types::I64, 1);
                let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                ctx.def_local(dst, t_bits, t_tag);
            } else {

                let k_val = ctx.b.ins().iconst(types::I32, kind as i64);
                let k_i8  = ctx.b.ins().ireduce(types::I8, k_val);
                let args_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, (base as i64 + 1) * 16);
                let a_cnt_i8 = ctx.b.ins().iconst(types::I8, 2);
                ctx.spill_all();
                let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_method_dispatch, &[
                    recv_bits, recv_tag, k_i8, args_ptr, a_cnt_i8, ctx.executor_ptr
                ]);
                if !ctx.unowned_recv_regs[base as usize] && !ctx.should_skip_dec_ref(dst) {
                    let (old_bits, old_tag) = ctx.use_local(dst);
                    emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
                }
                ctx.def_local(dst, r_bits, r_tag);
            }
            return;
        }
        MethodKind::Push if arg_count == 1 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Json) => {
            let (v_bits, v_tag) = ctx.use_local(base + 1);
            if ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray {
                ctx.b.ins().call(symbols.xcx_jit_array_push, &[recv_bits, recv_tag, v_bits, v_tag]);
            } else {
                ctx.b.ins().call(symbols.xcx_jit_json_push, &[recv_bits, recv_tag, v_bits, v_tag]);
                ctx.emit_halt_if_errors(symbols);
            }
            let t_bits = ctx.b.ins().iconst(types::I64, 1);
            let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, t_bits, t_tag);
            return;
        }
        MethodKind::Pop if arg_count == 0 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_pop, &[recv_bits, recv_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Clear if arg_count == 0 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            ctx.b.ins().call(symbols.xcx_jit_array_clear, &[recv_bits, recv_tag]);
            let t_bits = ctx.b.ins().iconst(types::I64, 1);
            let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, t_bits, t_tag);
            return;
        }
        MethodKind::Run if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Fiber => {
            ctx.spill_all();
            let call = ctx.b.ins().call(symbols.xcx_jit_fiber_run, &[recv_bits, recv_tag, ctx.executor_ptr]);
            ctx.reload_globals();
            let r_bits_i8 = ctx.b.inst_results(call)[0];
            let r_bits = ctx.b.ins().uextend(types::I64, r_bits_i8);
            let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, r_bits, bool_tag);
            return;
        }
        MethodKind::IsDone if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Fiber => {
            let call = ctx.b.ins().call(symbols.xcx_jit_fiber_is_done, &[recv_bits, recv_tag]);
            let r_bits_i8 = ctx.b.inst_results(call)[0];
            let r_bits = ctx.b.ins().uextend(types::I64, r_bits_i8);
            let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, r_bits, bool_tag);
            return;
        }
        MethodKind::IsEmpty if arg_count == 0 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            let call = ctx.b.ins().call(symbols.xcx_jit_array_is_empty, &[recv_bits, recv_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, r_bits, bool_tag);
            return;
        }
        MethodKind::Contains if arg_count == 1 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            let (v_bits, v_tag) = ctx.use_local(base + 1);
            let call = ctx.b.ins().call(symbols.xcx_jit_array_contains, &[recv_bits, recv_tag, v_bits, v_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, r_bits, bool_tag);
            return;
        }
        MethodKind::Find if arg_count == 1 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            let (v_bits, v_tag) = ctx.use_local(base + 1);
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_find, &[recv_bits, recv_tag, v_bits, v_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Insert if arg_count == 2 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            let (idx_bits, _idx_tag) = ctx.use_local(base + 1);
            let (v_bits, v_tag) = ctx.use_local(base + 2);
            ctx.b.ins().call(symbols.xcx_jit_array_insert, &[recv_bits, recv_tag, idx_bits, v_bits, v_tag]);
            let t_bits = ctx.b.ins().iconst(types::I64, 1);
            let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, t_bits, t_tag);
            return;
        }
        MethodKind::Delete | MethodKind::Remove if arg_count == 1 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            let (idx_bits, _idx_tag) = ctx.use_local(base + 1);
            ctx.b.ins().call(symbols.xcx_jit_array_delete, &[recv_bits, recv_tag, idx_bits]);
            let t_bits = ctx.b.ins().iconst(types::I64, 1);
            let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, t_bits, t_tag);
            return;
        }
        MethodKind::Sort if arg_count == 0 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            ctx.b.ins().call(symbols.xcx_jit_array_sort, &[recv_bits, recv_tag]);
            let t_bits = ctx.b.ins().iconst(types::I64, 1);
            let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, t_bits, t_tag);
            return;
        }
        MethodKind::Reverse if arg_count == 0 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray) => {
            ctx.b.ins().call(symbols.xcx_jit_array_reverse, &[recv_bits, recv_tag]);
            let t_bits = ctx.b.ins().iconst(types::I64, 1);
            let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, t_bits, t_tag);
            return;
        }
        MethodKind::Size | MethodKind::Len | MethodKind::Count if arg_count == 0 && (ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Json || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map) => {
            let (r_bits, r_tag) = if ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Array || ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::BoolArray {
                // Direct layout load: Vec len is at offset 24 from RwLock<ArrayObj> pointer.
                let s_bits = ctx.b.ins().load(types::I64, MemFlags::trusted(), recv_bits, 24);
                let s_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
                (s_bits, s_tag)
            } else if ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map {
                // Direct layout load: Vec len is at offset 8 from RwLock<MapObj> pointer.
                let s_bits = ctx.b.ins().load(types::I64, MemFlags::trusted(), recv_bits, 8);
                let s_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
                (s_bits, s_tag)
            } else {

                let k_val = ctx.b.ins().iconst(types::I32, kind as i64);
                let k_i8  = ctx.b.ins().ireduce(types::I8, k_val);
                let args_ptr = ctx.b.ins().iconst(types::I64, 0);
                let a_cnt_i8 = ctx.b.ins().iconst(types::I8, 0);
                ctx.spill_all();
                let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_method_dispatch, &[
                    recv_bits, recv_tag, k_i8, args_ptr, a_cnt_i8, ctx.executor_ptr
                ]);
                ctx.emit_halt_if_errors(symbols);
                (r_bits, r_tag)
            };
            if !ctx.should_skip_dec_ref(dst) {
                let (old_bits, old_tag) = ctx.use_local(dst);
                emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
            }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::ToStr if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Json => {
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_json_to_str, &[recv_bits, recv_tag]);
            if !ctx.should_skip_dec_ref(dst) {
                let (old_bits, old_tag) = ctx.use_local(dst);
                emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
            }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::StartsWith if arg_count == 1 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (pattern_bits, pattern_tag) = ctx.use_local(base + 1);
            let call = ctx.b.ins().call(symbols.xcx_jit_string_starts_with, &[recv_bits, recv_tag, pattern_bits, pattern_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let r_i64 = ctx.b.ins().uextend(types::I64, r_bits);
            let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, r_i64, bool_tag);
            return;
        }
        MethodKind::EndsWith if arg_count == 1 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (pattern_bits, pattern_tag) = ctx.use_local(base + 1);
            let call = ctx.b.ins().call(symbols.xcx_jit_string_ends_with, &[recv_bits, recv_tag, pattern_bits, pattern_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let r_i64 = ctx.b.ins().uextend(types::I64, r_bits);
            let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, r_i64, bool_tag);
            return;
        }
        MethodKind::Year | MethodKind::Month | MethodKind::Day | MethodKind::Hour | MethodKind::Minute | MethodKind::Second | MethodKind::Ms if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Date => {
            let kind_raw = match kind {
                MethodKind::Year => 1,
                MethodKind::Month => 2,
                MethodKind::Day => 3,
                MethodKind::Hour => 4,
                MethodKind::Minute => 5,
                MethodKind::Second => 6,
                MethodKind::Ms => 7,
                _ => 0,
            };
            let kind_val = ctx.b.ins().iconst(types::I64, kind_raw);
            let call = ctx.b.ins().call(symbols.xcx_jit_date_field, &[recv_bits, recv_tag, kind_val]);
            let r_bits = ctx.b.inst_results(call)[0];
            let r_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Upper if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_string_upper, &[recv_bits, recv_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Lower if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_string_lower, &[recv_bits, recv_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Trim if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_string_trim, &[recv_bits, recv_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Slice if arg_count == 2 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (start_bits, _start_tag) = ctx.use_local(base + 1);
            let (end_bits, _end_tag) = ctx.use_local(base + 2);
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_string_slice, &[recv_bits, recv_tag, start_bits, end_bits]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Replace if arg_count == 2 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (f_bits, f_tag) = ctx.use_local(base + 1);
            let (t_bits, t_tag) = ctx.use_local(base + 2);
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_string_replace, &[recv_bits, recv_tag, f_bits, f_tag, t_bits, t_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::IndexOf if arg_count == 1 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (pat_bits, pat_tag) = ctx.use_local(base + 1);
            let call = ctx.b.ins().call(symbols.xcx_jit_string_index_of, &[recv_bits, recv_tag, pat_bits, pat_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let r_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::LastIndexOf if arg_count == 1 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let (pat_bits, pat_tag) = ctx.use_local(base + 1);
            let call = ctx.b.ins().call(symbols.xcx_jit_string_last_index_of, &[recv_bits, recv_tag, pat_bits, pat_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let r_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::ToInt if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let call = ctx.b.ins().call(symbols.xcx_jit_string_to_int, &[recv_bits, recv_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let r_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::ToFloat if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::String => {
            let call = ctx.b.ins().call(symbols.xcx_jit_string_to_float, &[recv_bits, recv_tag]);
            let r_val = ctx.b.inst_results(call)[0];
            let r_bits = ctx.b.ins().bitcast(types::I64, MemFlags::new(), r_val);
            let r_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_FLOAT as i64);
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Contains if arg_count == 1 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map => {
            let (key_bits, key_tag) = ctx.use_local(base + 1);
            let call = ctx.b.ins().call(symbols.xcx_jit_map_contains, &[recv_bits, recv_tag, key_bits, key_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let r_i64 = ctx.b.ins().uextend(types::I64, r_bits);
            let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, r_i64, bool_tag);
            return;
        }
        MethodKind::Remove | MethodKind::Delete if arg_count == 1 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map => {
            let (key_bits, key_tag) = ctx.use_local(base + 1);
            let call = ctx.b.ins().call(symbols.xcx_jit_map_remove, &[recv_bits, recv_tag, key_bits, key_tag]);
            let r_bits = ctx.b.inst_results(call)[0];
            let r_i64 = ctx.b.ins().uextend(types::I64, r_bits);
            let bool_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, r_i64, bool_tag);
            return;
        }
        MethodKind::Clear if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map => {
            ctx.b.ins().call(symbols.xcx_jit_map_clear, &[recv_bits, recv_tag]);
            let t_bits = ctx.b.ins().iconst(types::I64, 1);
            let t_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.def_local(dst, t_bits, t_tag);
            return;
        }
        MethodKind::Keys if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map => {
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_map_keys, &[recv_bits, recv_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Values if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Map => {
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_map_values, &[recv_bits, recv_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        MethodKind::Values if arg_count == 0 && ctx.get_reg_type(base as usize) == crate::vm::opcode::TypeTag::Set => {
            let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_values, &[recv_bits, recv_tag]);
            if !ctx.should_skip_dec_ref(dst) { let (o_b, o_t) = ctx.use_local(dst); emit_conditional_dec_ref(ctx, symbols, o_b, o_t); }
            ctx.def_local(dst, r_bits, r_tag);
            return;
        }
        _ => {}
    }


    let kind_val = ctx.b.ins().iconst(types::I32, kind as i64);
    let kind_i8  = ctx.b.ins().ireduce(types::I8, kind_val);
    let args_ptr = if arg_count > 0 {
        ctx.b.ins().iadd_imm(ctx.locals_ptr, (base as i64 + 1) * 16)
    } else {
        ctx.b.ins().iconst(types::I64, 0)
    };
    let arg_cnt_val = ctx.b.ins().iconst(types::I32, arg_count as i64);
    let arg_cnt_i8  = ctx.b.ins().ireduce(types::I8, arg_cnt_val);

    ctx.spill_all();
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_method_dispatch, &[
        recv_bits, recv_tag, kind_i8, args_ptr, arg_cnt_i8, ctx.executor_ptr
    ]);
    ctx.emit_halt_if_errors(symbols);
    ctx.reload_globals();

    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_method_call_custom(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    method_name_idx: u32,
    base: u8,
    arg_count: u8,
    constants: &[crate::vm::value::Value],
) {
    let (recv_bits, recv_tag) = ctx.use_local(base);
    
    let name_val: crate::vm::value::Value = constants[method_name_idx as usize];
    let name_str = name_val.as_string();
    let data_ptr = name_str.data.as_ptr() as i64;
    let data_len = name_str.data.len() as i32;

    let name_ptr_val = ctx.b.ins().iconst(types::I64, data_ptr);
    let name_len_val = ctx.b.ins().iconst(types::I32, data_len as i64);

    let args_ptr = if arg_count > 0 {
        ctx.b.ins().iadd_imm(ctx.locals_ptr, (base as i64 + 1) * 16)
    } else {
        ctx.b.ins().iconst(types::I64, 0)
    };

    let ac_val = ctx.b.ins().iconst(types::I32, arg_count as i64);
    let ac_i8  = ctx.b.ins().ireduce(types::I8, ac_val);

    ctx.spill_all();
    let (r_bits, r_tag) = ctx.call_ffi_value(symbols.xcx_jit_method_call_custom, &[
        recv_bits, recv_tag, name_ptr_val, name_len_val, args_ptr, ac_i8, ctx.executor_ptr
    ]);
    ctx.reload_globals();

    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, r_bits, r_tag);
}

pub fn emit_method_call_named(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    kind: MethodKind,
    base: u8,
    arg_count: u8,
    names_idx: u32,
    constants: &[crate::vm::value::Value],
) {
    let (recv_bits, recv_tag) = ctx.use_local(base);


    let kind_val = ctx.b.ins().iconst(types::I32, kind as i64);
    let kind_i8  = ctx.b.ins().ireduce(types::I8, kind_val);
    let args_ptr = if arg_count > 0 {
        ctx.b.ins().iadd_imm(ctx.locals_ptr, (base as i64 + 1) * 16)
    } else {
        ctx.b.ins().iconst(types::I64, 0)
    };
    let arg_cnt_val = ctx.b.ins().iconst(types::I32, arg_count as i64);
    let arg_cnt_i8  = ctx.b.ins().ireduce(types::I8, arg_cnt_val);

    let name_val = constants[names_idx as usize];
    let names_bits_val = ctx.b.ins().iconst(types::I64, name_val.bits as i64);
    let names_tag_val = ctx.b.ins().iconst(types::I64, name_val.tag as i64);

    ctx.spill_all();
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_method_dispatch_named, &[
        recv_bits, recv_tag, kind_i8, args_ptr, arg_cnt_i8, names_bits_val, names_tag_val, ctx.executor_ptr
    ]);
    ctx.emit_halt_if_errors(symbols);
    ctx.reload_globals();

    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}
