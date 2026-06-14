/// NaN-boxing constants retained for float serialization and legacy reference.
/// All non-float Values use the new 16-byte { bits: u64, tag: u8 } scheme.
/// These are no longer used for tag detection — use the `tag` field of Value.

// --- Tag discriminants (u8) ---
pub const TAG_FLOAT:   u64 = 0;
pub const TAG_INT:     u64 = 1;
pub const TAG_BOOL:    u64 = 2;
pub const TAG_DATE:    u64 = 3;
pub const TAG_STR:     u64 = 4;
pub const TAG_ARR:     u64 = 5;
pub const TAG_SET:     u64 = 6;
pub const TAG_MAP:     u64 = 7;
pub const TAG_TBL:     u64 = 8;
pub const TAG_FUNC:    u64 = 9;
pub const TAG_ROW:     u64 = 10;
pub const TAG_JSON:    u64 = 11;
pub const TAG_FIB:     u64 = 12;
pub const TAG_DB:      u64 = 13;
pub const TAG_CLOSURE: u64 = 14;
pub const TAG_ARENA:   u64 = 15;
pub const TAG_FUNC_PTR: u64 = 16;


/// Minimum tag value that indicates a heap pointer.
pub const TAG_FIRST_PTR: u64 = TAG_STR;

/// Used when packing floats. Any double that looks like a NaN in the QNAN space
/// of the old encoding is remapped to a canonical NaN so that old serialized
/// constant pools round-trip correctly.
pub const QNAN_BASE: u64 = 0x7FF0_0000_0000_0000;
pub const QNAN_CANONICAL: u64 = 0x7FF8_0000_0000_0001;

/// Safely pack an f64 value into its u64 bit representation.
/// Any bit pattern that falls in the QNAN_BASE range (which the old NaN-boxing
/// scheme used for tagged values) is remapped to a canonical quiet NaN.
#[inline]
pub fn pack_float_bits(f: f64) -> u64 {
    let b = f.to_bits();
    if (b & QNAN_BASE) == QNAN_BASE && b != f64::INFINITY.to_bits() && b != f64::NEG_INFINITY.to_bits() {
        // This is a NaN — canonicalize to avoid confusion
        QNAN_CANONICAL
    } else {
        b
    }
}

#[inline]
pub fn unpack_float_bits(bits: u64) -> f64 {
    f64::from_bits(bits)
}
