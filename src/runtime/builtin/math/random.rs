use rand::Rng;
use crate::vm::value::Value;

#[unsafe(no_mangle)]
pub extern "C" fn xcx_jit_random_int(
    out: *mut Value,
    min_bits: u64, min_tag: u64,
    max_bits: u64, max_tag: u64,
    step_bits: u64, step_tag: u64,
    has_step: bool
) {
    let min_val = Value { bits: min_bits, tag: min_tag };
    let max_val = Value { bits: max_bits, tag: max_tag };
    let step_val = Value { bits: step_bits, tag: step_tag };
    
    let min = min_val.as_i64();
    let max = max_val.as_i64();
    let step = step_val.as_i64();

    let mut rng = rand::rng();
    let diff = max - min;
    let abs_diff = diff.abs();
    let abs_step = if has_step { step.abs().max(1) } else { 1 };
    let steps = abs_diff / abs_step;
    let k = rng.random_range(0..=steps);
    let sign = if diff >= 0 { 1 } else { -1 };
    
    let res = min + k * sign * abs_step;
    unsafe { *out = Value::from_i64(res); }
}

#[unsafe(no_mangle)]
pub extern "C" fn xcx_jit_random_float(
    out: *mut Value,
    min_bits: u64, min_tag: u64,
    max_bits: u64, max_tag: u64,
    step_bits: u64, step_tag: u64,
    has_step: bool
) {
    let min_val = Value { bits: min_bits, tag: min_tag };
    let max_val = Value { bits: max_bits, tag: max_tag };
    let step_val = Value { bits: step_bits, tag: step_tag };
    
    let min = min_val.as_f64();
    let max = max_val.as_f64();
    let step = step_val.as_f64();

    let mut rng = rand::rng();
    let diff = max - min;
    let abs_diff = diff.abs();
    let abs_step = if has_step { step.abs() } else { 0.5 };
    
    let res = if abs_step > 0.0 {
        let steps = (abs_diff / abs_step).floor() as i64;
        let k = rng.random_range(0..=steps);
        let sign = if diff >= 0.0 { 1.0 } else { -1.0 };
        min + (k as f64) * sign * abs_step
    } else {
        let t: f64 = rng.random();
        min + t * diff
    };
    
    unsafe { *out = Value::from_f64(res); }
}
