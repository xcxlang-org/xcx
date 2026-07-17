use cranelift::prelude::*;
use super::codegen_ctx::CodegenCtx;
use crate::vm::opcode::TypeTag;

pub fn emit_guard_int(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    reg: u8,
    fail_ip: usize,
) {
    if ctx.known_types[reg as usize] == TypeTag::Int {
        return;
    }
    let (_bits, tag) = ctx.use_local(reg);
    let expected = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
    let not_int = ctx.b.ins().icmp(IntCC::NotEqual, tag, expected);
    
    let fail = ctx.create_block();
    let ok   = ctx.create_block();
    ctx.b.ins().brif(not_int, fail, &[], ok, &[]);
    ctx.b.switch_to_block(fail); 
    
    let sip = ctx.b.ins().iconst(types::I64, ctx.start_ip as i64);
    ctx.b.ins().call(symbols.xcx_jit_report_guard_failure, &[ctx.executor_ptr, sip]);
    
    ctx.spill_all();
    let rv = ctx.b.ins().iconst(types::I32, fail_ip as i64);
    ctx.b.ins().return_(&[rv]);
    ctx.b.switch_to_block(ok); 
    
    ctx.known_types[reg as usize] = TypeTag::Int;
}

pub fn emit_guard_float(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    reg: u8,
    fail_ip: usize,
) {
    if ctx.known_types[reg as usize] == TypeTag::Float {
        return;
    }
    let (_bits, tag) = ctx.use_local(reg);
    let expected = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_FLOAT as i64);
    let not_float = ctx.b.ins().icmp(IntCC::NotEqual, tag, expected);

    let fail = ctx.create_block();
    let ok   = ctx.create_block();
    
    ctx.b.ins().brif(not_float, fail, &[], ok, &[]);
    ctx.b.switch_to_block(fail); 
    
    let sip = ctx.b.ins().iconst(types::I64, ctx.start_ip as i64);
    ctx.b.ins().call(symbols.xcx_jit_report_guard_failure, &[ctx.executor_ptr, sip]);
    
    ctx.spill_all();
    let rv = ctx.b.ins().iconst(types::I32, fail_ip as i64);
    ctx.b.ins().return_(&[rv]);
    ctx.b.switch_to_block(ok); 
    
    ctx.known_types[reg as usize] = TypeTag::Float;
}

pub fn emit_guard_bool(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    reg: u8,
    fail_ip: usize,
    expected_bool: bool
) {
    let (bits, tag) = ctx.use_local(reg);
    
    let expected_val = ctx.b.ins().iconst(types::I64, if expected_bool { 1 } else { 0 });
    let not_match = ctx.b.ins().icmp(IntCC::NotEqual, bits, expected_val);
    
    let either_fail = if ctx.known_types[reg as usize] == TypeTag::Bool {
        not_match
    } else {
        let expected_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
        let not_bool = ctx.b.ins().icmp(IntCC::NotEqual, tag, expected_tag);
        ctx.b.ins().bor(not_bool, not_match)
    };

    let fail = ctx.create_block();
    let ok   = ctx.create_block();
    
    ctx.b.ins().brif(either_fail, fail, &[], ok, &[]);
    ctx.b.switch_to_block(fail); 
    
    let sip = ctx.b.ins().iconst(types::I64, ctx.start_ip as i64);
    ctx.b.ins().call(symbols.xcx_jit_report_guard_failure, &[ctx.executor_ptr, sip]);
    
    ctx.spill_all();
    let rv = ctx.b.ins().iconst(types::I32, fail_ip as i64);
    ctx.b.ins().return_(&[rv]);
    ctx.b.switch_to_block(ok); 
    
    ctx.known_types[reg as usize] = TypeTag::Bool;
}

pub fn emit_loop_exit(
    ctx: &mut CodegenCtx,
    cond: Value,
    loop_header: Block,
    loop_target: usize,
    _start_ip: usize,
    exit_ip: usize,
    current_terminated: &mut bool
) {
    if let Some(&target_blk) = ctx.bc_blocks.get(&loop_target) {
        let exit_blk = ctx.create_block();
        ctx.b.ins().brif(cond, target_blk, &[], exit_blk, &[]);
        ctx.b.switch_to_block(exit_blk);
        
        ctx.sync_for_jump();
        let rv = ctx.b.ins().iconst(types::I32, exit_ip as i64);
        ctx.b.ins().return_(&[rv]);
    } else {
        let exit_blk = ctx.create_block();
        ctx.b.ins().brif(cond, loop_header, &[], exit_blk, &[]);
        ctx.b.switch_to_block(exit_blk);
    }
    *current_terminated = true;
}

