use crate::vm::value::Value;
use crate::vm::VM;
use crate::vm::core::executor::Executor;
use crate::vm::OpResult;
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub use crate::runtime::ffi_helpers::{
    array_ffi::*, set_ffi::*, random_ffi::*, pow_ffi::*, method_ffi::*, fiber_ffi::*
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_inc_ref(bits: u64, tag: u64) {
    let v = Value { bits, tag };
    unsafe { v.inc_ref(); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_dec_ref(bits: u64, tag: u64) {
    let v = Value { bits, tag };
    unsafe { v.dec_ref(); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_str_append_var(
    vm_ptr: *mut VM,
    var_idx: u32,
    src_bits: u64,
    src_tag: u64,
) {
    let vm = unsafe { &*vm_ptr };
    let rhs_val = Value { bits: src_bits, tag: src_tag };
    let rhs_bytes = if rhs_val.tag == crate::vm::value::nan_boxing::TAG_STR {
        let arc = crate::vm::value::heap_object::as_string(&rhs_val);
        arc.data.clone()
    } else {
        rhs_val.to_string().into_bytes()
    };

    let raw = vm.get_global(var_idx as usize);
    if raw.tag == crate::vm::value::nan_boxing::TAG_STR {
        let ptr = raw.bits as *const crate::vm::object::StringObj;
        let arc = unsafe { Arc::from_raw(ptr) };

        let sc = Arc::strong_count(&arc);
        if sc <= 2 {
            let obj_ptr = ptr as *mut crate::vm::object::StringObj;
            unsafe {
                (*obj_ptr).hash = None;
                (*obj_ptr).data.extend_from_slice(&rhs_bytes);
            }
            let bits = Arc::into_raw(arc) as u64;
            let new_val = Value { bits, tag: crate::vm::value::nan_boxing::TAG_STR };
            vm.set_global(var_idx as usize, new_val);
        } else {
            let mut combined = Vec::with_capacity(arc.data.len() + rhs_bytes.len());
            combined.extend_from_slice(&arc.data);
            combined.extend_from_slice(&rhs_bytes);
            std::mem::forget(arc);
            let new_arc = Arc::new(crate::vm::object::StringObj::new(combined));
            let new_val = crate::vm::value::heap_object::from_string(new_arc);
            vm.set_global(var_idx as usize, new_val);
        }
    } else {
        let s1 = raw.to_string();
        let suffix = String::from_utf8_lossy(&rhs_bytes).into_owned();
        let combined = s1 + &suffix;
        let new_val = Value::from_string(Arc::new(crate::vm::object::StringObj::new(combined.into_bytes())));
        vm.set_global(var_idx as usize, new_val);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_str_append_local(
    locals_ptr: *mut Value,
    local_idx: u32,
    src_bits: u64,
    src_tag: u64,
) {
    let locals = unsafe { std::slice::from_raw_parts_mut(locals_ptr, (local_idx + 1) as usize) };
    let rhs_val = Value { bits: src_bits, tag: src_tag };
    let rhs_bytes = if rhs_val.tag == crate::vm::value::nan_boxing::TAG_STR {
        let arc = crate::vm::value::heap_object::as_string(&rhs_val);
        arc.data.clone()
    } else {
        rhs_val.to_string().into_bytes()
    };

    let raw = locals[local_idx as usize];
    if raw.tag == crate::vm::value::nan_boxing::TAG_STR {
        let ptr = raw.bits as *const crate::vm::object::StringObj;
        let arc = unsafe { Arc::from_raw(ptr) };

        let sc = Arc::strong_count(&arc);
        let wc = Arc::weak_count(&arc);
        if sc <= 1 && wc == 0 {
            let obj_ptr = ptr as *mut crate::vm::object::StringObj;
            unsafe {
                (*obj_ptr).hash = None;
                (*obj_ptr).data.extend_from_slice(&rhs_bytes);
            }
            let bits = Arc::into_raw(arc) as u64;
            let new_val = Value { bits, tag: crate::vm::value::nan_boxing::TAG_STR };
            locals[local_idx as usize] = new_val;
        } else {
            let mut combined = Vec::with_capacity(arc.data.len() + rhs_bytes.len());
            combined.extend_from_slice(&arc.data);
            combined.extend_from_slice(&rhs_bytes);
            std::mem::forget(arc);
            let new_arc = Arc::new(crate::vm::object::StringObj::new(combined));
            let new_val = crate::vm::value::heap_object::from_string(new_arc);
            unsafe { new_val.replace_at(&mut locals[local_idx as usize]); }
        }
    } else {
        let s1 = raw.to_string();
        let suffix = String::from_utf8_lossy(&rhs_bytes).into_owned();
        let combined = s1 + &suffix;
        let new_val = Value::from_string(Arc::new(crate::vm::object::StringObj::new(combined.into_bytes())));
        unsafe { new_val.replace_at(&mut locals[local_idx as usize]); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_check_recursion(exec_ptr: *mut Executor) -> u32 {
    let executor = unsafe { &mut *exec_ptr };
    if executor.call_depth >= crate::vm::core::executor::RECURSION_LIMIT {
        crate::runtime::builtin::io::eprint_buffered(&format!(
            "ERROR halt: Recursion limit exceeded ({} frames)\n",
            crate::vm::core::executor::RECURSION_LIMIT
        ));
        executor.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        return 1;
    }
    executor.call_depth += 1;
    return 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_dec_recursion(exec_ptr: *mut Executor) {
    let executor = unsafe { &mut *exec_ptr };
    executor.call_depth -= 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_call_recursive(
    out: *mut Value,
    func_idx: u64,
    args_ptr: *const Value,
    arg_count: u8,
    exec_ptr: *mut Executor,
) -> u32 {
    let executor = unsafe { &mut *exec_ptr };

    let (chunk_max_locals, jit_ptr, uses_heap) = {
        let f = &executor.ctx.functions[func_idx as usize];
        let mut jp = f.jit_ptr.load(Ordering::Acquire);

        if !executor.vm.disable_jit.load(Ordering::Acquire) && jp.is_null() {
            let count = f.call_count.fetch_add(1, Ordering::Relaxed) + 1;
            if count >= 5 {
                let mut jit = executor.vm.jit.lock();
                let func_id_idx = f.bytecode.as_ptr() as usize;
                let name = f.name.clone();
                match jit.compile_method(func_id_idx, func_idx as u32, f, &executor.ctx.constants, &executor.ctx.functions, &name) {
                    Ok(ptr) => {
                        jp = ptr as *mut std::ffi::c_void;
                        f.jit_ptr.store(jp, Ordering::Release);
                    },
                    Err(_e) => {
                        executor.vm.disable_jit.store(true, Ordering::Release);
                    }
                }
            }
        }
        (f.max_locals, jp, f.uses_heap.load(Ordering::Acquire))
    };

    let old_stack_ptr = executor.stack_ptr;
    let locals_start = executor.stack_ptr;
    executor.stack_ptr += chunk_max_locals;

    if executor.call_depth >= crate::vm::core::executor::RECURSION_LIMIT {
        crate::runtime::builtin::io::eprint_buffered(&format!(
            "ERROR halt: Recursion limit exceeded ({} frames)\n",
            crate::vm::core::executor::RECURSION_LIMIT
        ));
        executor.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        executor.stack_ptr = old_stack_ptr;
        unsafe { *out = Value::from_i64(0); }
        return 1;
    }
    executor.call_depth += 1;

    if !jit_ptr.is_null() {
        let locals_ptr = unsafe { executor.stack.as_mut_ptr().add(locals_start) };
        if uses_heap {
            let f = &executor.ctx.functions[func_idx as usize];
            for &i in f.used_locals.iter() {
                if (i as usize) < chunk_max_locals {
                    unsafe { *locals_ptr.add(i as usize) = Value::from_bool(false); }
                }
            }
        }
        if !uses_heap {
            if arg_count > 0 {
                unsafe { std::ptr::copy_nonoverlapping(args_ptr as *const Value, locals_ptr, arg_count as usize); }
            }
        } else {
            for i in 0..arg_count as usize {
                let arg = unsafe { *(args_ptr as *const Value).add(i) };
                if arg.is_ptr() { unsafe { arg.inc_ref(); } }
                unsafe { *locals_ptr.add(i) = arg; }
            }
        }

        let globals_ptr = executor.globals_raw;
        let consts_ptr = executor.ctx.constants.as_ptr() as *const Value;
        let vm_ptr = Arc::as_ptr(&executor.vm) as *mut VM;
        let shutdown_ptr = &crate::vm::core::executor::SHUTDOWN as *const std::sync::atomic::AtomicBool as *const bool;

        let mut out_val = Value { bits: 0, tag: 0 };
        let tmp_out = &mut out_val as *mut Value;

        let jit_fn: crate::jit::abi::JITFunction = unsafe { std::mem::transmute(jit_ptr) };
        let status = unsafe { jit_fn(tmp_out, locals_ptr, globals_ptr, consts_ptr, vm_ptr, executor, shutdown_ptr) };

        let result_val = out_val;
        if result_val.is_ptr() { unsafe { result_val.inc_ref(); } }
        unsafe { *out = result_val; }

        executor.stack_ptr = old_stack_ptr;
        executor.call_depth -= 1;
        return status as u32;
    }

    let args = if args_ptr.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(args_ptr as *const Value, arg_count as usize) }
    };

    let vm_arc = executor.vm.clone();
    let ores: OpResult = {
        let (f_max_locals, f_bytecode, f_used_locals) = {
            let f = &executor.ctx.functions[func_idx as usize];
            (f.max_locals, f.bytecode.clone(), f.used_locals.clone())
        };

        let l_ptr = executor.stack.as_mut_ptr();
        let locals = unsafe { std::slice::from_raw_parts_mut(l_ptr.add(locals_start), f_max_locals) };

        for &i in f_used_locals.iter() {
            if (i as usize) < f_max_locals {
                locals[i as usize] = Value::from_bool(false);
            }
        }

        for (i, arg) in args.iter().enumerate() {
            if i < f_max_locals {
                let v = *arg;
                if v.is_ptr() { unsafe { v.inc_ref(); } }
                locals[i] = v;
            }
        }

        let res = executor.execute_bytecode_inner(&f_bytecode, &mut 0, locals, &vm_arc);

        if let OpResult::Return(Some(v)) = &res {
            if v.is_ptr() { unsafe { v.inc_ref(); } }
        }

        for v in locals { unsafe { v.dec_ref(); } }
        res
    };

    executor.stack_ptr = old_stack_ptr;
    executor.call_depth -= 1;

    match ores {
        OpResult::Return(Some(v)) => {
            unsafe { *out = v; }
            0
        }
        OpResult::Halt => {
            unsafe { *out = Value::from_i64(0); }
            1
        }
        _ => {
            unsafe { *out = Value::from_i64(0); }
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_has(col_bits: u64, col_tag: u64, item_bits: u64, item_tag: u64) -> bool {
    Value { bits: col_bits, tag: col_tag }.has(Value { bits: item_bits, tag: item_tag })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_int_concat(out: *mut Value, a: i64, b: i64) {
    let s = format!("{}{}", a, b);
    let string_obj = crate::vm::object::StringObj::new(s.into_bytes());
    unsafe { *out = Value::from_string(std::sync::Arc::new(string_obj)); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_add(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    let a = Value { bits: a_bits, tag: a_tag };
    let b = Value { bits: b_bits, tag: b_tag };
    let res = a.add(b);
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_sub(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    unsafe { *out = Value { bits: a_bits, tag: a_tag }.sub(Value { bits: b_bits, tag: b_tag }); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_mul(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    unsafe { *out = Value { bits: a_bits, tag: a_tag }.mul(Value { bits: b_bits, tag: b_tag }); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_div(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64, exec_ptr: *mut Executor) {
    match (Value { bits: a_bits, tag: a_tag }).div(Value { bits: b_bits, tag: b_tag }) {
        Ok(res) => unsafe { *out = res; },
        Err(_) => {
            if !exec_ptr.is_null() {
                let exec = unsafe { &mut *exec_ptr };
                crate::runtime::builtin::io::eprint_buffered("ERROR halt: division by zero\n");
                exec.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            unsafe { *out = Value::from_i64(0); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_abort_div(exec_ptr: *mut Executor) {
    if !exec_ptr.is_null() {
        let exec = unsafe { &mut *exec_ptr };
        crate::runtime::builtin::io::eprint_buffered("ERROR halt: division by zero\n");
        exec.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_has_errors(exec_ptr: *mut Executor) -> u32 {
    if exec_ptr.is_null() { return 0; }
    let exec = unsafe { &*exec_ptr };
    if exec.vm.error_count.load(std::sync::atomic::Ordering::Relaxed) > 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_mod(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64, exec_ptr: *mut Executor) {
    match (Value { bits: a_bits, tag: a_tag }).rem(Value { bits: b_bits, tag: b_tag }) {
        Ok(res) => unsafe { *out = res; },
        Err(_) => {
            if !exec_ptr.is_null() {
                let exec = unsafe { &mut *exec_ptr };
                crate::runtime::builtin::io::eprint_buffered("ERROR halt: modulo by zero\n");
                exec.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            unsafe { *out = Value::from_i64(0); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_neg(out: *mut Value, a_bits: u64, a_tag: u64) {
    unsafe { *out = Value { bits: a_bits, tag: a_tag }.neg(); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_eq(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    let cmp = Value { bits: a_bits, tag: a_tag } == Value { bits: b_bits, tag: b_tag };
    unsafe { *out = Value::from_bool(cmp); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_ne(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    let cmp = Value { bits: a_bits, tag: a_tag } != Value { bits: b_bits, tag: b_tag };
    unsafe { *out = Value::from_bool(cmp); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_gt(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    let cmp = Value { bits: a_bits, tag: a_tag } > Value { bits: b_bits, tag: b_tag };
    unsafe { *out = Value::from_bool(cmp); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_lt(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    let cmp = Value { bits: a_bits, tag: a_tag } < Value { bits: b_bits, tag: b_tag };
    unsafe { *out = Value::from_bool(cmp); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_ge(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    let cmp = Value { bits: a_bits, tag: a_tag } >= Value { bits: b_bits, tag: b_tag };
    unsafe { *out = Value::from_bool(cmp); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_le(out: *mut Value, a_bits: u64, a_tag: u64, b_bits: u64, b_tag: u64) {
    let cmp = Value { bits: a_bits, tag: a_tag } <= Value { bits: b_bits, tag: b_tag };
    unsafe { *out = Value::from_bool(cmp); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_cast_string(out: *mut Value, val_bits: u64, val_tag: u64) {
    let v = Value { bits: val_bits, tag: val_tag };
    if v.is_string() {
        unsafe {
            v.inc_ref();
            *out = v;
        }
    } else {
        let s = v.as_string_lossy();
        unsafe { *out = Value::from_string(std::sync::Arc::new(crate::vm::object::StringObj::new(s.into_bytes()))); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_cast_int(out: *mut Value, val_bits: u64, val_tag: u64) {
    let v = Value { bits: val_bits, tag: val_tag };
    unsafe { *out = Value::from_i64(v.cast_int()); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_cast_float(out: *mut Value, val_bits: u64, val_tag: u64) {
    let v = Value { bits: val_bits, tag: val_tag };
    unsafe { *out = Value::from_f64(v.cast_float()); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_cast_bool(out: *mut Value, val_bits: u64, val_tag: u64) {
    let v = Value { bits: val_bits, tag: val_tag };
    unsafe { *out = Value::from_bool(!v.is_bool_false()); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_row_get(out: *mut Value, row_bits: u64, row_tag: u64, col_idx: u32) {
    let v = Value { bits: row_bits, tag: row_tag };
    if !v.is_row() { unsafe { *out = Value::from_i64(0); return; } }
    let row = v.as_row();
    let table = row.table.read();
    if (col_idx as usize) < table.columns.len() {
        let val = table.rows[row.row_idx as usize][col_idx as usize];
        unsafe { val.inc_ref(); }
        unsafe { *out = val; }
    } else {
        unsafe { *out = Value::from_i64(0); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_table_size(table_bits: u64, table_tag: u64) -> i64 {
    let v = Value { bits: table_bits, tag: table_tag };
    if !v.is_table() { return 0; }
    v.as_table().read().rows.len() as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_table_get_row(out: *mut Value, table_bits: u64, table_tag: u64, row_idx: i64) {
    let v = Value { bits: table_bits, tag: table_tag };
    if !v.is_table() { unsafe { *out = Value::from_i64(0); return; } }
    let t_rc = v.as_table();
    unsafe { *out = Value::from_row(std::sync::Arc::new(crate::vm::object::RowObj {
        table: t_rc.clone(),
        row_idx: row_idx as u32,
    })); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_table_push_row(table_bits: u64, table_tag: u64, row_bits: u64, row_tag: u64) {
    let t_val = Value { bits: table_bits, tag: table_tag };
    let r_val = Value { bits: row_bits, tag: row_tag };
    if !t_val.is_table() || !r_val.is_row() { return; }
    
    let t_rc = t_val.as_table();
    let r_obj = r_val.as_row();
    
    let mut table = t_rc.write();
    let r_table = r_obj.table.read();
    let row_data = &r_table.rows[r_obj.row_idx as usize];
    
    let mut row_copy = Vec::with_capacity(row_data.len());
    for v in row_data {
        unsafe { v.inc_ref(); }
        row_copy.push(*v);
    }
    table.rows.push(row_copy);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_table_clone_skeleton(out: *mut Value, src_bits: u64, src_tag: u64) {
    let src_val = Value { bits: src_bits, tag: src_tag };
    if !src_val.is_table() { unsafe { *out = Value::from_bool(false); return; } }
    
    let t_rc = src_val.as_table();
    let t_read = t_rc.read();
    unsafe { *out = Value::from_table(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::TableObj {
        table_name: t_read.table_name.clone(),
        columns: t_read.columns.clone(),
        rows: Vec::new(),
        sql_binding: t_read.sql_binding.clone(),
        sql_where: None,
        pending_op: None,
    }))); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_json_bind(out: *mut Value, json_bits: u64, json_tag: u64, path_bits: u64, path_tag: u64) {
    let json = Value { bits: json_bits, tag: json_tag };
    let path_val = Value { bits: path_bits, tag: path_tag };
    if !path_val.is_string() { unsafe { *out = Value::from_i64(0); return; } }
    let path = path_val.as_string_lossy();
    if let Some(val) = crate::runtime::builtin::json::access::get_path_value_xcx(json, &path) {
        unsafe { val.inc_ref(); }
        unsafe { *out = val; }
    } else {
        unsafe { *out = Value::from_i64(0); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_json_bind_const(out: *mut Value, json_bits: u64, json_tag: u64, path_ptr: *const u8, path_len: u64) {
    let json = Value { bits: json_bits, tag: json_tag };
    let path_bytes = unsafe { std::slice::from_raw_parts(path_ptr, path_len as usize) };
    let path = String::from_utf8_lossy(path_bytes);
    if let Some(val) = crate::runtime::builtin::json::access::get_path_value_xcx(json, &path) {
        unsafe { val.inc_ref(); }
        unsafe { *out = val; }
    } else {
        unsafe { *out = Value::from_i64(0); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_get_member(out: *mut Value, obj_bits: u64, obj_tag: u64, name_ptr: *const u8, name_len: u64) {
    let obj = Value { bits: obj_bits, tag: obj_tag };
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len as usize) };
    let name = String::from_utf8_lossy(name_bytes);
    let res = crate::vm::core::runtime_ops::RuntimeOps::get_member(obj, &name);
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_fiber_state(exec_ptr: *mut Executor, next_ip: u64, is_yield: u32) {
    let ex = unsafe { &mut *exec_ptr };
    ex.fiber_next_ip = next_ip as usize;
    ex.fiber_yielded = is_yield != 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_report_guard_failure(_exec_ptr: *mut Executor, _failing_ip: u64) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_wait(ms: u64) {
    crate::runtime::builtin::io::flush_buffered();
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_dec_ref_range(ptr: *mut Value, count: u32) {
    let slice = unsafe { std::slice::from_raw_parts(ptr, count as usize) };
    for v in slice {
        unsafe { v.dec_ref(); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_length(bits: u64, tag: u64) -> i64 {
    let v = Value { bits, tag: tag };
    if v.is_string() {
        v.as_string().data.len() as i64
    } else {
        0
    }
}

thread_local! {
    static JSON_PARSE_CACHE: std::cell::RefCell<Vec<(usize, crate::vm::object::JsonVal, String, bool)>> = std::cell::RefCell::new(Vec::new());
}
fn is_flat(val: &crate::vm::object::JsonVal) -> bool {
    match val {
        crate::vm::object::JsonVal::Array(_) => false,
        crate::vm::object::JsonVal::Object(o) => {
            let vec = unsafe { &*(*o).data_ptr() };
            for (_, v) in vec.iter() {
                match v {
                    crate::vm::object::JsonVal::Array(_) | crate::vm::object::JsonVal::Object(_) => return false,
                    _ => {}
                }
            }
            true
        }
        _ => true,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_json_parse(out: *mut Value, bits: u64, tag: u64) {
    let v = Value { bits, tag };
    if !v.is_string() { unsafe { *out = Value::from_i64(0); return; } }
    
    let string_ptr = if v.tag == crate::vm::value::nan_boxing::TAG_ARENA {
        crate::vm::value::heap_object::arena_ptr::<crate::vm::object::StringObj>(&v)
    } else {
        v.unpack_ptr::<crate::vm::object::StringObj>()
    };
    
    let key = string_ptr as usize;
    let bytes = unsafe { &(*string_ptr).data };
    let s_str = unsafe { std::str::from_utf8_unchecked(bytes) };

    let cached = JSON_PARSE_CACHE.with(|cache| {
        let c = cache.borrow();
        if let Some(item) = c.iter().find(|(k, _, _, _)| *k == key) {
            return Some((item.1.clone(), item.3));
        }
        if let Some(item) = c.iter().find(|(_, _, s, _)| s == s_str) {
            return Some((item.1.clone(), item.3));
        }
        None
    });

    if let Some((json_val, cached_flat)) = cached {
        let cloned_val = if cached_flat {
            json_val.shallow_clone()
        } else {
            json_val.deep_clone()
        };
        let new_obj = std::sync::Arc::new(crate::vm::object::JsonObj::new(cloned_val));
        unsafe { *out = Value::from_json(new_obj); }
        return;
    }

    let parsed_val = crate::runtime::builtin::json::parse::handle_json_parse(s_str);
    if parsed_val.is_json() {
        let json_obj = parsed_val.as_json();
        let json_val = json_obj.root.clone();
        let s_string = s_str.to_string();
        let flat = is_flat(&json_val);
        JSON_PARSE_CACHE.with(|cache| {
            let mut c = cache.borrow_mut();
            let exists = c.iter().any(|(k, _, _, _)| *k == key) || c.iter().any(|(_, _, s, _)| s == s_str);
            if !exists {
                if c.len() >= 32 {
                    c.remove(0);
                }
                c.push((key, json_val, s_string, flat));
            }
        });
        
        let cloned_val = if flat {
            json_obj.root.shallow_clone()
        } else {
            json_obj.root.deep_clone()
        };
        let new_obj = std::sync::Arc::new(crate::vm::object::JsonObj::new(cloned_val));
        unsafe { *out = Value::from_json(new_obj); }
    } else {
        unsafe { *out = parsed_val; }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_date_now(out: *mut Value) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    unsafe { *out = Value::from_date(now); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_perf_ms(out: *mut Value, vm_ptr: *mut VM) {
    let vm = unsafe { &*vm_ptr };
    let elapsed = vm.start_instant.elapsed().as_millis() as i64;
    unsafe { *out = Value::from_i64(elapsed); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_perf_us(out: *mut Value, vm_ptr: *mut VM) {
    let vm = unsafe { &*vm_ptr };
    let elapsed = vm.start_instant.elapsed().as_micros() as i64;
    unsafe { *out = Value::from_i64(elapsed); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_perf_ns(out: *mut Value, vm_ptr: *mut VM) {
    let vm = unsafe { &*vm_ptr };
    let elapsed = vm.start_instant.elapsed().as_nanos() as i64;
    unsafe { *out = Value::from_i64(elapsed); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_print(bits: u64, tag: u64) {
    let v = Value { bits, tag: tag };
    #[cfg(unix)]
    crate::runtime::builtin::io::write_buffered(&(v.to_string() + "\x1b[K\r\n"));
    #[cfg(not(unix))]
    crate::runtime::builtin::io::write_buffered(&(v.to_string() + "\x1b[K\n"));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_halt_alert(bits: u64, tag: u64) {
    let v = Value { bits, tag: tag };
    crate::runtime::builtin::io::eprint_buffered(&format!("ALERT: {}", v.to_string()));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_halt_error(_exec_ptr: *mut Executor, bits: u64, tag: u64) {
    let v = Value { bits, tag: tag };
    crate::runtime::builtin::io::eprint_buffered(&format!("ERROR halt: {}", v.to_string()));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_halt_fatal(bits: u64, tag: u64) {
    let v = Value { bits, tag: tag };
    eprintln!("FATAL ERROR halt: {}", v.to_string());
    std::process::exit(1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_typeof(out: *mut Value, bits: u64, tag: u64) {
    let v = Value { bits, tag: tag };
    let type_name = v.type_name();
    unsafe { *out = Value::from_string(std::sync::Arc::new(crate::vm::object::StringObj::new(type_name.as_bytes().to_vec()))); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_store_read(out: *mut Value, bits: u64, tag: u64) {
    let key = Value { bits, tag: tag };
    let path = key.to_string();
    crate::runtime::builtin::store::fs_ops::validate_path_safety(&path);
    match std::fs::read(&path) {
        Ok(b) => {
            unsafe { *out = Value::from_string(std::sync::Arc::new(crate::vm::object::StringObj::new(b))); }
        }
        Err(_) => {
            unsafe { *out = Value::from_i64(0); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_store_write(out: *mut Value, k_bits: u64, k_tag: u64, v_bits: u64, v_tag: u64) {
    let key = Value { bits: k_bits, tag: k_tag };
    let val = Value { bits: v_bits, tag: v_tag };
    let path = key.to_string();
    crate::runtime::builtin::store::fs_ops::validate_path_safety(&path);
    let res = std::fs::write(path, val.to_string()).is_ok();
    unsafe { *out = Value::from_bool(res); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_store_append(out: *mut Value, k_bits: u64, k_tag: u64, v_bits: u64, v_tag: u64) {
    let key = Value { bits: k_bits, tag: k_tag };
    let val = Value { bits: v_bits, tag: v_tag };
    let path = key.to_string();
    crate::runtime::builtin::store::fs_ops::validate_path_safety(&path);
    use std::io::Write as _;
    let ok = if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(path) {
        f.write_all(val.to_string().as_bytes()).is_ok()
    } else { false };
    unsafe { *out = Value::from_bool(ok); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_store_exists(out: *mut Value, bits: u64, tag: u64) {
    let key = Value { bits, tag: tag };
    let path = key.to_string();
    crate::runtime::builtin::store::fs_ops::validate_path_safety(&path);
    let res = std::path::Path::new(&path).exists();
    unsafe { *out = Value::from_bool(res); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_store_delete(out: *mut Value, bits: u64, tag: u64) {
    let key = Value { bits, tag: tag };
    let path = key.to_string();
    crate::runtime::builtin::store::fs_ops::validate_path_safety(&path);
    let res = std::fs::remove_file(path).is_ok();
    unsafe { *out = Value::from_bool(res); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_set_member(obj_bits: u64, obj_tag: u64, name_ptr: *const u8, name_len: u64, val_bits: u64, val_tag: u64) {
    let obj = Value { bits: obj_bits, tag: obj_tag };
    let val = Value { bits: val_bits, tag: val_tag };
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len as usize) };
    let name = String::from_utf8_lossy(name_bytes);
    crate::vm::core::runtime_ops::RuntimeOps::set_member(obj, &name, val);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_str_append_member(obj_bits: u64, obj_tag: u64, name_ptr: *const u8, name_len: u64, val_bits: u64, val_tag: u64) {
    let obj = Value { bits: obj_bits, tag: obj_tag };
    let val = Value { bits: val_bits, tag: val_tag };
    let name_bytes = unsafe { std::slice::from_raw_parts(name_ptr, name_len as usize) };
    let name = String::from_utf8_lossy(name_bytes);
    crate::vm::core::runtime_ops::RuntimeOps::str_append_member(obj, &name, val);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_str_append_element(
    arr_bits: u64,
    arr_tag: u64,
    idx: i64,
    val_bits: u64,
    val_tag: u64,
) -> u32 {
    let arr = Value { bits: arr_bits, tag: arr_tag };
    let val = Value { bits: val_bits, tag: val_tag };
    crate::vm::core::runtime_ops::RuntimeOps::str_append_element(arr, idx as usize, val);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_env_get(out: *mut Value, bits: u64, tag: u64) {
    let key = Value { bits, tag: tag };
    let key_s = key.to_string();
    if let Ok(val) = std::env::var(key_s) {
        unsafe { *out = Value::from_string(std::sync::Arc::new(crate::vm::object::StringObj::new(val.into_bytes()))); }
    } else {
        unsafe { *out = Value::from_i64(0); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_env_args(out: *mut Value) {
    let args: Vec<String> = std::env::args().collect();
    let mut vals = Vec::with_capacity(args.len());
    for a in args {
        vals.push(Value::from_string(std::sync::Arc::new(crate::vm::object::StringObj::new(a.into_bytes()))));
    }
    unsafe { *out = Value::from_array(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::ArrayObj::new(vals)))); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_crypto_hash(out: *mut Value, bits1: u64, tag1: u64, bits2: u64, tag2: u64) {
    let data = Value { bits: bits1, tag: tag1 };
    let salt = Value { bits: bits2, tag: tag2 };
    let data_bytes = if data.is_string() {
        data.as_string().data.clone()
    } else {
        data.to_string().into_bytes()
    };
    let res = crate::runtime::builtin::crypto::hash::hash_impl(data_bytes, salt.to_string());
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_crypto_verify(bits1: u64, tag1: u64, bits2: u64, tag2: u64, bits3: u64, tag3: u64) -> i32 {
    let data = Value { bits: bits1, tag: tag1 };
    let salt = Value { bits: bits2, tag: tag2 };
    let hash = Value { bits: bits3, tag: tag3 };
    if crate::runtime::builtin::crypto::hash::verify_impl(data.to_string(), salt.to_string(), hash.to_string()) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_crypto_token(out: *mut Value, bits: u64, tag: u64) {
    let len = Value { bits, tag: tag }.as_i64() as usize;
    let res = crate::runtime::builtin::crypto::token::token_impl(len);
    unsafe { *out = res; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_map_init(
    out: *mut Value,
    _exec: *mut Executor,
    elements_ptr: *const Value,
    count: u32, 
) {
    let mut elements = Vec::with_capacity(count as usize);
    if !elements_ptr.is_null() && count > 0 {
        let slice = unsafe { std::slice::from_raw_parts(elements_ptr, (count * 2) as usize) };
        for chunk in slice.chunks_exact(2) {
            let key = chunk[0];
            let val = chunk[1];
            unsafe { key.inc_ref(); val.inc_ref(); }
            elements.push((key, val));
        }
    }
    unsafe { *out = Value::from_map(std::sync::Arc::new(parking_lot::RwLock::new(crate::vm::object::MapObj::new(elements)))); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_table_init(
    out: *mut Value,
    skeleton_idx: u32,
    base: u32,
    row_count: u32,
    col_count: u32,
    locals_ptr: *const Value,
    constants_ptr: *const Value,
) {
    let skeleton_val = unsafe { *constants_ptr.add(skeleton_idx as usize) };
    let total_vals = (row_count as usize) * (col_count as usize);
    let values = unsafe { std::slice::from_raw_parts(locals_ptr.add(base as usize), total_vals) };
    match crate::vm::core::runtime_ops::RuntimeOps::table_init(skeleton_val, row_count, values) {
        Ok(res) => {
            unsafe { *out = res; }
        }
        Err(e) => {
            eprintln!("XCX JIT TableInit Error: {}", e);
            unsafe { *out = Value::from_bool(false); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_database_init(
    out: *mut Value,
    engine_bits: u64, engine_tag: u64,
    path_bits: u64, path_tag: u64,
    locals_ptr: *mut Value,
    tables_base_reg: u32,
    table_count: u32,
    executor_ptr: *mut Executor,
) {
    let engine_val = Value { bits: engine_bits, tag: engine_tag };
    let path_val = Value { bits: path_bits, tag: path_tag };
    let exec = unsafe { &mut *executor_ptr };
    
    let engine = engine_val.to_string();
    let path = path_val.to_string();
    
    let mut table_names = Vec::with_capacity(table_count as usize);
    for i in 0..table_count {
        let t_val = unsafe { *locals_ptr.add((tables_base_reg + i) as usize) };
        table_names.push(t_val.to_string());
    }
    
    let _ = exec.handle_database_init(0, engine, path, &table_names, 0, unsafe { std::slice::from_raw_parts_mut(locals_ptr, 256) });
    unsafe { *out = *locals_ptr.add(0); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_yield(
    exec_ptr: *mut Executor,
    val_bits: u64, val_tag: u64,
    next_ip: u64,
    out_ptr: *mut Value,
) -> u32 {
    let ex = unsafe { &mut *exec_ptr };
    if ex.in_fiber {
        let val = Value { bits: val_bits, tag: val_tag };
        unsafe { val.inc_ref(); }
        ex.fiber_next_ip = next_ip as usize;
        ex.fiber_yielded = true;
        unsafe { *out_ptr = val; }
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_http_serve(
    func_idx: u32,
    port_bits: u64, port_tag: u64,
    host_bits: u64, host_tag: u64,
    routes_bits: u64, routes_tag: u64,
    exec_ptr: *mut Executor,
) -> u32 {
    let exec = unsafe { &mut *exec_ptr };
    let port = Value { bits: port_bits, tag: port_tag }.as_i64() as u16;
    let host = Value { bits: host_bits, tag: host_tag }.to_string();
    let routes = Value { bits: routes_bits, tag: routes_tag };
    let vm_arc = exec.vm.clone();
    
    match crate::runtime::builtin::net::server::serve_impl(func_idx, port, host, routes, &exec.ctx, &vm_arc) {
        OpResult::Halt => 1,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_http_respond(
    out: *mut Value,
    status_bits: u64, status_tag: u64,
    body_bits: u64, body_tag: u64,
    headers_bits: u64, headers_tag: u64,
    exec_ptr: *mut Executor,
) -> u32 {
    let exec = unsafe { &mut *exec_ptr };
    let status = Value { bits: status_bits, tag: status_tag }.as_i64() as u32;
    let body = Value { bits: body_bits, tag: body_tag };
    let headers = Value { bits: headers_bits, tag: headers_tag };
    
    match crate::runtime::builtin::net::respond::respond_impl(status, body, headers, exec.ctx.http_req.clone()) {
        OpResult::Halt => {
            unsafe { *out = Value::from_bool(false); }
            1
        }
        _ => {
            unsafe { *out = Value::from_bool(true); }
            0
        }
    }
}

