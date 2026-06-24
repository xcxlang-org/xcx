use crate::vm::value::Value;
use crate::vm::core::executor::Executor;
use crate::vm::OpResult;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_method_dispatch(
    out: *mut Value,
    receiver_bits: u64,
    receiver_tag: u64,
    kind: u8,
    args_ptr: *const core::ffi::c_void,
    arg_count: u8,
    executor_ptr: *mut Executor,
) {
    let receiver = Value { bits: receiver_bits, tag: receiver_tag };
    let args = if args_ptr.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(args_ptr as *const Value, arg_count as usize) }
    };
    
    let executor = unsafe { &mut *executor_ptr };
    
    match unsafe { executor.dispatch_method(receiver, kind, args, None) } {
        Ok(val) => {
            unsafe { *out = val; }
        }
        Err(_) => {
            executor.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            unsafe { *out = Value::from_i64(0); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_method_dispatch_named(
    out: *mut Value,
    receiver_bits: u64,
    receiver_tag: u64,
    kind: u8,
    args_ptr: *const core::ffi::c_void,
    arg_count: u8,
    names_bits: u64,
    names_tag: u64,
    executor_ptr: *mut Executor,
) {
    let receiver = Value { bits: receiver_bits, tag: receiver_tag };
    let args = if args_ptr.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(args_ptr as *const Value, arg_count as usize) }
    };
    
    let executor = unsafe { &mut *executor_ptr };

    let names_val = Value { bits: names_bits, tag: names_tag };
    let mut names_vec = Vec::new();
    if names_val.is_array() {
        let arr = names_val.as_array();
        let arr_rd = arr.read();
        for v in arr_rd.elements.iter() {
            names_vec.push(v.to_string());
        }
    }
    let names = if names_vec.is_empty() { None } else { Some(names_vec.as_slice()) };
    
    match unsafe { executor.dispatch_method(receiver, kind, args, names) } {
        Ok(val) => {
            unsafe { *out = val; }
        }
        Err(_) => {
            executor.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            unsafe { *out = Value::from_i64(0); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_method_call_custom(
    out: *mut Value,
    receiver_bits: u64,
    receiver_tag: u64,
    name_ptr: *const u8,
    name_len: u32,
    args_ptr: *const core::ffi::c_void,
    arg_count: u8,
    executor_ptr: *mut Executor,
) {
    let receiver = Value { bits: receiver_bits, tag: receiver_tag };
    let method_name = if name_ptr.is_null() {
        String::new()
    } else {
        unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(name_ptr, name_len as usize)).to_string() }
    };
    let args = if args_ptr.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(args_ptr as *const Value, arg_count as usize) }
    };
    let executor = unsafe { &mut *executor_ptr };
    let vm_arc = executor.vm.clone();
    let mut locals = [Value::from_bool(false); 256];
    match executor.handle_method_call_custom(0, receiver, &method_name, args, 0, &mut locals, &vm_arc) {
        OpResult::Continue => {
            unsafe { *out = locals[0]; }
        }
        _ => {
            executor.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            unsafe { *out = Value::from_i64(0); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_starts_with(
    str_bits: u64,
    str_tag: u64,
    pattern_bits: u64,
    pattern_tag: u64,
) -> i8 {
    let s_val = Value { bits: str_bits, tag: str_tag };
    let p_val = Value { bits: pattern_bits, tag: pattern_tag };
    if !s_val.is_string() || !p_val.is_string() { return 0; }
    
    let s_bytes = s_val.as_string();
    let p_bytes = p_val.as_string();
    if s_bytes.data.starts_with(&p_bytes.data) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_ends_with(
    str_bits: u64,
    str_tag: u64,
    pattern_bits: u64,
    pattern_tag: u64,
) -> i8 {
    let s_val = Value { bits: str_bits, tag: str_tag };
    let p_val = Value { bits: pattern_bits, tag: pattern_tag };
    if !s_val.is_string() || !p_val.is_string() { return 0; }
    
    let s_bytes = s_val.as_string();
    let p_bytes = p_val.as_string();
    if s_bytes.data.ends_with(&p_bytes.data) { 1 } else { 0 }
}