pub fn emit_loop_next_generic(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    cond: Value,
    target_blk: Block,
    next_blk: Block
) {
    ctx.b.ins().brif(cond, target_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_loop_next_int(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    reg: u8,
    limit_reg: u8,
    loop_header: Block,
    target_ip: usize,
    start_ip: usize,
    exit_ip: usize,
    current_terminated: &mut bool
) {
    let (v_bits, v_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(v_bits, 1);
    ctx.def_local(reg, next, v_tag);
    
    ctx.sync_for_jump();

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);

    let cond = ctx.b.ins().icmp(IntCC::SignedLessThanOrEqual, next, limit_bits);
    emit_loop_exit(ctx, cond, loop_header, target_ip, start_ip, exit_ip, current_terminated);
}

pub fn emit_inc_local_loop_next(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    inc_reg: u8,
    reg: u8,
    limit_reg: u8,
    loop_header: Block,
    target_ip: usize,
    start_ip: usize,
    exit_ip: usize,
    current_terminated: &mut bool
) {
    let (v_bits, v_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(v_bits, 1);
    ctx.def_local(reg, next, v_tag);

    let final_comp_val = if inc_reg != reg {
        let (iv_bits, iv_tag) = ctx.use_local(inc_reg);
        let inxt = ctx.b.ins().iadd_imm(iv_bits, 1);
        ctx.def_local(inc_reg, inxt, iv_tag);
        next
    } else {
        next
    };

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedLessThanOrEqual, final_comp_val, limit_bits);
    ctx.sync_for_jump();
    emit_loop_exit(ctx, cond, loop_header, target_ip, start_ip, exit_ip, current_terminated);
}

pub fn emit_inc_var_loop_next(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    g_idx: u32,
    reg: u8,
    limit_reg: u8,
    loop_header: Block,
    target_ip: usize,
    start_ip: usize,
    exit_ip: usize,
    current_terminated: &mut bool
) {
    let (old_g_bits, old_g_tag) = ctx.use_global(g_idx);
    let next_g = ctx.b.ins().iadd_imm(old_g_bits, 1);
    ctx.def_global(g_idx, next_g, old_g_tag);

    let (rv_bits, rv_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(rv_bits, 1);
    ctx.def_local(reg, next, rv_tag);

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);

    let cond = ctx.b.ins().icmp(IntCC::SignedLessThanOrEqual, next, limit_bits);
    emit_loop_exit(ctx, cond, loop_header, target_ip, start_ip, exit_ip, current_terminated);
}

pub fn emit_array_loop_next(
    ctx: &mut CodegenCtx,
    idx_reg: u8,
    size_reg: u8,
    loop_header: Block,
    target_ip: u32,
    start_ip: usize,
    exit_ip: usize,
    current_terminated: &mut bool
) {
    let (idx_bits, idx_tag) = ctx.use_local(idx_reg);
    let next_idx = ctx.b.ins().iadd_imm(idx_bits, 1);
    ctx.def_local(idx_reg, next_idx, idx_tag);
    
    let (size_bits, _size_tag) = ctx.use_local(size_reg);
    
    let cond = ctx.b.ins().icmp(IntCC::SignedLessThan, next_idx, size_bits);
    emit_loop_exit(ctx, cond, loop_header, target_ip as usize, start_ip, exit_ip, current_terminated);
}

pub fn emit_table_iter(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    tbl_reg: u8,
    idx_reg: u8,
    row_reg: u8,
    limit_reg: u8,
    loop_header: Block,
    _start_ip: usize,
    exit_ip: usize,
    current_terminated: &mut bool,
) {
    let (idx_bits, idx_tag) = ctx.use_local(idx_reg);
    let next_idx = ctx.b.ins().iadd_imm(idx_bits, 1);
    ctx.def_local(idx_reg, next_idx, idx_tag);
    
    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedLessThan, next_idx, limit_bits);
    
    let body_blk = ctx.create_block();
    let exit_blk = ctx.create_block();
    
    ctx.b.ins().brif(cond, body_blk, &[], exit_blk, &[]);
    ctx.b.switch_to_block(body_blk);
    
    let (tbl_bits, tbl_tag) = ctx.use_local(tbl_reg);
    
    let (new_row_bits, new_row_tag) = ctx.call_ffi_value(symbols.xcx_jit_table_get_row, &[tbl_bits, tbl_tag, next_idx]);
    
    let (old_row_bits, old_row_tag) = ctx.use_local(row_reg);
    super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_row_bits, old_row_tag);
    
    ctx.def_local(row_reg, new_row_bits, new_row_tag);
    
    ctx.sync_for_jump();
    ctx.b.ins().jump(loop_header, &[]);
    
    ctx.b.switch_to_block(exit_blk);
    let rv = ctx.b.ins().iconst(types::I32, exit_ip as i64);
    ctx.b.ins().return_(&[rv]);
    *current_terminated = true;
}

