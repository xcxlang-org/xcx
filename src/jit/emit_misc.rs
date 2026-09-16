use cranelift::prelude::*;
use super::codegen_ctx::CodegenCtx;
use super::symbols::ImportedSymbols;

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
        // Frame recovery: the aborted frame's out slot must carry the declared
        // return type's default so the caller's destination register observes
        // a value of the declared type, never an arbitrary leftover.
        let (def_bits, def_tag) = match ctx.ret_default {
            Some(d) => (d.bits, d.tag),
            None => (0, crate::vm::value::TAG_BOOL),
        };
        let def_bits_v = ctx.b.ins().iconst(types::I64, def_bits as i64);
        let def_tag_v = ctx.b.ins().iconst(types::I64, def_tag as i64);
        super::nan_ops::emit_conditional_inc_ref(ctx, symbols, def_bits_v, def_tag_v);
        ctx.b.ins().store(super::abi::trusted(), def_bits_v, out_ptr, 0);
        ctx.b.ins().store(super::abi::trusted(), def_tag_v, out_ptr, 8);
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
