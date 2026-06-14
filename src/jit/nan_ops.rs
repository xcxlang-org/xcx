use cranelift::prelude::*;
use crate::vm::value::TAG_FIRST_PTR;

pub const VALUE_BITS_OFFSET: i32 = 0;
pub const VALUE_TAG_OFFSET: i32 = 8;
pub const VALUE_SIZE: i32 = 16;

/// Build a packed quiet-NaN Value from a raw i64 integer bits.
pub fn make_int_nanboxed(b: &mut FunctionBuilder, raw_i64: Value) -> Value {
    let mask_48 = b.ins().iconst(types::I64, 0x0000_FFFF_FFFF_FFFF);
    let payload = b.ins().band(raw_i64, mask_48);
    let prefix = b.ins().iconst(types::I64, 0x7FF1_0000_0000_0000 as i64);
    b.ins().bor(prefix, payload)
}

/// Build a packed quiet-NaN Value from a 0/1 boolean bit.
pub fn make_bool_nanboxed(b: &mut FunctionBuilder, bit: Value) -> Value {
    let prefix = b.ins().iconst(types::I64, 0x7FF2_0000_0000_0000 as i64);
    b.ins().bor(prefix, bit)
}

/// Build a float packed Value from an F64 value (bitcast directly).
pub fn make_float_nanboxed(b: &mut FunctionBuilder, raw_f64: Value) -> Value {
    b.ins().bitcast(types::I64, MemFlags::new(), raw_f64)
}

/// Extract raw i64 from a packed Integer with sign extension.
pub fn unpack_int(b: &mut FunctionBuilder, val: Value) -> Value {
    let shl = b.ins().ishl_imm(val, 16);
    b.ins().sshr_imm(shl, 16)
}

/// Extract boolean status (0 / 1) from quiet-NaN boolean Value.
pub fn unpack_bool(b: &mut FunctionBuilder, val: Value) -> Value {
    b.ins().band_imm(val, 1)
}

/// Cast a JIT packed float value to F64.
pub fn unpack_float(b: &mut FunctionBuilder, val: Value) -> Value {
    b.ins().bitcast(types::F64, MemFlags::new(), val)
}

/// Get 48-bit pointer payload of a tagged pointer value.
pub fn unpack_ptr(b: &mut FunctionBuilder, val: Value) -> Value {
    let mask_48 = b.ins().iconst(types::I64, 0x0000_FFFF_FFFF_FFFF);
    b.ins().band(val, mask_48)
}

/// Extract tag dynamically from quiet-NaN boxed Value.
pub fn get_tag(b: &mut FunctionBuilder, val: Value) -> Value {
    let mask_nan = b.ins().iconst(types::I64, 0xFFF0_0000_0000_0000u64 as i64);
    let high_bits = b.ins().band(val, mask_nan);
    let qnan_mark = b.ins().iconst(types::I64, 0x7FF0_0000_0000_0000u64 as i64);
    let is_tagged = b.ins().icmp(IntCC::Equal, high_bits, qnan_mark);
    
    let raw_tag = b.ins().ushr_imm(val, 48);
    let ext_tag = b.ins().band_imm(raw_tag, 0xF);
    
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().select(is_tagged, ext_tag, zero)
}

/// Extract payload bits dynamically.
pub fn get_bits(b: &mut FunctionBuilder, val: Value) -> Value {
    let mask_nan = b.ins().iconst(types::I64, 0xFFF0_0000_0000_0000u64 as i64);
    let high_bits = b.ins().band(val, mask_nan);
    let qnan_mark = b.ins().iconst(types::I64, 0x7FF0_0000_0000_0000u64 as i64);
    let is_tagged = b.ins().icmp(IntCC::Equal, high_bits, qnan_mark);
    
    let mask_48 = b.ins().iconst(types::I64, 0x0000_FFFF_FFFF_FFFF);
    let payload = b.ins().band(val, mask_48);
    
    b.ins().select(is_tagged, payload, val)
}

/// Build a packed Value dynamically from (bits, tag) Cranelift Values.
pub fn pack_value(b: &mut FunctionBuilder, bits: Value, tag: Value) -> Value {
    let tag_shift = b.ins().ishl_imm(tag, 48);
    let mask_48 = b.ins().iconst(types::I64, 0x0000_FFFF_FFFF_FFFF);
    let payload = b.ins().band(bits, mask_48);
    let tagged = b.ins().bor(tag_shift, payload);
    let nan_base = b.ins().iconst(types::I64, 0x7FF0_0000_0000_0000u64 as i64);
    let tagged_val = b.ins().bor(tagged, nan_base);
    
    let zero = b.ins().iconst(types::I64, 0);
    let is_nonzero = b.ins().icmp(IntCC::NotEqual, tag, zero);
    b.ins().select(is_nonzero, tagged_val, bits)
}

/// Unpack quiet-NaN to (bits, tag) dynamic Cranelift Value pair.
pub fn unpack_value(b: &mut FunctionBuilder, val: Value) -> (Value, Value) {
    let tag = get_tag(b, val);
    let bits = get_bits(b, val);
    (bits, tag)
}

/// Fast pointer identification check directly on unpacked tag: tag >= TAG_FIRST_PTR.
pub fn emit_is_ptr_tag(b: &mut FunctionBuilder, tag: Value) -> Value {
    let threshold = b.ins().iconst(types::I64, TAG_FIRST_PTR as i64);
    b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, tag, threshold)
}

/// Conditional dec_ref of dynamic JIT Value using bits and tag.
pub fn emit_conditional_dec_ref(
    ctx: &mut super::codegen_ctx::CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    bits: Value,
    tag: Value,
) {
    if !ctx.uses_heap { return; }
    let is_ptr = emit_is_ptr_tag(ctx.b, tag);

    let dec_blk  = ctx.create_block();
    let next_blk = ctx.create_block();

    ctx.b.ins().brif(is_ptr, dec_blk, &[], next_blk, &[]);

    ctx.b.switch_to_block(dec_blk);
    ctx.b.ins().call(symbols.xcx_jit_dec_ref, &[bits, tag]);
    ctx.b.ins().jump(next_blk, &[]);

    ctx.b.switch_to_block(next_blk);
}

/// Conditional inc_ref of dynamic JIT Value using bits and tag.
pub fn emit_conditional_inc_ref(
    ctx: &mut super::codegen_ctx::CodegenCtx,
    symbols: &super::symbols::ImportedSymbols,
    bits: Value,
    tag: Value,
) {
    if !ctx.uses_heap { return; }
    let is_ptr = emit_is_ptr_tag(ctx.b, tag);

    let inc_blk  = ctx.create_block();
    let next_blk = ctx.create_block();

    ctx.b.ins().brif(is_ptr, inc_blk, &[], next_blk, &[]);

    ctx.b.switch_to_block(inc_blk);
    ctx.b.ins().call(symbols.xcx_jit_inc_ref, &[bits, tag]);
    ctx.b.ins().jump(next_blk, &[]);

    ctx.b.switch_to_block(next_blk);
}
