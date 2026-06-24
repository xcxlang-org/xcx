use crate::vm::value::Value;
use crate::vm::core::executor::Executor;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_size(arr_bits: u64, _arr_tag: u64) -> i64 {
    let rlock_ptr = arr_bits as *const parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    arr_ref.elements.len() as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_get(out: *mut Value, arr_bits: u64, _arr_tag: u64, idx_bits: u64) {
    let rlock_ptr = arr_bits as *const parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let idx = idx_bits as usize;
    if idx < arr_ref.elements.len() {
        let v = arr_ref.elements[idx];
        if v.is_ptr() { unsafe { v.inc_ref(); } }
        unsafe { *out = v; }
    } else {
        crate::runtime::builtin::io::eprint_buffered(&format!("R303: Array index out of bounds: {} (Array length: {})\n", idx, arr_ref.elements.len()));
        crate::vm::core::vm::increment_error_count();
        unsafe { *out = Value::from_i64(0); }
    }
}

/// Returns the bits field of a boolean array element directly as i64 (0 or 1).
/// Avoids the output-pointer/inc_ref path used by xcx_jit_array_get for bool-typed reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_get_bool(arr_bits: u64, _arr_tag: u64, idx_bits: u64) -> i64 {
    let rlock_ptr = arr_bits as *const parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let idx = idx_bits as usize;
    if idx < arr_ref.elements.len() {
        unsafe { arr_ref.elements.get_unchecked(idx).bits as i64 }
    } else {
        crate::runtime::builtin::io::eprint_buffered(&format!("R303: Array index out of bounds: {} (Array length: {})\n", idx, arr_ref.elements.len()));
        crate::vm::core::vm::increment_error_count();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_push(arr_bits: u64, _arr_tag: u64, val_bits: u64, val_tag: u64) {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    let val = Value { bits: val_bits, tag: val_tag };
    if val.is_ptr() { unsafe { val.inc_ref(); } }
    arr_ref.elements.push(val);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_update(arr_bits: u64, _arr_tag: u64, idx_bits: u64, val_bits: u64, val_tag: u64) -> i32 {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    let idx = idx_bits as usize;
    if idx < arr_ref.elements.len() {
        let old = arr_ref.elements[idx];
        let val = Value { bits: val_bits, tag: val_tag };
        if val.is_ptr() { unsafe { val.inc_ref(); } }
        arr_ref.elements[idx] = val;
        if old.is_ptr() { unsafe { old.dec_ref(); } }
        1
    } else {
        crate::runtime::builtin::io::eprint_buffered(&format!("R303: Array update index out of bounds: {} (Array length: {})\n", idx, arr_ref.elements.len()));
        crate::vm::core::vm::increment_error_count();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_set_bool(arr_bits: u64, _arr_tag: u64, idx_bits: u64, val: u8) -> i32 {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    let idx = idx_bits as usize;
    if idx < arr_ref.elements.len() {
        *unsafe { arr_ref.elements.get_unchecked_mut(idx) } = Value::from_bool(val != 0);
        1
    } else {
        crate::runtime::builtin::io::eprint_buffered(&format!("R303: Array update index out of bounds: {} (Array length: {})\n", idx, arr_ref.elements.len()));
        crate::vm::core::vm::increment_error_count();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_init(
    out: *mut Value,
    _exec: *mut Executor,
    elements_ptr: *const Value,
    count: u32,
) {
    let elements = if elements_ptr.is_null() || count == 0 {
        Vec::new()
    } else {
        let slice = unsafe { std::slice::from_raw_parts(elements_ptr, count as usize) };
        for val in slice {
            unsafe { val.inc_ref(); }
        }
        slice.to_vec()
    };
    unsafe { *out = Value::from_array(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::ArrayObj::new(elements)))); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_get_int(arr_bits: u64, _arr_tag: u64, idx_bits: u64) -> i64 {
    let rlock_ptr = arr_bits as *const parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let idx = idx_bits as usize;
    if idx < arr_ref.elements.len() {
        unsafe { arr_ref.elements.get_unchecked(idx).bits as i64 }
    } else {
        crate::runtime::builtin::io::eprint_buffered(&format!("R303: Array index out of bounds: {} (Array length: {})\n", idx, arr_ref.elements.len()));
        crate::vm::core::vm::increment_error_count();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_set_int(arr_bits: u64, _arr_tag: u64, idx_bits: u64, val: i64) -> i32 {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    let idx = idx_bits as usize;
    if idx < arr_ref.elements.len() {
        unsafe { *arr_ref.elements.get_unchecked_mut(idx) = Value::from_i64(val); }
        1
    } else {
        crate::runtime::builtin::io::eprint_buffered(&format!("R303: Array update index out of bounds: {} (Array length: {})\n", idx, arr_ref.elements.len()));
        crate::vm::core::vm::increment_error_count();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_pop(out: *mut Value, arr_bits: u64, _arr_tag: u64) {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    let res = arr_ref.elements.pop().unwrap_or(Value::from_bool(false));
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_clear(arr_bits: u64, _arr_tag: u64) {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    for v in arr_ref.elements.drain(..) { unsafe { v.dec_ref(); } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_is_empty(arr_bits: u64, _arr_tag: u64) -> i64 {
    let rlock_ptr = arr_bits as *const parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    if arr_ref.elements.is_empty() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_contains(arr_bits: u64, _arr_tag: u64, v_bits: u64, v_tag: u64) -> i64 {
    let rlock_ptr = arr_bits as *const parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let val = Value { bits: v_bits, tag: v_tag };
    if arr_ref.elements.contains(&val) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_find(out: *mut Value, arr_bits: u64, _arr_tag: u64, v_bits: u64, v_tag: u64) {
    let rlock_ptr = arr_bits as *const parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let val = Value { bits: v_bits, tag: v_tag };
    let idx = arr_ref.elements.iter().position(|v| v == &val).map(|i| i as i64).unwrap_or(-1);
    unsafe { *out = Value::from_i64(idx); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_insert(arr_bits: u64, _arr_tag: u64, idx: i64, val_bits: u64, val_tag: u64) -> i32 {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    let val = Value { bits: val_bits, tag: val_tag };
    if idx >= 0 && (idx as usize) <= arr_ref.elements.len() {
        if val.is_ptr() { unsafe { val.inc_ref(); } }
        arr_ref.elements.insert(idx as usize, val);
        1
    } else {
        crate::runtime::builtin::io::eprint_buffered(&format!("R303: Array insert index out of bounds: {} (Array length: {})\n", idx, arr_ref.elements.len()));
        crate::vm::core::vm::increment_error_count();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_delete(arr_bits: u64, _arr_tag: u64, idx: i64) -> i32 {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    if idx >= 0 && (idx as usize) < arr_ref.elements.len() {
        let old = arr_ref.elements.remove(idx as usize);
        unsafe { old.dec_ref(); }
        1
    } else {
        crate::runtime::builtin::io::eprint_buffered(&format!("R303: Array delete index out of bounds: {} (Array length: {})\n", idx, arr_ref.elements.len()));
        crate::vm::core::vm::increment_error_count();
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_sort(arr_bits: u64, _arr_tag: u64) -> i64 {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    arr_ref.elements.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_array_reverse(arr_bits: u64, _arr_tag: u64) -> i64 {
    let rlock_ptr = arr_bits as *mut parking_lot::RwLock<crate::vm::object::ArrayObj>;
    let arr_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    arr_ref.elements.reverse();
    1
}

