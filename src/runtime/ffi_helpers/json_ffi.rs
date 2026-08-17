use crate::vm::value::{Value};
use crate::runtime::builtin::json::access::normalize_json_path;
use crate::vm::utils::json::value_to_json;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_json_get(out: *mut Value, json_bits: u64, json_tag: u64, path_bits: u64, path_tag: u64) {
    let json_val = Value { bits: json_bits, tag: json_tag };
    if !json_val.is_json() { unsafe { *out = Value::from_bool(false); return; } }
    
    let path_val = Value { bits: path_bits, tag: path_tag };
    let path_borrow = unsafe { path_val.as_str_borrow() };
    let path_temp;
    let path_str = match path_borrow {
        Some(s) => s,
        None => {
            path_temp = path_val.to_string();
            &path_temp
        }
    };
    
    let json_ptr = json_val.unpack_ptr::<crate::vm::object::JsonObj>();

    let is_simple = path_str.bytes().all(|b| b != b'/' && b != b'.' && b != b'[' && b != b']');
    
    let v_opt = if is_simple {
        match unsafe { &(*json_ptr).root } {
            crate::vm::object::JsonVal::Array(a) => {
                if let Ok(idx) = path_str.parse::<usize>() {
                    let a_read = unsafe { &*(*a).data_ptr() };
                    if idx < a_read.len() { Some(a_read[idx].clone()) } else { None }
                } else {
                    None
                }
            }
            crate::vm::object::JsonVal::Object(o) => {
                let o_read = unsafe { &*(*o).data_ptr() };
                o_read.iter().find(|(k, _)| k.as_str() == path_str).map(|(_, v)| v.clone())
            }
            _ => None,
        }
    } else {
        let pointer = normalize_json_path(path_str);
        unsafe { (*json_ptr).root.pointer(&pointer) }
    };
    
    if let Some(v) = v_opt {
        let res = crate::vm::utils::json::json_val_to_value(&v);
        if res.is_ptr() { unsafe { res.inc_ref(); } }
        unsafe { *out = res; }
    } else {
        unsafe { *out = Value::from_bool(false); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_json_set(json_bits: u64, json_tag: u64, path_bits: u64, path_tag: u64, val_bits: u64, val_tag: u64) -> i32 {
    let json_val = Value { bits: json_bits, tag: json_tag };
    if !json_val.is_json() { return 0; }
    
    let path_val = Value { bits: path_bits, tag: path_tag };
    let path_borrow = unsafe { path_val.as_str_borrow() };
    let path_temp;
    let path_str = match path_borrow {
        Some(s) => s,
        None => {
            path_temp = path_val.to_string();
            &path_temp
        }
    };
    
    let json_ptr = json_val.unpack_ptr::<crate::vm::object::JsonObj>() as *mut crate::vm::object::JsonObj;
    
    let val = Value { bits: val_bits, tag: val_tag };
    let is_simple = path_str.bytes().all(|b| b != b'/' && b != b'.' && b != b'[' && b != b']');
    
    unsafe {
        (*json_ptr).version.fetch_add(1, std::sync::atomic::Ordering::Release);
        (*json_ptr).root.make_mutable();
        
        if is_simple {
            if let crate::vm::object::JsonVal::Object(o) = &mut (*json_ptr).root {
                let obj = &mut *(*o).data_ptr();
                if let Some(pos) = obj.iter().position(|(k, _)| k.as_str() == path_str) {
                    obj[pos].1 = value_to_json(&val);
                } else {
                    obj.push((std::sync::Arc::new(path_str.to_string()), value_to_json(&val)));
                }
                return 1;
            }
        }
        let mut root_copy = (*json_ptr).root.clone();
        crate::vm::utils::set_json_value_at_path(&mut root_copy, path_str, value_to_json(&val));
        (*json_ptr).root = root_copy;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_json_push(json_bits: u64, json_tag: u64, val_bits: u64, val_tag: u64) -> i32 {
    let json_val = Value { bits: json_bits, tag: json_tag };
    if !json_val.is_json() { return 0; }

    let json_ptr = json_val.unpack_ptr::<crate::vm::object::JsonObj>() as *mut crate::vm::object::JsonObj;

    let val = Value { bits: val_bits, tag: val_tag };

    unsafe {
        (*json_ptr).version.fetch_add(1, std::sync::atomic::Ordering::Release);
        (*json_ptr).root.make_mutable();
        if let crate::vm::object::JsonVal::Array(a) = &mut (*json_ptr).root {
            let arr = &mut *(*a).data_ptr();
            arr.push(value_to_json(&val));
            1
        } else {
            crate::vm::core::vm::increment_error_count();
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_json_get_push(json_bits: u64, json_tag: u64, path_bits: u64, path_tag: u64, val_bits: u64, val_tag: u64) {
    let json_val = Value { bits: json_bits, tag: json_tag };
    if !json_val.is_json() { return; }
    
    let path_val = Value { bits: path_bits, tag: path_tag };
    let path_borrow = unsafe { path_val.as_str_borrow() };
    let path_temp;
    let path_str = match path_borrow {
        Some(s) => s,
        None => {
            path_temp = path_val.to_string();
            &path_temp
        }
    };
    
    let json_ptr = json_val.unpack_ptr::<crate::vm::object::JsonObj>() as *mut crate::vm::object::JsonObj;
    
    let val = Value { bits: val_bits, tag: val_tag };
    let is_simple = path_str.bytes().all(|b| b != b'/' && b != b'.' && b != b'[' && b != b']');
    
    unsafe {
        (*json_ptr).version.fetch_add(1, std::sync::atomic::Ordering::Release);
        
        if is_simple {
            (*json_ptr).root.make_mutable();
            if let crate::vm::object::JsonVal::Object(o) = &mut (*json_ptr).root {
                let o_write = &mut *(*o).data_ptr();
                if let Some(pos) = o_write.iter().position(|(k, _)| k.as_str() == path_str) {
                    o_write[pos].1.make_mutable();
                    if let crate::vm::object::JsonVal::Array(a) = &mut o_write[pos].1 {
                        let arr = &mut *(*a).data_ptr();
                        arr.push(value_to_json(&val));
                        return;
                    }
                }
            }
        }
        
        let mut root_copy = (*json_ptr).root.clone();
        crate::vm::utils::push_json_value_at_path(&mut root_copy, path_str, value_to_json(&val));
        (*json_ptr).root = root_copy;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_json_to_str(out: *mut Value, json_bits: u64, json_tag: u64) {
    let json_val = Value { bits: json_bits, tag: json_tag };
    if !json_val.is_json() {
        unsafe { *out = Value::from_bool(false); }
        return;
    }

    let json_ptr = json_val.unpack_ptr::<crate::vm::object::JsonObj>();

    unsafe {
        let ver = (*json_ptr).version.load(std::sync::atomic::Ordering::Acquire);
        let cached_ver = (*json_ptr).cached_version.load(std::sync::atomic::Ordering::Acquire);
        if ver == cached_ver {
            if let Some(s) = (*json_ptr).cached_str.lock().as_ref() {
                let res = Value::from_string(s.clone());
                if res.is_ptr() { res.inc_ref(); }
                *out = res;
                return;
            }
        }

        let capacity = match &(*json_ptr).root {
            crate::vm::object::JsonVal::Array(a) => a.read().len() * 64,
            crate::vm::object::JsonVal::Object(o) => o.read().len() * 64,
            _ => 1024,
        };
        let mut buf = String::with_capacity(capacity.max(4096));
        (*json_ptr).root.to_string_buf(&mut buf);

        let string_obj = std::sync::Arc::new(crate::vm::object::StringObj::new(buf.into_bytes()));
        
        let mut lock = (*json_ptr).cached_str.lock();
        if (*json_ptr).version.load(std::sync::atomic::Ordering::Acquire) == ver {
            *lock = Some(string_obj.clone());
            (*json_ptr).cached_version.store(ver, std::sync::atomic::Ordering::Release);
        }
        let res = Value::from_string(string_obj);
        if res.is_ptr() { res.inc_ref(); }
        *out = res;
    }
}

