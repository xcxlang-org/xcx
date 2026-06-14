use crate::vm::value::{Value, TAG_ARR, TAG_SET, TAG_MAP};

pub use crate::runtime::builtin::math::random::{xcx_jit_random_int, xcx_jit_random_float};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_random_choice(out: *mut Value, col_bits: u64, col_tag: u64) {
    let col = Value { bits: col_bits, tag: col_tag };
    if col.is_ptr() {
        let mut rng = rand::rng();
        match col.tag {
            TAG_ARR => {
                let arc = col.as_array();
                let arr = arc.read();
                if arr.elements.is_empty() { unsafe { *out = Value::from_bool(false); } }
                else { 
                    let v = arr.elements[rand::Rng::random_range(&mut rng, 0..arr.elements.len())]; 
                    if v.is_ptr() { unsafe { v.inc_ref(); } }
                    unsafe { *out = v; }
                }
            }
            TAG_SET => {
                let arc = col.as_set();
                let mut s_write = arc.write();
                if s_write.cache.is_none() {
                    s_write.cache = Some(s_write.elements.iter().cloned().collect());
                }
                let cache = s_write.cache.as_ref().unwrap();
                if cache.is_empty() { unsafe { *out = Value::from_bool(false); } }
                else { 
                    let v = cache[rand::Rng::random_range(&mut rng, 0..cache.len())]; 
                    if v.is_ptr() { unsafe { v.inc_ref(); } }
                    unsafe { *out = v; }
                }
            }
            TAG_MAP => {
                let arc = col.as_map();
                let map = arc.read();
                if map.elements.is_empty() { unsafe { *out = Value::from_bool(false); } }
                else {
                    let (k, _) = map.elements[rand::Rng::random_range(&mut rng, 0..map.elements.len())];
                    if k.is_ptr() { unsafe { k.inc_ref(); } }
                    unsafe { *out = k; }
                }
            }
            _ => unsafe { *out = Value::from_bool(false); }
        }
    } else {
        unsafe { *out = Value::from_bool(false); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_range(
    out: *mut Value,
    min_bits: u64, min_tag: u64,
    max_bits: u64, max_tag: u64,
    step_bits: u64, step_tag: u64,
    has_step: bool
) {
    let v_s = Value { bits: min_bits, tag: min_tag };
    let v_e = Value { bits: max_bits, tag: max_tag };
    let v_step_val = Value { bits: step_bits, tag: step_tag };

    let mut elements = std::collections::BTreeSet::new();

    if v_s.is_int() && v_e.is_int() {
        let v_start = v_s.as_i64();
        let v_end = v_e.as_i64();
        let v_step = if has_step { v_step_val.as_i64() } else { 1 };
        if v_step > 0 {
            let mut curr = v_start;
            while curr <= v_end {
                let val = Value::from_i64(curr);
                if !elements.insert(val) { unsafe { val.dec_ref(); } }
                curr += v_step;
            }
        } else if v_step < 0 {
            let mut curr = v_start;
            while curr >= v_end {
                let val = Value::from_i64(curr);
                if !elements.insert(val) { unsafe { val.dec_ref(); } }
                curr += v_step;
            }
        }
    } else if v_s.is_float() || v_e.is_float() {
        let v_start = v_s.cast_float();
        let v_end = v_e.cast_float();
        let v_step = if has_step { v_step_val.cast_float() } else { 1.0 };
        if v_step > 0.0 {
            let mut curr = v_start;
            while curr <= v_end + 1e-12 {
                let val = Value::from_f64(curr);
                if !elements.insert(val) { unsafe { val.dec_ref(); } }
                curr += v_step;
            }
        } else if v_step < 0.0 {
            let mut curr = v_start;
            while curr >= v_end - 1e-12 {
                let val = Value::from_f64(curr);
                if !elements.insert(val) { unsafe { val.dec_ref(); } }
                curr += v_step;
            }
        }
    } else if v_s.is_string() && v_e.is_string() {
        let s_start = v_s.as_string();
        let s_end = v_e.as_string();
        if s_start.data.len() == 1 && s_end.data.len() == 1 {
            let v_start = s_start.data[0] as i64;
            let v_end = s_end.data[0] as i64;
            let v_step = if has_step { v_step_val.as_i64() } else { 1 };
            if v_step > 0 {
                let mut curr = v_start;
                while curr <= v_end {
                    let v = Value::from_string(std::sync::Arc::new(crate::vm::object::StringObj::new(vec![curr as u8])));
                    if !elements.insert(v) { unsafe { v.dec_ref(); } }
                    curr += v_step;
                }
            } else if v_step < 0 {
                let mut curr = v_start;
                while curr >= v_end {
                    let v = Value::from_string(std::sync::Arc::new(crate::vm::object::StringObj::new(vec![curr as u8])));
                    if !elements.insert(v) { unsafe { v.dec_ref(); } }
                    curr += v_step;
                }
            }
        }
    }

    let res = Value::from_set(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::SetObj::new(elements))));
    unsafe { *out = res; }
}
