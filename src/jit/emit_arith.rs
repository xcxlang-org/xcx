use cranelift::prelude::*;
use cranelift_codegen::ir::FuncRef;
use super::codegen_ctx::CodegenCtx;
use crate::vm::value::{TAG_INT, TAG_FLOAT, TAG_BOOL};
use super::nan_ops::emit_conditional_dec_ref;

pub fn emit_const_int(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, val: i64) {
    let res = ctx.b.ins().iconst(types::I64, val);
    if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    let int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
    ctx.def_local(dst, res, int_tag);
    ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::Int;
    ctx.register_const[dst as usize] = Some(val);
}

pub fn emit_binop_int<F>(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    _sign_extend: bool, 
    op: F
) where F: FnOnce(&mut FunctionBuilder, Value, Value) -> Value {
    let (l_bits, _) = ctx.use_local(src1);
    let (r_bits, _) = if src1 == src2 { (l_bits, ctx.b.ins().iconst(types::I64, 0)) } else { ctx.use_local(src2) };
    
    let res = op(ctx.b, l_bits, r_bits);
    
    if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }

    let int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
    ctx.def_local(dst, res, int_tag);
    ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::Int;
    ctx.register_const[dst as usize] = None;
}

pub fn emit_add_int(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    if let (Some(l), Some(r)) = (ctx.register_const[src1 as usize], ctx.register_const[src2 as usize]) {
        emit_const_int(ctx, symbols, dst, l.wrapping_add(r));
        return;
    }
    emit_binop_int(ctx, symbols, dst, src1, src2, false, |b, l, r| b.ins().iadd(l, r));
}

pub fn emit_sub_int(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    if let (Some(l), Some(r)) = (ctx.register_const[src1 as usize], ctx.register_const[src2 as usize]) {
        emit_const_int(ctx, symbols, dst, l.wrapping_sub(r));
        return;
    }
    emit_binop_int(ctx, symbols, dst, src1, src2, false, |b, l, r| b.ins().isub(l, r));
}

pub fn emit_mul_int(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    if let (Some(l), Some(r)) = (ctx.register_const[src1 as usize], ctx.register_const[src2 as usize]) {
        emit_const_int(ctx, symbols, dst, l.wrapping_mul(r));
        return;
    }
    emit_binop_int(ctx, symbols, dst, src1, src2, true, |b, l, r| b.ins().imul(l, r));
}

pub fn emit_div_int(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    fail_ip: usize,
    is_mod: bool,
) {
    let (l_bits, _) = ctx.use_local(src1);
    let (r_bits, _) = ctx.use_local(src2);
    
    let is_zero = ctx.b.ins().icmp_imm(IntCC::Equal, r_bits, 0);
    let is_min = ctx.b.ins().icmp_imm(IntCC::Equal, l_bits, i64::MIN);
    let is_minus_one = ctx.b.ins().icmp_imm(IntCC::Equal, r_bits, -1);
    let is_overflow = ctx.b.ins().band(is_min, is_minus_one);
    let should_fail = ctx.b.ins().bor(is_zero, is_overflow);
    
    let fail = ctx.create_block();
    let ok = ctx.create_block();
    
    ctx.b.ins().brif(should_fail, fail, &[], ok, &[]);
    ctx.b.switch_to_block(fail);
    
    let sip = ctx.b.ins().iconst(types::I64, ctx.start_ip as i64);
    ctx.b.ins().call(symbols.xcx_jit_report_guard_failure, &[ctx.executor_ptr, sip]);
    
    ctx.spill_all();
    let rv = ctx.b.ins().iconst(types::I32, fail_ip as i64);
    ctx.b.ins().return_(&[rv]);
    
    ctx.b.switch_to_block(ok);
    let s = if is_mod {
        ctx.b.ins().srem(l_bits, r_bits)
    } else {
        ctx.b.ins().sdiv(l_bits, r_bits)
    };
    
    if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    
    let int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
    ctx.def_local(dst, s, int_tag);
    ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::Int;
}

