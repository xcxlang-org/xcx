use crate::vm::value::Value;
use crate::vm::core::executor::Executor;
use std::sync::Arc;
use crate::vm::object::StringObj;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_terminal_clear() {
    crate::runtime::builtin::io::clear();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_terminal_raw(exec_ptr: *mut Executor) {
    let exec = unsafe { &mut *exec_ptr };
    crate::runtime::builtin::io::raw_mode(exec);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_terminal_normal(exec_ptr: *mut Executor) {
    let exec = unsafe { &mut *exec_ptr };
    crate::runtime::builtin::io::normal_mode(exec);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_terminal_cursor(on: u8) {
    crate::runtime::builtin::io::cursor(on != 0);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_terminal_move(x_bits: u64, x_tag: u64, y_bits: u64, y_tag: u64) {
    let x = Value { bits: x_bits, tag: x_tag };
    let y = Value { bits: y_bits, tag: y_tag };
    let mut locals = vec![x, y];
    crate::runtime::builtin::io::move_cursor(0, 1, &mut locals);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_terminal_exit() {
    crate::runtime::builtin::io::exit();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_terminal_run(out: *mut Value, cmd_bits: u64, cmd_tag: u64) {
    let cmd_val = Value { bits: cmd_bits, tag: cmd_tag };
    let cmd = cmd_val.to_string();
    let output = crate::runtime::builtin::io::execute_run(&cmd);
    let res = match output {
        Ok(o) => {
            if !o.stdout.is_empty() {
                let s = String::from_utf8_lossy(&o.stdout);
                crate::runtime::builtin::io::write_buffered(&s);
                crate::runtime::builtin::io::flush_buffered();
            }
            if o.status.success() {
                if o.stdout.is_empty() {
                    Value::from_bool(true)
                } else {
                    Value::from_string(Arc::new(StringObj::new(o.stdout)))
                }
            } else {
                Value::from_bool(false)
            }
        }
        Err(_) => Value::from_bool(false),
    };
    unsafe { *out = res; }
}



#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_terminal_write(src_bits: u64, src_tag: u64) {
    let src_val = Value { bits: src_bits, tag: src_tag };
    crate::runtime::builtin::io::write_buffered(&src_val.to_string());
}