pub fn emit_table_size(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst_reg: u8,
    tbl_reg: u8,
) {
    let (tv_bits, tv_tag) = ctx.use_local(tbl_reg);
    let call = ctx.b.ins().call(symbols.xcx_jit_table_size, &[tv_bits, tv_tag]);
    let size_raw = ctx.b.inst_results(call)[0];
    let int_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
    
    ctx.def_local(dst_reg, size_raw, int_tag);
}

pub fn emit_yield(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    src: u8,
    next_ip: usize,
) {
    let (val_bits, val_tag) = ctx.use_local(src);
    
    ctx.b.ins().call(symbols.xcx_jit_inc_ref, &[val_bits, val_tag]);

    let nip_val = ctx.b.ins().iconst(types::I64, next_ip as i64);
    let y_val = ctx.b.ins().iconst(types::I32, 1);
    ctx.b.ins().call(symbols.xcx_jit_set_fiber_state, &[ctx.executor_ptr, nip_val, y_val]);

    // Construct the out Value for JIT ABI: write to out_ptr natively.
    // The executor expects return value out_ptr passed as the first parameter.
    let out_ptr = ctx.b.func.dfg.block_params(ctx.b.func.layout.entry_block().unwrap())[0];
    ctx.b.ins().store(super::abi::trusted(), val_bits, out_ptr, 0);
    ctx.b.ins().store(super::abi::trusted(), val_tag, out_ptr, 8);

    ctx.spill_all();
    ctx.b.ins().return_(&[]);
}

pub fn emit_yield_void(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    next_ip: usize,
) {
    let nip_val = ctx.b.ins().iconst(types::I64, next_ip as i64);
    let y_val = ctx.b.ins().iconst(types::I32, 1);
    ctx.b.ins().call(symbols.xcx_jit_set_fiber_state, &[ctx.executor_ptr, nip_val, y_val]);

    let out_ptr = ctx.b.func.dfg.block_params(ctx.b.func.layout.entry_block().unwrap())[0];
    let false_bits = ctx.b.ins().iconst(types::I64, 0);
    let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
    ctx.b.ins().store(super::abi::trusted(), false_bits, out_ptr, 0);
    ctx.b.ins().store(super::abi::trusted(), false_tag, out_ptr, 8);

    ctx.spill_all();
    ctx.b.ins().return_(&[]);
}

pub fn emit_method_yield(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    src: u8,
    next_ip: usize,
    terminated: &mut bool,
) {
    ctx.spill_globals();

    let (val_bits, val_tag) = ctx.use_local(src);
    let nip_val = ctx.b.ins().iconst(types::I64, next_ip as i64);
    let out_ptr = ctx.out_ptr;
    
    let call = ctx.b.ins().call(symbols.xcx_jit_yield, &[ctx.executor_ptr, val_bits, val_tag, nip_val, out_ptr]);
    let status = ctx.b.inst_results(call)[0];

    let yield_block = ctx.create_block();
    let cont_block = ctx.create_block();
    
    let cond = ctx.b.ins().icmp_imm(IntCC::NotEqual, status, 0);
    ctx.b.ins().brif(cond, yield_block, &[], cont_block, &[]);
    
    ctx.b.switch_to_block(yield_block);
    ctx.spill_all();
    let zero_i32 = ctx.b.ins().iconst(types::I32, 0);
    ctx.b.ins().return_(&[zero_i32]);
    
    ctx.b.switch_to_block(cont_block);
    *terminated = false;
}

