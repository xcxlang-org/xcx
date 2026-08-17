pub use super::tag::{
    TAG_FLOAT, TAG_INT, TAG_BOOL, TAG_DATE, TAG_STR, TAG_ARR, TAG_SET,
    TAG_MAP, TAG_TBL, TAG_FUNC, TAG_ROW, TAG_JSON, TAG_FIB, TAG_DB,
    TAG_FIRST_PTR, TAG_FUNC_PTR,
};

pub const QNAN_BASE: u64 = 0x7FF0_0000_0000_0000;
pub const QNAN_CANONICAL: u64 = 0x7FF8_0000_0000_0001;

#[inline]
pub fn pack_float_bits(f: f64) -> u64 {
    let b = f.to_bits();
    if (b & QNAN_BASE) == QNAN_BASE && b != f64::INFINITY.to_bits() && b != f64::NEG_INFINITY.to_bits() {
        QNAN_CANONICAL
    } else {
        b
    }
}

#[inline]
pub fn unpack_float_bits(bits: u64) -> f64 {
    f64::from_bits(bits)
}