pub fn emit_mod_int(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    fail_ip: usize,
) {
    emit_div_int(ctx, symbols, dst, src1, src2, fail_ip, true);
}

pub fn emit_binop_float<F>(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    op: F
) where F: FnOnce(&mut FunctionBuilder, Value, Value) -> Value {
    let (l_bits, _) = ctx.use_local(src1);
    let (r_bits, _) = if src1 == src2 { (l_bits, ctx.b.ins().iconst(types::I64, 0)) } else { ctx.use_local(src2) };
    
    let l_f64 = ctx.b.ins().bitcast(types::F64, MemFlags::new(), l_bits);
    let r_f64 = ctx.b.ins().bitcast(types::F64, MemFlags::new(), r_bits);
    
    let res = op(ctx.b, l_f64, r_f64);
    let res_bits = ctx.b.ins().bitcast(types::I64, MemFlags::new(), res);
    let float_tag = ctx.b.ins().iconst(types::I64, TAG_FLOAT as i64);
    
    if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    
    ctx.def_local(dst, res_bits, float_tag);
    ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::Float;
}

pub fn emit_add_float(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    emit_binop_float(ctx, symbols, dst, src1, src2, |b, l, r| b.ins().fadd(l, r));
}

pub fn emit_sub_float(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    emit_binop_float(ctx, symbols, dst, src1, src2, |b, l, r| b.ins().fsub(l, r));
}

pub fn emit_mul_float(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    emit_binop_float(ctx, symbols, dst, src1, src2, |b, l, r| b.ins().fmul(l, r));
}

pub fn emit_div_float(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    let (l_bits, _) = ctx.use_local(src1);
    let (r_bits, _) = if src1 == src2 { (l_bits, ctx.b.ins().iconst(types::I64, 0)) } else { ctx.use_local(src2) };
    
    let l_f64 = ctx.b.ins().bitcast(types::F64, MemFlags::new(), l_bits);
    let r_f64 = ctx.b.ins().bitcast(types::F64, MemFlags::new(), r_bits);
    
    let z_f64 = ctx.b.ins().f64const(0.0);
    let is_zero = ctx.b.ins().fcmp(FloatCC::Equal, r_f64, z_f64);
    
    let fast_blk = ctx.create_block();
    let slow_blk = ctx.create_block();
    
    ctx.b.ins().brif(is_zero, slow_blk, &[], fast_blk, &[]);
    
    ctx.b.switch_to_block(slow_blk);
    ctx.b.ins().call(symbols.xcx_jit_abort_div, &[ctx.executor_ptr]);
    ctx.spill_all();
    let t_status = ctx.b.ins().iconst(types::I32, 1);
    ctx.b.ins().return_(&[t_status]);
    
    ctx.b.switch_to_block(fast_blk);
    let res = ctx.b.ins().fdiv(l_f64, r_f64);
    let res_bits = ctx.b.ins().bitcast(types::I64, MemFlags::new(), res);
    let float_tag = ctx.b.ins().iconst(types::I64, TAG_FLOAT as i64);
    
    if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    
    ctx.def_local(dst, res_bits, float_tag);
    ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::Float;
}

pub fn emit_neg_int(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src: u8) {
    let (sv_bits, _) = ctx.use_local(src);
    let s = ctx.b.ins().irsub_imm(sv_bits, 0);
    let int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    ctx.def_local(dst, s, int_tag);
}

pub fn emit_neg_float(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src: u8) {
    let (sv_bits, _) = ctx.use_local(src);
    let f = ctx.b.ins().bitcast(types::F64, MemFlags::new(), sv_bits);
    let s = ctx.b.ins().fneg(f);
    let res_bits = ctx.b.ins().bitcast(types::I64, MemFlags::new(), s);
    let float_tag = ctx.b.ins().iconst(types::I64, TAG_FLOAT as i64);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    ctx.def_local(dst, res_bits, float_tag);
}

