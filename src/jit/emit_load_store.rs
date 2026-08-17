use cranelift::prelude::*;
use super::codegen_ctx::CodegenCtx;
use super::nan_ops::{emit_conditional_dec_ref, emit_conditional_inc_ref};

pub fn emit_load_const(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    val_idx: u32,
    constants: &[crate::vm::value::Value],
) {
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }

    let val = constants[val_idx as usize];
    if val.is_int() || val.is_bool() || val.is_float() {
        let bits = ctx.b.ins().iconst(types::I64, val.bits as i64);
        let tag  = ctx.b.ins().iconst(types::I64, val.tag as i64);
        ctx.def_local(dst, bits, tag);
        if val.is_int() {
            ctx.register_const[dst as usize] = Some(val.as_i64());
            ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::Int;
        } else if val.is_bool() {
            ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::Bool;
        } else if val.is_float() {
            ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::Float;
        }
    } else {
        let (c_bits, c_tag) = ctx.load_const(val_idx);
        emit_conditional_inc_ref(ctx, symbols, c_bits, c_tag);
        ctx.def_local(dst, c_bits, c_tag);
        if val.is_string() {
            ctx.known_types[dst as usize] = crate::vm::opcode::TypeTag::String;
        }
    }


}

pub fn emit_move(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    src: u8,
) {
    let (sv_bits, sv_tag) = ctx.use_local(src);

    if !ctx.is_known_non_ptr(src as usize) && !ctx.reg_is_never_ptr(src as usize) {
        emit_conditional_inc_ref(ctx, symbols, sv_bits, sv_tag);
    }

    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }

    let c = ctx.register_const[src as usize];
    let ty = ctx.known_types[src as usize];
    ctx.def_local(dst, sv_bits, sv_tag);
    ctx.register_const[dst as usize] = c;
    ctx.known_types[dst as usize] = ty;
}



pub fn emit_get_var(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    idx: u32,
    elide_inc: bool,
) {
    let (g_bits, g_tag) = ctx.use_global(idx);
    let global_is_int = ctx.global_is_int(idx);

    // `elide_inc` is only set when a following specialized MethodCall
    // consumes this register as receiver and result. Update/Set/Push never
    // release the receiver (the inc would leak — see `getvar_inc_elidable`);
    // Get branches release it via their dst-dec_ref, which is skipped
    // through `unowned_recv_regs` instead. Either way no inc is needed.
    if !global_is_int && !elide_inc {
        emit_conditional_inc_ref(ctx, symbols, g_bits, g_tag);
    }

    if !ctx.should_skip_dec_ref(dst) && !global_is_int {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }

    ctx.def_local(dst, g_bits, g_tag);

    // Must be set after def_local — def_local clears the borrow bit for the
    // redefined register. The bit then survives until the consumer
    // MethodCall defines the register again.
    if elide_inc {
        ctx.unowned_recv_regs[dst as usize] = true;
    }
}

pub fn emit_set_var(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    idx: u32,
    src: u8,
) {
    let (lv_bits, lv_tag) = ctx.use_local(src);
    let global_is_int = ctx.global_is_int(idx);
    
    if !global_is_int {
        emit_conditional_inc_ref(ctx, symbols, lv_bits, lv_tag);
        let (old_bits, old_tag) = ctx.use_global(idx);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }

    ctx.def_global(idx, lv_bits, lv_tag);
}

pub fn emit_inc_var(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    idx: u32,
) {
    let (old_bits, old_tag) = ctx.use_global(idx);
    let next = ctx.b.ins().iadd_imm(old_bits, 1);
    ctx.def_global(idx, next, old_tag);
}

pub fn emit_dec_var(
    ctx: &mut CodegenCtx,
    _symbols: &super::symbols::ImportedSymbols,
    idx: u32,
) {
    let (old_bits, old_tag) = ctx.use_global(idx);
    let next = ctx.b.ins().iadd_imm(old_bits, -1);
    ctx.def_global(idx, next, old_tag);
}

pub fn emit_inc_local(
    ctx: &mut CodegenCtx,
    reg: u8,
) {
    let (v_bits, v_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(v_bits, 1);
    ctx.def_local(reg, next, v_tag);
}

pub fn emit_dec_local(
    ctx: &mut CodegenCtx,
    reg: u8,
) {
    let (v_bits, v_tag) = ctx.use_local(reg);
    let next = ctx.b.ins().iadd_imm(v_bits, -1);
    ctx.def_local(reg, next, v_tag);
}

pub fn emit_row_get(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    row_reg: u8,
    col_idx: u16,
) {
    let (rv_bits, rv_tag) = ctx.use_local(row_reg);

    let idx_val = ctx.b.ins().iconst(types::I32, col_idx as i64);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_row_get, &[rv_bits, rv_tag, idx_val]);

    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }

    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_table_push_row(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    tbl_reg: u8,
    row_reg: u8,
) {
    let (tv_bits, tv_tag) = ctx.use_local(tbl_reg);
    let (rv_bits, rv_tag) = ctx.use_local(row_reg);
    
    ctx.b.ins().call(symbols.xcx_jit_table_push_row, &[tv_bits, tv_tag, rv_bits, rv_tag]);
}

pub fn emit_table_clone_skeleton(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst_reg: u8,
    src_reg: u8,
) {
    let (sv_bits, sv_tag) = ctx.use_local(src_reg);
    
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_table_clone_skeleton, &[sv_bits, sv_tag]);
    
    if !ctx.should_skip_dec_ref(dst_reg) {
        let (old_bits, old_tag) = ctx.use_local(dst_reg);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    
    ctx.def_local(dst_reg, res_bits, res_tag);
}

pub fn emit_json_bind_local(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    dst: u8,
    json_reg: u8,
    path_reg: u8,
) {
    let (jv_bits, jv_tag) = ctx.use_local(json_reg);
    let (pv_bits, pv_tag) = ctx.use_local(path_reg);

    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_json_bind, &[jv_bits, jv_tag, pv_bits, pv_tag]);

    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }

    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_json_bind_global(
    ctx: &mut CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    idx: u32,
    json_reg: u8,
    path_reg: u8,
) {
    let (jv_bits, jv_tag) = ctx.use_local(json_reg);
    let (pv_bits, pv_tag) = ctx.use_local(path_reg);

    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_json_bind, &[jv_bits, jv_tag, pv_bits, pv_tag]);

    let (old_bits, old_tag) = ctx.use_global(idx);
    emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);

    ctx.def_global(idx, res_bits, res_tag);
}
