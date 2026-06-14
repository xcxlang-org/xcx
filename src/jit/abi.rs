use cranelift::prelude::*;
use codegen::ir::MemFlags;
use crate::vm::value::Value as VMValue;

/// JIT-compiled function ABI.
///
/// Parameters (in order):
///   out_ptr      : *mut VMValue  — where to write the return Value (caller-allocated, 16-byte aligned)
///   locals_ptr   : *mut VMValue  — base of the locals array (16 bytes/slot)
///   globals_ptr  : *mut VMValue  — base of the globals array (16 bytes/slot)
///   consts_ptr   : *const VMValue— base of the constants pool (16 bytes/slot)
///   vm_ptr       : *mut VM       — VM instance
///   exec_ptr     : *mut Executor — Executor instance
///   shutdown_ptr : *const bool   — shutdown flag
///
/// The function writes the return value to *out_ptr and returns nothing.
pub type JITFunction = unsafe extern "C" fn(
    *mut VMValue,        // out_ptr
    *mut VMValue,        // locals_ptr
    *mut VMValue,        // globals_ptr
    *const VMValue,      // consts_ptr
    *mut crate::vm::core::vm::VM,
    *mut crate::vm::core::executor::Executor,
    *const bool,
) -> i32;
pub type MethodJitFunction = JITFunction;

#[inline(always)]
pub fn trusted() -> MemFlags {
    let mut f = MemFlags::new();
    f.set_notrap();
    f.set_aligned();
    f
}

#[inline]
pub fn decode_intcc(cc: u8) -> IntCC {
    match cc {
        0 => IntCC::Equal,
        1 => IntCC::NotEqual,
        2 => IntCC::SignedGreaterThan,
        3 => IntCC::SignedLessThan,
        4 => IntCC::SignedGreaterThanOrEqual,
        5 => IntCC::SignedLessThanOrEqual,
        _ => IntCC::Equal,
    }
}

#[inline]
pub fn decode_floatcc(cc: u8) -> FloatCC {
    match cc {
        0 => FloatCC::Equal,
        1 => FloatCC::NotEqual,
        2 => FloatCC::GreaterThan,
        3 => FloatCC::LessThan,
        4 => FloatCC::GreaterThanOrEqual,
        5 => FloatCC::LessThanOrEqual,
        _ => FloatCC::Equal,
    }
}
