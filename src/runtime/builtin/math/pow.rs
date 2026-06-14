use crate::vm::value::Value;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_pow_int(out: *mut Value, a: i64, b: i64) {
    if b < 0 {
        unsafe { *out = Value::from_f64((a as f64).powf(b as f64)); }
    } else {
        unsafe { *out = Value::from_i64(a.pow(b as u32)); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_pow_float(out: *mut Value, a: f64, b: f64) {
    unsafe { *out = Value::from_f64(a.powf(b)); }
}
