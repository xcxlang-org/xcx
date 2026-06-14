use crossterm::{execute, terminal::{disable_raw_mode, enable_raw_mode}, cursor::{MoveTo, Show, Hide}};
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use std::sync::atomic::{AtomicBool, Ordering};

pub static OS_RAW_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Clears the terminal screen (.terminal !clear).
pub fn clear() -> OpResult {
    super::write_buffered("\x1b[?25l\x1b[H");
    OpResult::Continue
}

/// Enables terminal raw mode (.terminal !raw).
pub fn raw_mode(executor: &mut Executor) -> OpResult {
    super::flush_buffered();
    if !OS_RAW_ACTIVE.load(Ordering::Acquire) {
        if let Err(_) = enable_raw_mode() {
            eprintln!("R440: Error: Failed to set terminal mode");
            return OpResult::Halt;
        }
        OS_RAW_ACTIVE.store(true, Ordering::Release);
        
        // Purge any pending event queue immediately after physically enabling raw mode
        while crossterm::event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
            let _ = crossterm::event::read();
        }
    }
    executor.terminal_raw_enabled = true;
    OpResult::Continue
}

/// Disables terminal raw mode (.terminal !normal).
pub fn normal_mode(executor: &mut Executor) -> OpResult {
    super::flush_buffered();
    executor.terminal_raw_enabled = false;
    OpResult::Continue
}

/// Toggles terminal cursor visibility (.terminal !cursor on/off).
pub fn cursor(on: bool) -> OpResult {
    let res = super::COMPILER_STDOUT.with(|buffered| {
        let mut buf = buffered.borrow_mut();
        if on {
            execute!(buf, Show)
        } else {
            execute!(buf, Hide)
        }
    });
    if res.is_err() {
        return OpResult::Halt;
    }
    OpResult::Continue
}

/// Moves the terminal cursor (.terminal !move x y).
pub fn move_cursor(x_src: u8, y_src: u8, locals: &[Value]) -> OpResult {
    let x = locals[x_src as usize].as_i64();
    let y = locals[y_src as usize].as_i64();
    
    if x < 0 || y < 0 || x > 32767 || y > 32767 {
         eprintln!("R441: Error: Cursor position out of bounds (x:{}, y:{})", x, y);
         return OpResult::Halt;
    }

    let res = super::COMPILER_STDOUT.with(|buffered| {
        let mut buf = buffered.borrow_mut();
        execute!(buf, MoveTo(x as u16, y as u16))
    });
    if res.is_err() {
         eprintln!("R441: Error: Cursor position out of bounds");
         return OpResult::Halt;
    }
    OpResult::Continue
}

/// Exits the terminal and the process (.terminal !exit).
pub fn exit() -> OpResult {
    if OS_RAW_ACTIVE.load(Ordering::Acquire) {
        let _ = disable_raw_mode();
        OS_RAW_ACTIVE.store(false, Ordering::Release);
    }
    std::process::exit(0);
}
