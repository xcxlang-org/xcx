use crate::vm::value::Value;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_size(set_bits: u64, set_tag: u64) -> i64 {
    let set = Value { bits: set_bits, tag: set_tag };
    if !set.is_set() { return 0; }
    let arc = set.as_set();
    arc.read().elements.len() as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_contains(set_bits: u64, set_tag: u64, val_bits: u64, val_tag: u64) -> bool {
    let set = Value { bits: set_bits, tag: set_tag };
    let val = Value { bits: val_bits, tag: val_tag };
    if !set.is_set() { return false; }
    let arc = set.as_set();
    arc.read().elements.contains(&val)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_remove(set_bits: u64, set_tag: u64, val_bits: u64, val_tag: u64) {
    let set = Value { bits: set_bits, tag: set_tag };
    let val = Value { bits: val_bits, tag: val_tag };
    if !set.is_set() { return; }
    let arc = set.as_set();
    let mut set_data = arc.write();
    if set_data.elements.remove(&val) {
        set_data.cache = None;
        if let Some(arr) = set_data.cached_arr.take() {
            unsafe { arr.dec_ref(); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_init(
    out: *mut Value,
    _exec: *mut crate::vm::core::executor::Executor,
    elements_ptr: *const Value,
    count: u32,
) {
    let mut elements = std::collections::BTreeSet::new();
    if !elements_ptr.is_null() && count > 0 {
        let slice = unsafe { std::slice::from_raw_parts(elements_ptr, count as usize) };
        for val in slice {
            unsafe { val.inc_ref(); }
            elements.insert(*val);
        }
    }
    unsafe { *out = Value::from_set(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::SetObj::new(elements)))); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_union(
    out: *mut Value,
    s1_bits: u64,
    s1_tag: u64,
    s2_bits: u64,
    s2_tag: u64,
) {
    let s1_val = Value { bits: s1_bits, tag: s1_tag };
    let s2_val = Value { bits: s2_bits, tag: s2_tag };
    if !s1_val.is_set() || !s2_val.is_set() {
        unsafe { *out = Value::from_bool(false); }
        return;
    }
    let s1 = s1_val.as_set();
    let s2 = s2_val.as_set();
    let s1_rd = s1.read();
    let s2_rd = s2.read();
    let mut elements = std::collections::BTreeSet::new();
    for v in s1_rd.elements.iter().chain(s2_rd.elements.iter()) {
        if elements.insert(*v) {
            unsafe { v.inc_ref(); }
        }
    }
    let res = Value::from_set(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::SetObj::new(elements))));
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_intersection(
    out: *mut Value,
    s1_bits: u64,
    s1_tag: u64,
    s2_bits: u64,
    s2_tag: u64,
) {
    let s1_val = Value { bits: s1_bits, tag: s1_tag };
    let s2_val = Value { bits: s2_bits, tag: s2_tag };
    if !s1_val.is_set() || !s2_val.is_set() { unsafe { *out = Value::from_bool(false); } return; }
    let s1 = s1_val.as_set();
    let s2 = s2_val.as_set();
    let s1_rd = s1.read();
    let s2_rd = s2.read();
    let mut elements = std::collections::BTreeSet::new();
    for v in s1_rd.elements.intersection(&s2_rd.elements) {
        unsafe { v.inc_ref(); }
        elements.insert(*v);
    }
    let res = Value::from_set(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::SetObj::new(elements))));
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_difference(
    out: *mut Value,
    s1_bits: u64,
    s1_tag: u64,
    s2_bits: u64,
    s2_tag: u64,
) {
    let s1_val = Value { bits: s1_bits, tag: s1_tag };
    let s2_val = Value { bits: s2_bits, tag: s2_tag };
    if !s1_val.is_set() || !s2_val.is_set() { unsafe { *out = Value::from_bool(false); } return; }
    let s1 = s1_val.as_set();
    let s2 = s2_val.as_set();
    let s1_rd = s1.read();
    let s2_rd = s2.read();
    let mut elements = std::collections::BTreeSet::new();
    for v in s1_rd.elements.difference(&s2_rd.elements) {
        unsafe { v.inc_ref(); }
        elements.insert(*v);
    }
    let res = Value::from_set(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::SetObj::new(elements))));
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_sym_difference(
    out: *mut Value,
    s1_bits: u64,
    s1_tag: u64,
    s2_bits: u64,
    s2_tag: u64,
) {
    let s1_val = Value { bits: s1_bits, tag: s1_tag };
    let s2_val = Value { bits: s2_bits, tag: s2_tag };
    if !s1_val.is_set() || !s2_val.is_set() { unsafe { *out = Value::from_bool(false); } return; }
    let s1 = s1_val.as_set();
    let s2 = s2_val.as_set();
    let s1_rd = s1.read();
    let s2_rd = s2.read();
    let mut elements = std::collections::BTreeSet::new();
    for v in s1_rd.elements.symmetric_difference(&s2_rd.elements) {
        unsafe { v.inc_ref(); }
        elements.insert(*v);
    }
    let res = Value::from_set(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::SetObj::new(elements))));
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_values(out: *mut Value, set_bits: u64, set_tag: u64) {
    let set = Value { bits: set_bits, tag: set_tag };
    if !set.is_set() {
        unsafe { *out = Value { bits: 0, tag: 0 }; }
        return;
    }
    let set_rc = set.as_set();
    {
        let cached = set_rc.read().cached_arr;
        if let Some(arr_val) = cached {
            unsafe {
                arr_val.inc_ref();
                *out = arr_val;
            }
            return;
        }
    }
    let mut set_data = set_rc.write();
    if let Some(arr_val) = set_data.cached_arr {
        unsafe {
            arr_val.inc_ref();
            *out = arr_val;
        }
        return;
    }
    let mut arr = Vec::with_capacity(set_data.elements.len());
    for v in set_data.elements.iter() {
        unsafe { v.inc_ref(); }
        arr.push(*v);
    }
    let res = Value::from_array(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::ArrayObj::new(arr))));
    unsafe {
        res.inc_ref();
        set_data.cached_arr = Some(res);
        *out = res;
    }
}