pub fn emit_cast_to_float(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src: u8) {
    use crate::vm::opcode::TypeTag as CTypeTag;

    if ctx.get_reg_type(src as usize) == CTypeTag::Float {
        let (v_bits, v_tag) = ctx.use_local(src);
        if dst != src {
            if !ctx.should_skip_dec_ref(dst) {
                let (old_bits, old_tag) = ctx.use_local(dst);
                super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
            }
            ctx.def_local(dst, v_bits, v_tag);
        }
        return;
    }

    if ctx.get_reg_type(src as usize) == CTypeTag::Int {
        let (sv_bits, _) = ctx.use_local(src);
        let f = ctx.b.ins().fcvt_from_sint(types::F64, sv_bits);
        let res_bits = ctx.b.ins().bitcast(types::I64, MemFlags::new(), f);
        let float_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_FLOAT as i64);
        
        if !ctx.should_skip_dec_ref(dst) {
            let (v_bits, v_tag) = ctx.use_local(dst);
            super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
        }
        ctx.def_local(dst, res_bits, float_tag);
        return;
    }

    let (s_bits, s_tag) = ctx.use_local(src);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_cast_float, &[s_bits, s_tag]);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_cmp_int(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    cc_raw: u8
) {
    let (l_bits, _) = ctx.use_local(src1);
    let (r_bits, _) = if src1 == src2 { (l_bits, ctx.b.ins().iconst(types::I64, 0)) } else { ctx.use_local(src2) };
    
    let cc = match cc_raw {
        0 => IntCC::Equal,
        1 => IntCC::NotEqual,
        2 => IntCC::SignedGreaterThan,
        3 => IntCC::SignedLessThan,
        4 => IntCC::SignedGreaterThanOrEqual,
        5 => IntCC::SignedLessThanOrEqual,
        _ => IntCC::Equal,
    };

    let cond = ctx.b.ins().icmp(cc, l_bits, r_bits);
    let res_i64 = ctx.b.ins().uextend(types::I64, cond);
    let bool_tag = ctx.b.ins().iconst(types::I64, TAG_BOOL as i64);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    ctx.def_local(dst, res_i64, bool_tag);
}

pub fn emit_cmp_float(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    cc_raw: u8,
) {
    let (l_bits, _) = ctx.use_local(src1);
    let (r_bits, _) = if src1 == src2 { (l_bits, ctx.b.ins().iconst(types::I64, 0)) } else { ctx.use_local(src2) };
    
    let l_f64 = ctx.b.ins().bitcast(types::F64, MemFlags::new(), l_bits);
    let r_f64 = ctx.b.ins().bitcast(types::F64, MemFlags::new(), r_bits);
    
    let cc = super::abi::decode_floatcc(cc_raw);
    let cmp = ctx.b.ins().fcmp(cc, l_f64, r_f64);
    let res = ctx.b.ins().uextend(types::I64, cmp);
    let bool_tag = ctx.b.ins().iconst(types::I64, TAG_BOOL as i64);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    ctx.def_local(dst, res, bool_tag);
}

