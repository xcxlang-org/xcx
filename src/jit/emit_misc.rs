use cranelift::prelude::*;
use super::codegen_ctx::CodegenCtx;
use super::symbols::ImportedSymbols;
use super::nan_ops::emit_conditional_dec_ref;

pub fn emit_halt_alert(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    src: u8,
) {
    let (sv_bits, sv_tag) = ctx.use_local(src);
    ctx.b.ins().call(symbols.xcx_jit_halt_alert, &[sv_bits, sv_tag]);
}

pub fn emit_halt_error(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    src: u8,
    terminated: &mut bool,
) {
    let (sv_bits, sv_tag) = ctx.use_local(src);
    ctx.spill_all();
    ctx.b.ins().call(symbols.xcx_jit_halt_error, &[ctx.executor_ptr, sv_bits, sv_tag]);
    if ctx.is_inner_func {
        let status = ctx.b.ins().iconst(types::I32, 1);
        let out_ptr = ctx.out_ptr;
        let false_bits = ctx.b.ins().iconst(types::I64, 0);
        let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
        ctx.b.ins().store(super::abi::trusted(), false_bits, out_ptr, 0);
        ctx.b.ins().store(super::abi::trusted(), false_tag, out_ptr, 8);
        ctx.b.ins().return_(&[status]);
    } else if ctx.b.func.signature.returns.is_empty() {
        ctx.b.ins().return_(&[]);
    } else {
        let ret_ty = ctx.b.func.signature.returns[0].value_type;
        let rv = if ret_ty == types::I64 {
            ctx.b.ins().iconst(types::I64, 1)
        } else {
            ctx.b.ins().iconst(types::I32, 1)
        };
        ctx.b.ins().return_(&[rv]);
    }
    *terminated = true;
}

pub fn emit_halt_fatal(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    src: u8,
    terminated: &mut bool,
) {
    let (sv_bits, sv_tag) = ctx.use_local(src);
    ctx.b.ins().call(symbols.xcx_jit_halt_fatal, &[sv_bits, sv_tag]);
    if ctx.is_inner_func {
        let status = ctx.b.ins().iconst(types::I32, 1);
        let out_ptr = ctx.out_ptr;
        let false_bits = ctx.b.ins().iconst(types::I64, 0);
        let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
        ctx.b.ins().store(super::abi::trusted(), false_bits, out_ptr, 0);
        ctx.b.ins().store(super::abi::trusted(), false_tag, out_ptr, 8);
        ctx.b.ins().return_(&[status]);
    } else if ctx.b.func.signature.returns.is_empty() {
        ctx.b.ins().return_(&[]);
    } else {
        let ret_ty = ctx.b.func.signature.returns[0].value_type;
        let rv = if ret_ty == types::I64 {
            ctx.b.ins().iconst(types::I64, 1)
        } else {
            ctx.b.ins().iconst(types::I32, 1)
        };
        ctx.b.ins().return_(&[rv]);
    }
    *terminated = true;
}

pub fn emit_env_get(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
    src: u8,
) {
    let (sv_bits, sv_tag) = ctx.use_local(src);
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_env_get, &[sv_bits, sv_tag]);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}

pub fn emit_env_args(
    ctx: &mut CodegenCtx,
    symbols: &ImportedSymbols,
    dst: u8,
) {
    let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_env_args, &[]);
    
    if !ctx.should_skip_dec_ref(dst) {
        let (old_bits, old_tag) = ctx.use_local(dst);
        emit_conditional_dec_ref(ctx, symbols, old_bits, old_tag);
    }
    ctx.def_local(dst, res_bits, res_tag);
}