pub fn emit_method_yield_void(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    next_ip: usize,
    terminated: &mut bool,
) {
    ctx.spill_globals();

    let nip_val = ctx.b.ins().iconst(types::I64, next_ip as i64);
    let false_bits = ctx.b.ins().iconst(types::I64, 0);
    let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
    let out_ptr = ctx.out_ptr;
    
    let call = ctx.b.ins().call(symbols.xcx_jit_yield, &[ctx.executor_ptr, false_bits, false_tag, nip_val, out_ptr]);
    let status = ctx.b.inst_results(call)[0];

    let yield_block = ctx.create_block();
    let cont_block = ctx.create_block();
    
    let cond = ctx.b.ins().icmp_imm(IntCC::NotEqual, status, 0);
    ctx.b.ins().brif(cond, yield_block, &[], cont_block, &[]);
    
    ctx.b.switch_to_block(yield_block);
    ctx.spill_all();
    let zero_i32 = ctx.b.ins().iconst(types::I32, 0);
    ctx.b.ins().return_(&[zero_i32]);
    
    ctx.b.switch_to_block(cont_block);
    *terminated = false;
}


pub fn emit_return_fiber(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    src: Option<u8>,
) {
    let out_ptr = ctx.b.func.dfg.block_params(ctx.b.func.layout.entry_block().unwrap())[0];

    if let Some(s) = src {
        let (val_bits, val_tag) = ctx.use_local(s);
        ctx.b.ins().call(symbols.xcx_jit_inc_ref, &[val_bits, val_tag]);
        ctx.b.ins().store(super::abi::trusted(), val_bits, out_ptr, 0);
        ctx.b.ins().store(super::abi::trusted(), val_tag,  out_ptr, 8);
    } else {
        let false_bits = ctx.b.ins().iconst(types::I64, 0);
        let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
        ctx.b.ins().store(super::abi::trusted(), false_bits, out_ptr, 0);
        ctx.b.ins().store(super::abi::trusted(), false_tag, out_ptr, 8);
    }

    let nip_val = ctx.b.ins().iconst(types::I64, 0);
    let y_val = ctx.b.ins().iconst(types::I32, 0);
    ctx.b.ins().call(symbols.xcx_jit_set_fiber_state, &[ctx.executor_ptr, nip_val, y_val]);

    ctx.spill_all();
    ctx.b.ins().return_(&[]);
}

pub fn emit_jump(
    ctx: &mut CodegenCtx,
    blocks: &std::collections::HashMap<usize, Block>,
    target: u32,
    terminated: &mut bool,
) {
    ctx.sync_for_jump();
    let target_blk = blocks[&(target as usize)];
    ctx.b.ins().jump(target_blk, &[]);
    *terminated = true;
}

