use cranelift::prelude::*;
use super::codegen_ctx::CodegenCtx;
use super::symbols::ImportedSymbols;
use super::nan_ops::emit_conditional_dec_ref;
use crate::vm::value::Value as VMValue;

pub fn emit_store_read(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    base: u8,
) {
    let (p_bits, p_tag) = ctx.use_local(base);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_store_read, &[p_bits, p_tag]);
    ctx.emit_halt_if_errors(symbols);
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_store_write(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    base: u8,
) {
    let (p_bits, p_tag) = ctx.use_local(base);
    let (c_bits, c_tag) = ctx.use_local(base + 1);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_store_write, &[p_bits, p_tag, c_bits, c_tag]);
    ctx.emit_halt_if_errors(symbols);
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_store_append(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    base: u8,
) {
    let (p_bits, p_tag) = ctx.use_local(base);
    let (c_bits, c_tag) = ctx.use_local(base + 1);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_store_append, &[p_bits, p_tag, c_bits, c_tag]);
    ctx.emit_halt_if_errors(symbols);
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_store_exists(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    base: u8,
) {
    let (p_bits, p_tag) = ctx.use_local(base);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_store_exists, &[p_bits, p_tag]);
    ctx.emit_halt_if_errors(symbols);
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_store_delete(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    base: u8,
) {
    let (p_bits, p_tag) = ctx.use_local(base);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_store_delete, &[p_bits, p_tag]);
    ctx.emit_halt_if_errors(symbols);
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

// NOTE: database_init uses special FFI signature handling. Leaving out_ptr assumption out for this special case unless it actually returns a Value! 
// Wait, database_init returns a string error or true. It returns a Value! So it DOES use call_ffi_value!
pub fn emit_database_init(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    engine_src: u8,
    path_src: u8,
    tables_base_reg: u8,
    table_count: u32,
) {
    ctx.spill_all();
    let (e_bits, e_tag) = ctx.use_local(engine_src);
    let (p_bits, p_tag) = ctx.use_local(path_src);
    let tb = ctx.b.ins().iconst(types::I32, tables_base_reg as i64);
    let tc = ctx.b.ins().iconst(types::I32, table_count as i64);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_database_init, &[
        e_bits, e_tag, p_bits, p_tag, ctx.locals_ptr, tb, tc, ctx.executor_ptr
    ]);
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_get_member(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    container: u8,
    name_idx: u32,
    constants: &[VMValue],
) {
    let name_val = constants[name_idx as usize];
    let name_arc = name_val.as_string();
    let name_ptr = name_arc.data.as_ptr() as i64;
    let name_len = name_arc.data.len() as i64;
    
    let np = ctx.b.ins().iconst(types::I64, name_ptr);
    let nl = ctx.b.ins().iconst(types::I64, name_len);
    let (c_bits, c_tag) = ctx.use_local(container);
    
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_get_member, &[c_bits, c_tag, np, nl]);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_set_member(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    container: u8,
    name_idx: u32,
    src: u8,
    constants: &[VMValue],
) {
    let name_val = constants[name_idx as usize];
    let name_arc = name_val.as_string();
    let name_ptr = name_arc.data.as_ptr() as i64;
    let name_len = name_arc.data.len() as i64;
    
    let (c_bits, c_tag) = ctx.use_local(container);
    let (s_bits, s_tag) = ctx.use_local(src);
    let np = ctx.b.ins().iconst(types::I64, name_ptr);
    let nl = ctx.b.ins().iconst(types::I64, name_len);
    ctx.b.ins().call(symbols.xcx_jit_set_member, &[c_bits, c_tag, np, nl, s_bits, s_tag]);
}

pub fn emit_str_append_member(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    container: u8,
    name_idx: u32,
    src: u8,
    constants: &[VMValue],
) {
    let name_val = constants[name_idx as usize];
    let name_arc = name_val.as_string();
    let name_ptr = name_arc.data.as_ptr() as i64;
    let name_len = name_arc.data.len() as i64;
    
    let (c_bits, c_tag) = ctx.use_local(container);
    let (s_bits, s_tag) = ctx.use_local(src);
    let np = ctx.b.ins().iconst(types::I64, name_ptr);
    let nl = ctx.b.ins().iconst(types::I64, name_len);
    ctx.b.ins().call(symbols.xcx_jit_str_append_member, &[c_bits, c_tag, np, nl, s_bits, s_tag]);
}

pub fn emit_str_append_element(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    container: u8,
    index: u8,
    src: u8,
) {
    let (c_bits, c_tag) = ctx.use_local(container);
    let (idx_bits, _idx_tag) = ctx.use_local(index);
    let (s_bits, s_tag) = ctx.use_local(src);
    ctx.b.ins().call(symbols.xcx_jit_str_append_element, &[c_bits, c_tag, idx_bits, s_bits, s_tag]);
}