pub fn emit_int_concat(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
) {
    let (l_bits, _) = ctx.use_local(src1);
    let (r_bits, _) = if src1 == src2 { (l_bits, ctx.b.ins().iconst(types::I64, 0)) } else { ctx.use_local(src2) };
    
    // Fast path assumes numbers are properly tracked so string creation shouldn't crash.
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_int_concat, &[l_bits, r_bits]);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (v_bits, v_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_cast_to_int(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src: u8,
) {
    use crate::vm::opcode::TypeTag as CTypeTag;

    if ctx.get_reg_type(src as usize) == CTypeTag::Int {
        let (v_bits, v_tag) = ctx.use_local(src);
        if dst != src {
            if !ctx.should_skip_dec_ref(dst) {
                let (old_bits, old_tag) = ctx.use_local(dst);
                emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
            }
            ctx.def_local(dst, v_bits, v_tag);
        }
        return;
    }

    let (s_bits, s_tag) = ctx.use_local(src);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_cast_int, &[s_bits, s_tag]);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_poly_int_fast_path<F>(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    ffi_func: FuncRef,
    op: F
) where F: FnOnce(&mut FunctionBuilder, Value, Value) -> Value {
    use crate::vm::opcode::TypeTag as CTypeTag;

    if ctx.get_reg_type(src1 as usize) == CTypeTag::Int && ctx.get_reg_type(src2 as usize) == CTypeTag::Int {
        let (l_bits, _) = ctx.use_local(src1);
        let (r_bits, _) = ctx.use_local(src2);
        let s = op(ctx.b, l_bits, r_bits);
        let int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
        
        if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
            let (v_bits, v_tag) = ctx.use_local(dst);
            super::nan_ops::emit_conditional_dec_ref(ctx, symbols, v_bits, v_tag);
        }
        ctx.def_local(dst, s, int_tag);
        return;
    }
    
    let (v1_bits, v1_tag) = ctx.use_local(src1);
    let (v2_bits, v2_tag) = ctx.use_local(src2);
    
    let cmp_int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
    let is_int1 = ctx.b.ins().icmp(IntCC::Equal, v1_tag, cmp_int_tag);
    let is_int2 = ctx.b.ins().icmp(IntCC::Equal, v2_tag, cmp_int_tag);
    let both_int = ctx.b.ins().band(is_int1, is_int2);
    
    let fast_path = ctx.create_block();
    let slow_path = ctx.create_block();
    let next_blk  = ctx.create_block();
    
    let res_bits_var = ctx.b.declare_var(types::I64);
    let res_tag_var  = ctx.b.declare_var(types::I64);

    ctx.b.ins().brif(both_int, fast_path, &[], slow_path, &[]);
    
    // --- Fast Path ---
    ctx.b.switch_to_block(fast_path);
    let s = op(ctx.b, v1_bits, v2_bits);
    ctx.b.def_var(res_bits_var, s);
    ctx.b.def_var(res_tag_var, cmp_int_tag);
    ctx.b.ins().jump(next_blk, &[]);

    // --- Slow Path ---
    ctx.b.switch_to_block(slow_path);
    let (s_bits, s_tag) = ctx.call_ffi_value(ffi_func, &[v1_bits, v1_tag, v2_bits, v2_tag]);
    ctx.b.def_var(res_bits_var, s_bits);
    ctx.b.def_var(res_tag_var, s_tag);
    ctx.b.ins().jump(next_blk, &[]);

    // --- Next Blk ---
    ctx.b.switch_to_block(next_blk);
    let final_bits = ctx.b.use_var(res_bits_var);
    let final_tag = ctx.b.use_var(res_tag_var);
    
    if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    
    ctx.def_local(dst, final_bits, final_tag);
    ctx.clear_block_state(true);
}

pub fn emit_add_poly(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    emit_poly_int_fast_path(ctx, symbols, dst, src1, src2, symbols.xcx_jit_add, |b, l, r| b.ins().iadd(l, r));
}

pub fn emit_sub_poly(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    emit_poly_int_fast_path(ctx, symbols, dst, src1, src2, symbols.xcx_jit_sub, |b, l, r| b.ins().isub(l, r));
}

pub fn emit_mul_poly(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    emit_poly_int_fast_path(ctx, symbols, dst, src1, src2, symbols.xcx_jit_mul, |b, l, r| b.ins().imul(l, r));
}

