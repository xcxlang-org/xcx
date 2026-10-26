pub mod input;
pub mod terminal;

pub use input::*;
pub use terminal::*;

use std::cell::RefCell;
use std::io::{BufWriter, Stdout, Write};
use std::sync::OnceLock;

thread_local! {
    pub static COMPILER_STDOUT: RefCell<BufWriter<Stdout>> = RefCell::new(BufWriter::with_capacity(64 * 1024, std::io::stdout()));
}

pub fn is_verbose_enabled() -> bool {
    static CACHE: OnceLock<bool> = OnceLock::new();
    *CACHE.get_or_init(|| {
        let is_test_bin = std::env::current_exe()
            .ok()
            .map(|p| {
                let path_lower = p.to_string_lossy().to_lowercase();
                let file_name = p.file_name()
                    .map(|f| f.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                file_name.contains("test")
                    || file_name.contains("runner")
                    || path_lower.contains("/deps/")
                    || path_lower.contains("\\deps\\")
            })
            .unwrap_or(false);
            
        let in_test = is_test_bin || std::env::var("XCX_IN_TEST_HARNESS").is_ok();
        
        if in_test {
            let args: Vec<String> = std::env::args().collect();
            args.iter().any(|a| a == "--nocapture" || a == "--show-output")
        } else {
            true
        }
    })
}

pub fn eprint_buffered(s: &str) {
    if is_verbose_enabled() {
        eprintln!("{}", s);
    }
}

/// Line ending for `>!` output. The erase-to-EOL sequence only serves
/// interactive redraw and is emitted when stdout is a terminal (or raw
/// mode is active); redirected pipes and files receive plain text, so
/// scripts and CI read program output without stripping escape codes.
pub fn print_line_ending() -> &'static str {
    static STDOUT_IS_TTY: OnceLock<bool> = OnceLock::new();
    let tty = *STDOUT_IS_TTY.get_or_init(|| std::io::IsTerminal::is_terminal(&std::io::stdout()));
    if tty || terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
        "\x1b[K\n"
    } else {
        "\n"
    }
}

pub fn write_buffered(s: &str) {
    if !is_verbose_enabled() {
        return;
    }
    COMPILER_STDOUT.with(|buffered| {
        let mut b = buffered.borrow_mut();
        let _ = b.write_all(s.as_bytes());
        if !terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
            let _ = b.flush();
        }
    });
}

pub fn flush_buffered() {
    if !is_verbose_enabled() {
        return;
    }
    COMPILER_STDOUT.with(|buffered| {
        let mut buf = buffered.borrow_mut();
        if terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
            let _ = buf.write_all(b"\x1b[J");
        }
        let _ = buf.write_all(b"\x1b[?25h");
        let _ = buf.flush();
    });
}
