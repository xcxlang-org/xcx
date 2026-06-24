use std::sync::Arc;
use crate::vm::object::StringObj;
use crate::vm::value::Value;
use crate::vm::TypeTag;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crossterm::event::{self, Event, KeyCode};

#[cfg(windows)]
type HANDLE = *mut std::ffi::c_void;

#[cfg(windows)]
unsafe extern "system" {
    fn GetStdHandle(nStdHandle: u32) -> HANDLE;
    fn FlushConsoleInputBuffer(hConsoleInput: HANDLE) -> i32;
}

#[cfg(unix)]
unsafe extern "C" {
    fn tcflush(fd: i32, queue_selector: i32) -> i32;
}

/// Discards all queued/buffered keystroke events in the OS/kernel standard-input device.
pub fn flush_stdin_device() {
    #[cfg(windows)]
    unsafe {
        let handle = GetStdHandle(0xfffffff6); // STD_INPUT_HANDLE
        if !handle.is_null() && handle != !0 as HANDLE {
            let _ = FlushConsoleInputBuffer(handle);
        }
    }

    #[cfg(unix)]
    unsafe {
        let _ = tcflush(0, 0); // fd 0 = stdin, 0 = TCIFLUSH flag
    }
}

pub fn input(dst: u8, ty: TypeTag, locals: &mut [Value], executor: &mut Executor, _vm_arc: &Arc<VM>) -> OpResult {
    if executor.terminal_raw_enabled {
        return read_key(dst, locals, executor);
    }


    if crate::runtime::builtin::io::terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
        super::flush_buffered();
        let res = loop {
            match event::read() {
                Ok(Event::Key(ke)) => {
                    let kv = map_key_code_to_value(ke.code);
                    if !kv.to_string().is_empty() {
                        break kv;
                    }
                }
                Ok(_) => continue,
                Err(_) => {
                    eprintln!("R443: Error: Failed to read input");
                    crate::vm::core::vm::increment_error_count();
                    return OpResult::Halt;
                }
            }
        };
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = res;
        return OpResult::Continue;
    }

    super::flush_buffered();
    use std::io::BufRead;
    
    let mut line = String::new();
    let stdin = std::io::stdin();
    let _ = stdin.lock().read_line(&mut line);

    let trimmed = line.trim_end_matches(['\n', '\r']);
    
    let val = match ty {
        TypeTag::Int => {
            if trimmed.contains('.') {
                eprintln!("R103: Error: Type mismatch - expected integer, got float at input");
                crate::vm::core::vm::increment_error_count();
                return OpResult::Halt;
            }
            if let Ok(n) = trimmed.parse::<i64>() {
                Value::from_i64(n)
            } else {
                eprintln!("R103: Error: Type mismatch - expected integer, got '{}' at input", trimmed);
                crate::vm::core::vm::increment_error_count();
                return OpResult::Halt;
            }
        }
        TypeTag::Float => {
            if let Ok(f) = trimmed.parse::<f64>() {
                Value::from_f64(f)
            } else {
                eprintln!("R103: Error: Type mismatch - expected float, got '{}' at input", trimmed);
                crate::vm::core::vm::increment_error_count();
                return OpResult::Halt;
            }
        }
        TypeTag::Bool => {
            if trimmed == "true" {
                Value::from_bool(true)
            } else if trimmed == "false" {
                Value::from_bool(false)
            } else {
                eprintln!("R103: Error: Type mismatch - expected boolean, got '{}' at input", trimmed);
                crate::vm::core::vm::increment_error_count();
                return OpResult::Halt;
            }
        }
        TypeTag::String => {
            Value::from_string(Arc::new(StringObj::new(trimmed.to_string().into_bytes())))
        }
        TypeTag::Unknown => {
            if let Ok(n) = trimmed.parse::<i64>() {
                Value::from_i64(n)
            } else if let Ok(f) = trimmed.parse::<f64>() {
                Value::from_f64(f)
            } else if trimmed == "true" {
                Value::from_bool(true)
            } else if trimmed == "false" {
                Value::from_bool(false)
            } else {
                Value::from_string(Arc::new(StringObj::new(trimmed.to_string().into_bytes())))
            }
        }
        _ => {
            Value::from_string(Arc::new(StringObj::new(trimmed.to_string().into_bytes())))
        }
    };
    
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = val;
    OpResult::Continue
}