pub fn emit_poly_div_mod_fast_path(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    ffi_func: FuncRef,
    is_mod: bool
) {
    use crate::vm::opcode::TypeTag as CTypeTag;

    if ctx.get_reg_type(src1 as usize) == CTypeTag::Int && ctx.get_reg_type(src2 as usize) == CTypeTag::Int {
        let (l_bits, _) = ctx.use_local(src1);
        let (r_bits, _) = ctx.use_local(src2);
        
        let divisor_opt = ctx.register_const[src2 as usize];
        if is_mod {
            if let Some(divisor) = divisor_opt {
                if divisor > 0 && (divisor & (divisor - 1)) == 0 {
                    let is_neg = ctx.b.ins().icmp_imm(IntCC::SignedLessThan, l_bits, 0);
                    let fast_blk = ctx.create_block();
                    let slow_blk = ctx.create_block();
                    let merge_blk = ctx.create_block();
                    
                    let res_val_var = ctx.b.declare_var(types::I64);
                    ctx.b.ins().brif(is_neg, slow_blk, &[], fast_blk, &[]);
                    
                    ctx.b.switch_to_block(fast_blk);
                    let fast_val = ctx.b.ins().band_imm(l_bits, divisor - 1);
                    ctx.b.def_var(res_val_var, fast_val);
                    ctx.b.ins().jump(merge_blk, &[]);
                    
                    ctx.b.switch_to_block(slow_blk);
                    let slow_val = ctx.b.ins().srem(l_bits, r_bits);
                    ctx.b.def_var(res_val_var, slow_val);
                    ctx.b.ins().jump(merge_blk, &[]);
                    
                    ctx.b.switch_to_block(merge_blk);
                    let s = ctx.b.use_var(res_val_var);
                    
                    let cmp_int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
                    
                    if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
                        let (old_bits, old_tag) = ctx.use_local(dst);
                        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
                    }
                    
                    ctx.def_local(dst, s, cmp_int_tag);
                    ctx.clear_block_state(true);
                    return;
                }
            }
        }
        
        let is_zero = ctx.b.ins().icmp_imm(IntCC::Equal, r_bits, 0);
        let is_min  = ctx.b.ins().icmp_imm(IntCC::Equal, l_bits, i64::MIN);
        let is_minus_one = ctx.b.ins().icmp_imm(IntCC::Equal, r_bits, -1);
        let is_overflow = ctx.b.ins().band(is_min, is_minus_one);
        let should_fail_native = ctx.b.ins().bor(is_zero, is_overflow);
        
        let local_slow = ctx.create_block();
        let local_math = ctx.create_block();
        let local_next = ctx.create_block();
        
        let res_bits_var = ctx.b.declare_var(types::I64);
        let res_tag_var  = ctx.b.declare_var(types::I64);
        let cmp_int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);

        ctx.b.ins().brif(should_fail_native, local_slow, &[], local_math, &[]);
        
        ctx.b.switch_to_block(local_math);
        let s = if is_mod {
            ctx.b.ins().srem(l_bits, r_bits)
        } else {
            ctx.b.ins().sdiv(l_bits, r_bits)
        };
        ctx.b.def_var(res_bits_var, s);
        ctx.b.def_var(res_tag_var, cmp_int_tag);
        ctx.b.ins().jump(local_next, &[]);

        ctx.b.switch_to_block(local_slow);
        let (s_bits, s_tag) = ctx.call_ffi_value(ffi_func, &[l_bits, cmp_int_tag, r_bits, cmp_int_tag, ctx.executor_ptr]);
        ctx.emit_halt_if_errors(symbols);
        ctx.b.def_var(res_bits_var, s_bits);
        ctx.b.def_var(res_tag_var, s_tag);
        ctx.b.ins().jump(local_next, &[]);

        ctx.b.switch_to_block(local_next);
        let final_bits = ctx.b.use_var(res_bits_var);
        let final_tag = ctx.b.use_var(res_tag_var);
        
        if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
            let (old_bits, old_tag) = ctx.use_local(dst);
            super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
        }
        
        ctx.def_local(dst, final_bits, final_tag);
        ctx.clear_block_state(true);
        return;
    }

    let (v1_bits, v1_tag) = ctx.use_local(src1);
    let (v2_bits, v2_tag) = ctx.use_local(src2);
    
    let cmp_int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
    let is_int1 = ctx.b.ins().icmp(IntCC::Equal, v1_tag, cmp_int_tag);
    let is_int2 = ctx.b.ins().icmp(IntCC::Equal, v2_tag, cmp_int_tag);
    let both_int = ctx.b.ins().band(is_int1, is_int2);
    
    let fast_path = ctx.create_block();
    let slow_path = ctx.create_block();
    let next_blk  = ctx.create_block();
    
    let res_bits_var = ctx.b.declare_var(types::I64);
    let res_tag_var  = ctx.b.declare_var(types::I64);

    ctx.b.ins().brif(both_int, fast_path, &[], slow_path, &[]);
    
    ctx.b.switch_to_block(fast_path);
    let is_zero = ctx.b.ins().icmp_imm(IntCC::Equal, v2_bits, 0);
    let is_min  = ctx.b.ins().icmp_imm(IntCC::Equal, v1_bits, i64::MIN);
    let is_minus_one = ctx.b.ins().icmp_imm(IntCC::Equal, v2_bits, -1);
    let is_overflow = ctx.b.ins().band(is_min, is_minus_one);
    let should_fail = ctx.b.ins().bor(is_zero, is_overflow);
    
    let do_math = ctx.create_block();
    ctx.b.ins().brif(should_fail, slow_path, &[], do_math, &[]);
    
    ctx.b.switch_to_block(do_math);
    let s = if is_mod {
        let divisor_opt = ctx.register_const[src2 as usize];
        if let Some(divisor) = divisor_opt {
            if divisor > 0 && (divisor & (divisor - 1)) == 0 {
                let is_neg = ctx.b.ins().icmp_imm(IntCC::SignedLessThan, v1_bits, 0);
                let fast_blk = ctx.create_block();
                let slow_blk = ctx.create_block();
                let merge_blk = ctx.create_block();
                
                let res_val_var = ctx.b.declare_var(types::I64);
                ctx.b.ins().brif(is_neg, slow_blk, &[], fast_blk, &[]);
                
                ctx.b.switch_to_block(fast_blk);
                let fast_val = ctx.b.ins().band_imm(v1_bits, divisor - 1);
                ctx.b.def_var(res_val_var, fast_val);
                ctx.b.ins().jump(merge_blk, &[]);
                
                ctx.b.switch_to_block(slow_blk);
                let slow_val = ctx.b.ins().srem(v1_bits, v2_bits);
                ctx.b.def_var(res_val_var, slow_val);
                ctx.b.ins().jump(merge_blk, &[]);
                
                ctx.b.switch_to_block(merge_blk);
                ctx.b.use_var(res_val_var)
            } else {
                ctx.b.ins().srem(v1_bits, v2_bits)
            }
        } else {
            ctx.b.ins().srem(v1_bits, v2_bits)
        }
    } else {
        ctx.b.ins().sdiv(v1_bits, v2_bits)
    };
    ctx.b.def_var(res_bits_var, s);
    ctx.b.def_var(res_tag_var, cmp_int_tag);
    ctx.b.ins().jump(next_blk, &[]);
    
    ctx.b.switch_to_block(slow_path);
    let (s_bits, s_tag) = ctx.call_ffi_value(ffi_func, &[v1_bits, v1_tag, v2_bits, v2_tag, ctx.executor_ptr]);
    ctx.emit_halt_if_errors(symbols);
    ctx.b.def_var(res_bits_var, s_bits);
    ctx.b.def_var(res_tag_var, s_tag);
    ctx.b.ins().jump(next_blk, &[]);
    
    ctx.b.switch_to_block(next_blk);
    let final_bits = ctx.b.use_var(res_bits_var);
    let final_tag = ctx.b.use_var(res_tag_var);

    if ctx.uses_heap && !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        super::nan_ops::emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, final_bits, final_tag);
    ctx.clear_block_state(true);
}

