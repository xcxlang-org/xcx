use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::object::StringObj;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_upper(out: *mut Value, bits: u64, _tag: u64) {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    let s_str = String::from_utf8_lossy(&s_arc.data);
    let res_arc = Arc::new(StringObj::new(s_str.to_uppercase().into_bytes()));
    unsafe { *out = Value::from_string(res_arc); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_lower(out: *mut Value, bits: u64, _tag: u64) {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    let s_str = String::from_utf8_lossy(&s_arc.data);
    let res_arc = Arc::new(StringObj::new(s_str.to_lowercase().into_bytes()));
    unsafe { *out = Value::from_string(res_arc); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_trim(out: *mut Value, bits: u64, _tag: u64) {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    let s_str = String::from_utf8_lossy(&s_arc.data);
    let res_arc = Arc::new(StringObj::new(s_str.trim().to_string().into_bytes()));
    unsafe { *out = Value::from_string(res_arc); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_slice(out: *mut Value, bits: u64, _tag: u64, start: i64, end: i64) {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    let s_str = String::from_utf8_lossy(&s_arc.data);
    let chars: Vec<char> = s_str.chars().collect();
    let len = chars.len() as i64;
    
    if start < 0 || end > len || start > end {
        // Provide empty string fallback in JIT if bounds check fails to avoid crashing
        let empty = Arc::new(StringObj::new(Vec::new()));
        unsafe { *out = Value::from_string(empty); }
        return;
    }
    
    let res_str: String = chars[start as usize..end as usize].iter().collect();
    let res_arc = Arc::new(StringObj::new(res_str.into_bytes()));
    unsafe { *out = Value::from_string(res_arc); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_replace(out: *mut Value, bits: u64, _tag: u64, f_bits: u64, f_tag: u64, t_bits: u64, t_tag: u64) {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    let f_ext = crate::vm::value::heap_object::as_string(&Value { bits: f_bits, tag: f_tag });
    let t_ext = crate::vm::value::heap_object::as_string(&Value { bits: t_bits, tag: t_tag });
    
    let base = String::from_utf8_lossy(&s_arc.data);
    let from = String::from_utf8_lossy(&f_ext.data);
    let to   = String::from_utf8_lossy(&t_ext.data);
    
    let res_arc = Arc::new(StringObj::new(base.replace(from.as_ref(), to.as_ref()).into_bytes()));
    unsafe { *out = Value::from_string(res_arc); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_index_of(bits: u64, _tag: u64, f_bits: u64, f_tag: u64) -> i64 {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    let f_ext = crate::vm::value::heap_object::as_string(&Value { bits: f_bits, tag: f_tag });
    
    let base = String::from_utf8_lossy(&s_arc.data);
    let from = String::from_utf8_lossy(&f_ext.data);
    
    match base.find(from.as_ref()) {
        Some(idx) => {
            // Find counts by bytes, we need char index for XCX
            let char_idx = base[..idx].chars().count() as i64;
            char_idx
        },
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_last_index_of(bits: u64, _tag: u64, f_bits: u64, f_tag: u64) -> i64 {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    let f_ext = crate::vm::value::heap_object::as_string(&Value { bits: f_bits, tag: f_tag });
    
    let base = String::from_utf8_lossy(&s_arc.data);
    let from = String::from_utf8_lossy(&f_ext.data);
    
    match base.rfind(from.as_ref()) {
        Some(idx) => {
            let char_idx = base[..idx].chars().count() as i64;
            char_idx
        },
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_to_int(bits: u64, _tag: u64) -> i64 {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    match String::from_utf8_lossy(&s_arc.data).parse::<i64>() {
        Ok(v) => v,
        Err(_) => panic!("halt.error:Cannot parse string to integer"),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_to_float(bits: u64, _tag: u64) -> f64 {
    let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
    match String::from_utf8_lossy(&s_arc.data).parse::<f64>() {
        Ok(v) => v,
        Err(_) => panic!("halt.error:Cannot parse string to float"),
    }
}