/// Reads a single key event (non-blocking).
pub fn read_key(dst: u8, locals: &mut [Value], executor: &Executor) -> OpResult {
    if !executor.terminal_raw_enabled {
        eprintln!("R442: Alert: input.key() called outside !raw mode");
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = Value::from_string(Arc::new(StringObj::new(vec![])));
        return OpResult::Continue;
    }

    let mut last_key_val = Value::from_string(Arc::new(StringObj::new(vec![])));
    // First poll: wait up to 15ms to allow the OS console thread to deliver any buffered key events
    let mut has_event = event::poll(std::time::Duration::from_millis(15)).unwrap_or(false);
    while has_event {
        match event::read() {
            Ok(Event::Key(ke)) => {
                let kv = map_key_code_to_value(ke.code);
                if !kv.to_string().is_empty() {
                    last_key_val = kv;
                }
            }
            Ok(_) => {} // Ignore mouse/focus/resize
            Err(_) => break,
        }
        // Subsequent polls: drain already buffered events immediately with zero delay
        has_event = event::poll(std::time::Duration::from_millis(0)).unwrap_or(false);
    }
    let res = last_key_val;
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}

/// Waits for a single key event (blocking).
pub fn wait_key(dst: u8, locals: &mut [Value], executor: &Executor, _vm_arc: &Arc<VM>) -> OpResult {
    if !executor.terminal_raw_enabled {
        eprintln!("R442: Alert: input.key() called outside !raw mode");
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = Value::from_string(Arc::new(StringObj::new(vec![])));
        return OpResult::Continue;
    }

    super::flush_buffered();
    let res = loop {
        match event::read() {
            Ok(Event::Key(ke)) => {
                let kv = map_key_code_to_value(ke.code);
                if !kv.to_string().is_empty() {
                    break kv;
                }
            }
            Ok(_) => continue,
            Err(_) => {
                eprintln!("R443: Error: Failed to read input");
                crate::vm::core::vm::increment_error_count();
                return OpResult::Halt;
            }
        }
    };
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}

/// Checks if input is ready (poll).
pub fn is_ready(dst: u8, locals: &mut [Value]) -> OpResult {
    let ready = event::poll(std::time::Duration::from_millis(0)).unwrap_or(false);
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = Value::from_bool(ready);
    OpResult::Continue
}

fn map_key_code_to_value(code: KeyCode) -> Value {
    match code {
        KeyCode::Char(c) => Value::from_string(Arc::new(StringObj::new(vec![c as u8]))),
        KeyCode::Esc => Value::from_string(Arc::new(StringObj::new(b"ESC".to_vec()))),
        KeyCode::Enter => Value::from_string(Arc::new(StringObj::new(b"ENTER".to_vec()))),
        KeyCode::Tab => Value::from_string(Arc::new(StringObj::new(b"TAB".to_vec()))),
        KeyCode::Backspace => Value::from_string(Arc::new(StringObj::new(b"BACKSPACE".to_vec()))),
        KeyCode::Up => Value::from_string(Arc::new(StringObj::new(b"UP".to_vec()))),
        KeyCode::Down => Value::from_string(Arc::new(StringObj::new(b"DOWN".to_vec()))),
        KeyCode::Left => Value::from_string(Arc::new(StringObj::new(b"LEFT".to_vec()))),
        KeyCode::Right => Value::from_string(Arc::new(StringObj::new(b"RIGHT".to_vec()))),
        KeyCode::F(n) => Value::from_string(Arc::new(StringObj::new(format!("F{}", n).into_bytes()))),
        _ => Value::from_string(Arc::new(StringObj::new(vec![]))),
    }
}