pub fn emit_div_poly(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    emit_poly_div_mod_fast_path(ctx, symbols, dst, src1, src2, symbols.xcx_jit_div, false);
}

pub fn emit_mod_poly(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src1: u8, src2: u8) {
    emit_poly_div_mod_fast_path(ctx, symbols, dst, src1, src2, symbols.xcx_jit_mod, true);
}

pub fn emit_neg_poly(ctx: &mut CodegenCtx, symbols: &super::symbols::ImportedSymbols, dst: u8, src: u8) {
    let (v_bits, v_tag) = ctx.use_local(src);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_neg, &[v_bits, v_tag]);
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_poly_cmp_fast_path(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    ffi_func: FuncRef,
    cc: IntCC,
    cc_raw: u8,
) {
    let (v1_bits, v1_tag) = ctx.use_local(src1);
    let (v2_bits, v2_tag) = ctx.use_local(src2);
    
    let cmp_int_tag = ctx.b.ins().iconst(types::I64, TAG_INT as i64);
    let is_int1 = ctx.b.ins().icmp(IntCC::Equal, v1_tag, cmp_int_tag);
    let is_int2 = ctx.b.ins().icmp(IntCC::Equal, v2_tag, cmp_int_tag);
    let both_int = ctx.b.ins().band(is_int1, is_int2);
    
    let int_fast_path = ctx.create_block();
    let check_float = ctx.create_block();
    let float_fast_path = ctx.create_block();
    let slow_path = ctx.create_block();
    let next_blk  = ctx.create_block();
    let res_var = ctx.b.declare_var(types::I64);
    let bool_tag_var = ctx.b.declare_var(types::I64);
    let bool_tag = ctx.b.ins().iconst(types::I64, TAG_BOOL as i64);
    
    ctx.b.ins().brif(both_int, int_fast_path, &[], check_float, &[]);
    
    ctx.b.switch_to_block(check_float);
    let cmp_float_tag = ctx.b.ins().iconst(types::I64, TAG_FLOAT as i64);
    let is_float1 = ctx.b.ins().icmp(IntCC::Equal, v1_tag, cmp_float_tag);
    let is_float2 = ctx.b.ins().icmp(IntCC::Equal, v2_tag, cmp_float_tag);
    let both_float = ctx.b.ins().band(is_float1, is_float2);
    ctx.b.ins().brif(both_float, float_fast_path, &[], slow_path, &[]);
    
    ctx.b.switch_to_block(int_fast_path);
    let cmp = ctx.b.ins().icmp(cc, v1_bits, v2_bits);
    let ext_res = ctx.b.ins().uextend(types::I64, cmp);
    ctx.b.def_var(res_var, ext_res);
    ctx.b.def_var(bool_tag_var, bool_tag);
    ctx.b.ins().jump(next_blk, &[]);
    
    ctx.b.switch_to_block(float_fast_path);
    let l_f64 = ctx.b.ins().bitcast(types::F64, MemFlags::new(), v1_bits);
    let r_f64 = ctx.b.ins().bitcast(types::F64, MemFlags::new(), v2_bits);
    let float_cc = super::abi::decode_floatcc(cc_raw);
    let cmp_f = ctx.b.ins().fcmp(float_cc, l_f64, r_f64);
    let ext_res_f = ctx.b.ins().uextend(types::I64, cmp_f);
    ctx.b.def_var(res_var, ext_res_f);
    ctx.b.def_var(bool_tag_var, bool_tag);
    ctx.b.ins().jump(next_blk, &[]);
    
    ctx.b.switch_to_block(slow_path);
    let (res_bits, res_tag) = ctx.call_ffi_value(ffi_func, &[v1_bits, v1_tag, v2_bits, v2_tag]);
    ctx.b.def_var(res_var, res_bits);
    ctx.b.def_var(bool_tag_var, res_tag);
    ctx.b.ins().jump(next_blk, &[]);
    
    ctx.b.switch_to_block(next_blk);
    ctx.clear_block_state(true);
    let final_res = ctx.b.use_var(res_var);
    let final_tag = ctx.b.use_var(bool_tag_var);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, final_res, final_tag);
}

pub fn emit_cmp_poly(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src1: u8,
    src2: u8,
    cc_raw: u8,
) {
    let (ffi_func, cc) = match cc_raw {
        0 => (symbols.xcx_jit_eq, IntCC::Equal),
        1 => (symbols.xcx_jit_ne, IntCC::NotEqual),
        2 => (symbols.xcx_jit_gt, IntCC::SignedGreaterThan),
        3 => (symbols.xcx_jit_lt, IntCC::SignedLessThan),
        4 => (symbols.xcx_jit_ge, IntCC::SignedGreaterThanOrEqual),
        _ => (symbols.xcx_jit_le, IntCC::SignedLessThanOrEqual),
    };
    emit_poly_cmp_fast_path(ctx, symbols, dst, src1, src2, ffi_func, cc, cc_raw);
}
