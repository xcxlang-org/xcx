use crate::vm::value::Value;
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_fiber_is_done(fib_bits: u64, fib_tag: u64) -> bool {
    let fib = Value { bits: fib_bits, tag: fib_tag };
    if !fib.is_fiber() { return true; }
    fib.as_fiber().read().is_done
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_fiber_next(
    out: *mut Value,
    fib_bits: u64,
    fib_tag: u64,
    executor_ptr: *mut Executor,
) {
    let fib_val = Value { bits: fib_bits, tag: fib_tag };
    if !fib_val.is_fiber() { 
        unsafe { *out = Value::from_i64(0); }
        return; 
    }
    
    let executor = unsafe { &mut *executor_ptr };
    let fib_rc = fib_val.as_fiber();
    let vm_arc = executor.vm.clone();
    
    let mut locals = [Value::from_bool(false); 1];
    
    executor.handle_fiber_method(
        0, 
        fib_rc, 
        MethodKind::Next, 
        &[], 
        None, 
        0, 
        &mut locals, 
        &vm_arc, 
    );
    
    unsafe { *out = locals[0]; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_fiber_run(
    fib_bits: u64,
    fib_tag: u64,
    executor_ptr: *mut Executor,
) -> bool {
    let fib_val = Value { bits: fib_bits, tag: fib_tag };
    if !fib_val.is_fiber() { 
        return true; 
    }
    
    let executor = unsafe { &mut *executor_ptr };
    let fib_rc = fib_val.as_fiber();
    let vm_arc = executor.vm.clone();
    
    let mut locals = [Value::from_bool(false); 1];
    
    executor.handle_fiber_method(
        0, 
        fib_rc.clone(), 
        MethodKind::Run, 
        &[], 
        None, 
        0, 
        &mut locals, 
        &vm_arc, 
    );
    
    !fib_rc.read().is_done
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_fiber_create(
    out: *mut Value,
    func_idx: i64,
    base: i8,
    arg_count: i8,
    executor_ptr: *mut Executor,
    locals_ptr: *mut Value,
) {
    let executor = unsafe { &mut *executor_ptr };
    let chunk = executor.ctx.functions[func_idx as usize].clone();
    let mut f_locals = vec![Value::from_bool(false); chunk.max_locals];
    
    let args_slice = unsafe { std::slice::from_raw_parts(locals_ptr.offset(base as isize), arg_count as usize) };
    for i in 0..arg_count as usize {
        let v = args_slice[i];
        if v.is_ptr() { unsafe { v.inc_ref(); } }
        if i < f_locals.len() {
            f_locals[i] = v;
        }
    }
    
    let fiber = crate::vm::object::FiberObj {
        func_id: func_idx as usize,
        ip: 0,
        locals: f_locals,
        status: crate::vm::object::FiberStatus::Suspended,
        is_done: false,
        yielded_value: None,
        trace_revision: 0,
    };
    
    unsafe { *out = Value::from_fiber(std::sync::Arc::new(parking_lot::RwLock::new(fiber))); }
}