pub fn emit_jump_if(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    src: u8,
    target: u32,
    if_true: bool,
) {
    let (sv_bits, sv_tag) = ctx.use_local(src);
    let is_false = if ctx.get_reg_type(src as usize) == crate::vm::opcode::TypeTag::Bool {
        ctx.b.ins().icmp_imm(IntCC::Equal, sv_bits, 0)
    } else {
        let expected_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
        let expected_bits = ctx.b.ins().iconst(types::I64, 0);
        let eq_tag = ctx.b.ins().icmp(IntCC::Equal, sv_tag, expected_tag);
        let eq_bits = ctx.b.ins().icmp(IntCC::Equal, sv_bits, expected_bits);
        ctx.b.ins().band(eq_tag, eq_bits)
    };
    
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();
    
    ctx.sync_for_jump();

    if if_true {
        ctx.b.ins().brif(is_false, next_blk, &[], target_blk, &[]);
    } else {
        ctx.b.ins().brif(is_false, target_blk, &[], next_blk, &[]);
    }
    
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_return(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    src: Option<u8>,
    terminated: &mut bool,
) {
    ctx.spill_globals();

    if ctx.is_inner_func {
        ctx.cleanup_all(symbols, src);
        let out_ptr = ctx.out_ptr;
        if let Some(s) = src {
            let (bits, tag) = ctx.use_local(s);
            ctx.b.ins().store(super::abi::trusted(), bits, out_ptr, 0);
            ctx.b.ins().store(super::abi::trusted(), tag,  out_ptr, 8);
        } else {
            let false_bits = ctx.b.ins().iconst(types::I64, 0);
            let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
            ctx.b.ins().store(super::abi::trusted(), false_bits, out_ptr, 0);
            ctx.b.ins().store(super::abi::trusted(), false_tag, out_ptr, 8);
        }
        let status = ctx.b.ins().iconst(types::I32, 0);
        ctx.b.ins().return_(&[status]);
        *terminated = true;
        return;
    }

    let out_ptr = ctx.out_ptr;

    if let Some(s) = src {
        let (bits, tag) = ctx.use_local(s);
        ctx.b.ins().store(super::abi::trusted(), bits, out_ptr, 0);
        ctx.b.ins().store(super::abi::trusted(), tag,  out_ptr, 8);
    } else {
        let false_bits = ctx.b.ins().iconst(types::I64, 0);
        let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
        ctx.b.ins().store(super::abi::trusted(), false_bits, out_ptr, 0);
        ctx.b.ins().store(super::abi::trusted(), false_tag, out_ptr, 8);
    }

    ctx.cleanup_all(symbols, src);
    let rv = ctx.b.ins().iconst(types::I32, 0);
    ctx.b.ins().return_(&[rv]);
    *terminated = true;
}

pub fn emit_loop_next(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    reg: u8,
    limit_reg: u8,
    target: u32,
) {
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();
    
    let (v_bits, v_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(v_bits, 1);
    ctx.def_local(reg, next, v_tag);

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedLessThanOrEqual, next, limit_bits);
    
    ctx.sync_for_jump();
    ctx.b.ins().brif(cond, target_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_loop_prev(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    reg: u8,
    limit_reg: u8,
    target: u32,
) {
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();
    
    let (v_bits, v_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(v_bits, -1);
    ctx.def_local(reg, next, v_tag);

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedGreaterThanOrEqual, next, limit_bits);
    
    ctx.sync_for_jump();
    ctx.b.ins().brif(cond, target_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_inc_local_loop_next_opcode(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    inc_reg: u8,
    reg: u8,
    limit_reg: u8,
    target: u32,
) {
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();

    let (v_bits, v_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(v_bits, 1);
    ctx.def_local(reg, next, v_tag);
    
    let (iv_bits, iv_tag) = ctx.use_local(inc_reg);
    let inxt = ctx.b.ins().iadd_imm(iv_bits, 1);
    ctx.def_local(inc_reg, inxt, iv_tag);

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedLessThanOrEqual, next, limit_bits);
    
    ctx.sync_for_jump();
    ctx.b.ins().brif(cond, target_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_dec_local_loop_prev_opcode(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    dec_reg: u8,
    reg: u8,
    limit_reg: u8,
    target: u32,
) {
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();

    let (v_bits, v_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(v_bits, -1);
    ctx.def_local(reg, next, v_tag);
    
    let (iv_bits, iv_tag) = ctx.use_local(dec_reg);
    let inxt = ctx.b.ins().iadd_imm(iv_bits, -1);
    ctx.def_local(dec_reg, inxt, iv_tag);

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedGreaterThanOrEqual, next, limit_bits);
    
    ctx.sync_for_jump();
    ctx.b.ins().brif(cond, target_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_inc_var_loop_next_opcode(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    g_idx: u32,
    reg: u8,
    limit_reg: u8,
    target: u32,
) {
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();

    let (old_g_bits, old_g_tag) = ctx.use_global(g_idx);
    let next_g = ctx.b.ins().iadd_imm(old_g_bits, 1);
    ctx.def_global(g_idx, next_g, old_g_tag);

    let (rv_bits, rv_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(rv_bits, 1);
    ctx.def_local(reg, next, rv_tag);

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedLessThanOrEqual, next, limit_bits);

    ctx.sync_for_jump();
    ctx.b.ins().brif(cond, target_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_dec_var_loop_prev_opcode(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    g_idx: u32,
    reg: u8,
    limit_reg: u8,
    target: u32,
) {
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();

    let (old_g_bits, old_g_tag) = ctx.use_global(g_idx);
    let next_g = ctx.b.ins().iadd_imm(old_g_bits, -1);
    ctx.def_global(g_idx, next_g, old_g_tag);

    let (rv_bits, rv_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(rv_bits, -1);
    ctx.def_local(reg, next, rv_tag);

    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedGreaterThanOrEqual, next, limit_bits);

    ctx.sync_for_jump();
    ctx.b.ins().brif(cond, target_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_array_loop_next_opcode(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    idx_reg: u8,
    size_reg: u8,
    target: u32,
) {
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();

    let (idx_bits, idx_tag) = ctx.use_local(idx_reg);
    let next_idx = ctx.b.ins().iadd_imm(idx_bits, 1);
    ctx.def_local(idx_reg, next_idx, idx_tag);
    
    let (size_bits, _size_tag) = ctx.use_local(size_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedLessThan, next_idx, size_bits);
    
    ctx.sync_for_jump();
    ctx.b.ins().brif(cond, target_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_table_iter_opcode(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    blocks: &std::collections::HashMap<usize, Block>,
    tbl_reg: u8,
    idx_reg: u8,
    row_reg: u8,
    limit_reg: u8,
    target: u32,
) {
    let target_blk = blocks[&(target as usize)];
    let next_blk = ctx.create_block();

    let (idx_bits, idx_tag) = ctx.use_local(idx_reg);
    let next_idx = ctx.b.ins().iadd_imm(idx_bits, 1);
    ctx.def_local(idx_reg, next_idx, idx_tag);
    
    let (limit_bits, _limit_tag) = ctx.use_local(limit_reg);
    let cond = ctx.b.ins().icmp(IntCC::SignedLessThan, next_idx, limit_bits);
    
    let fetch_blk = ctx.create_block();
    
    ctx.b.ins().brif(cond, fetch_blk, &[], next_blk, &[]);
    ctx.b.switch_to_block(fetch_blk);
    
    let (tbl_bits, tbl_tag) = ctx.use_local(tbl_reg);
    let (new_row_bits, new_row_tag) = ctx.call_ffi_value(symbols.xcx_jit_table_get_row, &[tbl_bits, tbl_tag, next_idx]);
    
    let (old_row_bits, old_row_tag) = ctx.use_local(row_reg);
    super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_row_bits, old_row_tag);
    
    ctx.def_local(row_reg, new_row_bits, new_row_tag);
    
    ctx.sync_for_jump();
    ctx.b.ins().jump(target_blk, &[]);
    
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(false);
}

pub fn emit_http_serve(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    func_idx: u32,
    port_src: u8,
    host_src: u8,
    _workers_src: u8,
    routes_src: u8,
) {
    ctx.spill_all();
    let f_idx = ctx.b.ins().iconst(types::I32, func_idx as i64);
    let (p_bits, p_tag) = ctx.use_local(port_src);
    let (h_bits, h_tag) = ctx.use_local(host_src);
    let (r_bits, r_tag) = ctx.use_local(routes_src);
    
    let call = ctx.b.ins().call(symbols.xcx_jit_http_serve, &[
        f_idx, p_bits, p_tag, h_bits, h_tag, r_bits, r_tag, ctx.executor_ptr
    ]);
    let halt_status = ctx.b.inst_results(call)[0];
    
    let halt_block = ctx.create_block();
    let cont_block = ctx.create_block();
    
    let cond = ctx.b.ins().icmp_imm(IntCC::NotEqual, halt_status, 0);
    ctx.b.ins().brif(cond, halt_block, &[], cont_block, &[]);
    
    ctx.b.switch_to_block(halt_block);
    let one_i32 = ctx.b.ins().iconst(types::I32, 1);
    ctx.b.ins().return_(&[one_i32]);
    
    ctx.b.switch_to_block(cont_block);
}

pub fn emit_http_respond(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    status_src: u8,
    body_src: u8,
    headers_src: u8,
) {
    ctx.spill_all();
    let (s_bits, s_tag) = ctx.use_local(status_src);
    let (b_bits, b_tag) = ctx.use_local(body_src);
    let (h_bits, h_tag) = ctx.use_local(headers_src);
    
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_http_respond, &[
        s_bits, s_tag, b_bits, b_tag, h_bits, h_tag, ctx.executor_ptr
    ]);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}
