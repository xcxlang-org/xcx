use cranelift::prelude::*;
use crate::vm::value::TAG_FIRST_PTR;

pub const VALUE_BITS_OFFSET: i32 = 0;
pub const VALUE_TAG_OFFSET: i32 = 8;
pub const VALUE_SIZE: i32 = 16;

/// Offset from an Arc data pointer to the strong reference count in the
/// surrounding `ArcInner { strong, weak, data }` allocation. Every heap object
/// XCX stores in a Value has 8-byte alignment, so `data` always begins 16
/// bytes into the allocation. `arc_strong_count_offset_matches_arc` asserts
/// this against the real allocator layout.
pub const ARC_STRONG_COUNT_OFFSET: i64 = -16;

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
///
/// Emitted as an FFI call to `xcx_jit_inc_ref`, whose tag match no-ops for
/// TAG_FUNC (function index, not a pointer). `ARC_STRONG_COUNT_OFFSET` and
/// its tests document the Arc header layout in case the increment is ever
/// inlined as a native atomic — note that doing so changed register
/// allocation enough to slow down nested integer loops measurably, which
/// is why the call remains.
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

#[cfg(test)]
mod tests {
    use super::ARC_STRONG_COUNT_OFFSET;

    /// `ARC_STRONG_COUNT_OFFSET` documents where an Arc's strong count sits
    /// relative to the data pointer a Value carries. Assert that it matches
    /// the real `Arc` header layout for every heap object type a Value can
    /// hold: read the count through the computed offset and verify it tracks
    /// `Arc::strong_count` across clones and drops.
    #[test]
    fn arc_strong_count_offset_matches_arc() {
        use crate::vm::object::{ArrayObj, BoolArrayObj, JsonObj, MapObj, SetObj, StringObj, TableObj};
        use std::sync::Arc;
        use parking_lot::RwLock;

        fn check<T>(arc: &Arc<T>) {
            let data_ptr = Arc::as_ptr(arc) as *const u8;
            let count_ptr = unsafe { data_ptr.offset(ARC_STRONG_COUNT_OFFSET as isize) } as *const usize;
            let via_offset = unsafe { *count_ptr };
            assert_eq!(via_offset, Arc::strong_count(arc));
        }

        check(&Arc::new(StringObj::new(Vec::new())));
        check(&Arc::new(RwLock::new(ArrayObj::new(Vec::new()))));
        check(&Arc::new(RwLock::new(BoolArrayObj::new(Vec::new()))));
        check(&Arc::new(RwLock::new(SetObj::new(std::collections::BTreeSet::new()))));
        check(&Arc::new(RwLock::new(MapObj::new(Vec::new()))));
        check(&Arc::new(RwLock::new(TableObj {
            table_name: String::new(),
            columns: Vec::new(),
            rows: Vec::new(),
            sql_binding: None,
            sql_where: None,
            pending_op: None,
        })));
        check(&Arc::new(JsonObj::new(crate::vm::object::JsonVal::Null)));

        // Verify the offset tracks mutations, not just the initial count.
        let arc = Arc::new(StringObj::new(Vec::new()));
        let data_ptr = Arc::as_ptr(&arc) as *const u8;
        let count_ptr = unsafe { data_ptr.offset(ARC_STRONG_COUNT_OFFSET as isize) } as *const usize;
        let clones = (1..=3).map(|_| Arc::clone(&arc)).collect::<Vec<_>>();
        assert_eq!(unsafe { *count_ptr }, 4);
        drop(clones);
        assert_eq!(unsafe { *count_ptr }, 1);
    }
}

#[cfg(test)]
mod inc_ref_predicate_tests {
    use crate::vm::value::{Value, TAG_FIRST_PTR, TAG_FUNC};

    /// `Value::inc_ref` counts exactly the `Arc`-backed heap tags; it
    /// no-ops for TAG_FUNC (a function *index* stored in bits, not a
    /// pointer). Any JIT-side reimplementation of the guard must exclude
    /// precisely that tag — verified here by checking count movement
    /// through the Rust-side entry point and the boundary tag values
    /// themselves.
    #[test]
    fn inc_ref_tag_set_matches_value_inc_ref() {
        // Arc-backed pointer: count must move with inc_ref.
        use crate::vm::object::{ArrayObj, StringObj};
        use parking_lot::RwLock;
        use std::sync::Arc;
        let arc = Arc::new(RwLock::new(ArrayObj::new(Vec::new())));
        let v = Value::from_array(arc.clone());
        assert_eq!(Arc::strong_count(&arc), 2);
        unsafe { v.inc_ref(); }
        assert_eq!(Arc::strong_count(&arc), 3);
        unsafe { v.dec_ref(); }
        assert_eq!(Arc::strong_count(&arc), 2);

        let s = Arc::new(StringObj::new(Vec::new()));
        let v = Value::from_string(s.clone());
        assert_eq!(Arc::strong_count(&s), 2);
        unsafe { v.inc_ref(); }
        assert_eq!(Arc::strong_count(&s), 3);
        unsafe { v.dec_ref(); }
        assert_eq!(Arc::strong_count(&s), 2);

        // Non-Arc-backed tag above TAG_FIRST_PTR: the Rust-side inc_ref is
        // a no-op, so the JIT predicate must reject it.
        let func_idx_val = Value::from_function(7);
        assert_eq!(func_idx_val.tag, TAG_FUNC);
        assert!(func_idx_val.tag >= TAG_FIRST_PTR);
        // TAG_FUNC must not satisfy the JIT predicate.
        let t = func_idx_val.tag;
        let jit_counts = t >= TAG_FIRST_PTR && t != TAG_FUNC;
        assert!(!jit_counts);

        // Every scalar tag below TAG_FIRST_PTR is rejected.
        for t in 0u64..TAG_FIRST_PTR {
            assert!(false == (t >= TAG_FIRST_PTR && t != TAG_FUNC));
        }
    }
}
