use crate::vm::value::Value;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_size(map_bits: u64, _map_tag: u64) -> i64 {
    let rlock_ptr = map_bits as *const parking_lot::RwLock<crate::vm::object::MapObj>;
    let map_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    map_ref.elements.len() as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_contains(map_bits: u64, _map_tag: u64, key_bits: u64, key_tag: u64) -> i32 {
    let rlock_ptr = map_bits as *const parking_lot::RwLock<crate::vm::object::MapObj>;
    let map_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let key = Value { bits: key_bits, tag: key_tag };
    
    let mut found = false;
    for (k, _) in map_ref.elements.iter() {
        if *k == key {
            found = true;
            break;
        }
    }
    if found { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_get(out: *mut Value, map_bits: u64, _map_tag: u64, key_bits: u64, key_tag: u64) {
    let rlock_ptr = map_bits as *const parking_lot::RwLock<crate::vm::object::MapObj>;
    let map_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let key = Value { bits: key_bits, tag: key_tag };
    
    for (k, v) in map_ref.elements.iter() {
        if *k == key {
            if v.is_ptr() { unsafe { v.inc_ref(); } }
            unsafe { *out = *v; }
            return;
        }
    }
    
    unsafe { *out = Value::from_i64(0); } // nil equivalent
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_insert(map_bits: u64, _map_tag: u64, key_bits: u64, key_tag: u64, val_bits: u64, val_tag: u64) -> i32 {
    let rlock_ptr = map_bits as *mut parking_lot::RwLock<crate::vm::object::MapObj>;
    let map_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    
    let key = Value { bits: key_bits, tag: key_tag };
    let val = Value { bits: val_bits, tag: val_tag };
    
    let mut found = false;
    for (k, v) in map_ref.elements.iter_mut() {
        if *k == key {
            if val.is_ptr() { unsafe { val.inc_ref(); } }
            if v.is_ptr() { unsafe { v.dec_ref(); } }
            *v = val;
            found = true;
            break;
        }
    }
    
    if !found {
        if key.is_ptr() { unsafe { key.inc_ref(); } }
        if val.is_ptr() { unsafe { val.inc_ref(); } }
        map_ref.elements.push((key, val));
    }
    
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_remove(map_bits: u64, _map_tag: u64, key_bits: u64, key_tag: u64) -> i32 {
    let rlock_ptr = map_bits as *mut parking_lot::RwLock<crate::vm::object::MapObj>;
    let map_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    let key = Value { bits: key_bits, tag: key_tag };
    
    let before = map_ref.elements.len();
    if let Some(pos) = map_ref.elements.iter().position(|(k, _)| *k == key) {
        let (k, v) = map_ref.elements.remove(pos);
        unsafe {
            k.dec_ref();
            v.dec_ref();
        }
    }
    if map_ref.elements.len() < before { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_clear(map_bits: u64, _map_tag: u64) -> i32 {
    let rlock_ptr = map_bits as *mut parking_lot::RwLock<crate::vm::object::MapObj>;
    let map_ref = unsafe { &mut *(*rlock_ptr).data_ptr() };
    for (k, v) in map_ref.elements.drain(..) {
        unsafe {
            k.dec_ref();
            v.dec_ref();
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_keys(out: *mut Value, map_bits: u64, _map_tag: u64) {
    let rlock_ptr = map_bits as *const parking_lot::RwLock<crate::vm::object::MapObj>;
    let map_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let mut keys = Vec::new();
    for (k, _) in map_ref.elements.iter() {
        if k.is_ptr() { unsafe { k.inc_ref(); } }
        keys.push(*k);
    }
    let array_obj = crate::vm::object::ArrayObj::new(keys);
    let cell = std::sync::Arc::new(parking_lot::RwLock::new(array_obj));
    unsafe {
        *out = Value::from_array(cell);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_values(out: *mut Value, map_bits: u64, _map_tag: u64) {
    let rlock_ptr = map_bits as *const parking_lot::RwLock<crate::vm::object::MapObj>;
    let map_ref = unsafe { &*(*rlock_ptr).data_ptr() };
    let mut vals = Vec::new();
    for (_, v) in map_ref.elements.iter() {
        if v.is_ptr() { unsafe { v.inc_ref(); } }
        vals.push(*v);
    }
    let array_obj = crate::vm::object::ArrayObj::new(vals);
    let cell = std::sync::Arc::new(parking_lot::RwLock::new(array_obj));
    unsafe {
        *out = Value::from_array(cell);
    }
}
