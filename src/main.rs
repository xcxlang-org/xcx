use std::fs;
use std::env;
use std::sync::Arc;
use std::path::Path;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use xcx::frontend::parser::Parser;
use xcx::frontend::parser::expander::Expander;
use xcx::sema::{Checker, SymbolTable};
use xcx::compiler::Compiler;
use xcx::vm::{VM, SharedContext, SHUTDOWN};
use xcx::error::Reporter;

struct TerminalCleanup;

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        if xcx::runtime::builtin::io::terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

fn main() {
    let _cleanup = TerminalCleanup;

    ctrlc::set_handler(move || {
        SHUTDOWN.store(true, std::sync::atomic::Ordering::SeqCst);
        println!("\n[XCX] Shutdown signal received. Cleaning up...");
        if xcx::runtime::builtin::io::terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
            let _ = crossterm::terminal::disable_raw_mode();
        }
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    let raw_args: Vec<String> = env::args().collect();
    let mut args = Vec::new();
    for arg in raw_args {
        if arg.contains('|') {
            for part in arg.split('|') {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    args.push(trimmed.to_string());
                }
            }
        } else {
            args.push(arg);
        }
    }

    let is_tool = args.len() > 1 && (args[1] == "pax" || args[1] == "doc");

    if !is_tool && args.iter().any(|r| r == "--help" || r == "-h" || r == "help") {
        println!("xcx — statically typed language with JIT compilation\n");
        println!("Usage:");
        println!("  xcx                    Start REPL");
        println!("  xcx <file>             Run file\n");
        println!("Options:");
        println!("  -v, --version          Show version");
        println!("  -h, --help             Show this help");
        println!("  Multiple options can be combined with '|' (e.g. --no-jit | --bytecode)\n");
        println!("Execution:");
        println!("  --no-jit               Disable JIT compiler");
        println!("  --no-inline            Disable HIR-level inline optimizer pass");
        println!("  --threshold=N, --th=N  Set JIT compilation threshold (default: 50)\n");
        println!("Dev tools:");
        println!("  --check                Validate syntax and types only (dry run)");
        println!("  --bytecode             Print bytecode output and exit\n");
        println!("Run 'xcx <file> [options]' to execute with flags.");
        println!("Inside REPL, type !help for REPL commands.\n");
        println!("Website: xcxlang.com | Email: contact@xcxlang.com");
        return;
    }

    if !is_tool && args.iter().any(|r| r == "--version" || r == "-v" || r == "version") {
        let mut version = env!("CARGO_PKG_VERSION");
        if version.ends_with(".0") {
            version = &version[..version.len() - 2];
        }
        println!("xcx {} ({}/{})", version, env::consts::OS, env::consts::ARCH);
        return;
    }

    let mut disable_jit = false;
    let mut disable_inline = false;
    let mut check_only = false;
    let mut dump_bytecode_only = false;
    
    if let Some(pos) = args.iter().position(|r| r == "--no-jit") {
        disable_jit = true;
        args.remove(pos);
    }
    if let Some(pos) = args.iter().position(|r| r == "--no-inline") {
        disable_inline = true;
        args.remove(pos);
    }
    if !is_tool {
        if let Some(pos) = args.iter().position(|r| r == "--check") {
            check_only = true;
            args.remove(pos);
        }
        if let Some(pos) = args.iter().position(|r| r == "--bytecode") {
            dump_bytecode_only = true;
            args.remove(pos);
        }
    }

    let mut jit_threshold = 50;
    let mut i = 0;
    while i < args.len() {
        if args[i].starts_with("--threshold=") {
            let val_str = &args[i]["--threshold=".len()..];
            match val_str.parse::<u32>() {
                Ok(val) => jit_threshold = val,
                Err(_) => {
                    eprintln!("Error: --threshold requires a valid unsigned integer.");
                    std::process::exit(1);
                }
            }
            args.remove(i);
        } else if args[i].starts_with("--th=") {
            let val_str = &args[i]["--th=".len()..];
            match val_str.parse::<u32>() {
                Ok(val) => jit_threshold = val,
                Err(_) => {
                    eprintln!("Error: --th requires a valid unsigned integer.");
                    std::process::exit(1);
                }
            }
            args.remove(i);
        } else {
            i += 1;
        }
    }

    if args.len() < 2 {
        let mut repl = xcx::repl::Repl::new(disable_jit, jit_threshold);
        repl.run();
        return;
    }

    let first_arg = &args[1];
    if first_arg == "pax" || first_arg == "doc" {
        let rel_path = if first_arg == "pax" { "lib/pax/src/pax.xcx" } else { "lib/doc/doc.xcx" };
        let mut resolved_path = rel_path.to_string();
        
        if !Path::new(&resolved_path).exists() {
            if let Ok(exe_path) = env::current_exe() {
                let mut current = exe_path.parent();
                while let Some(dir) = current {
                    let alt_path = dir.join(rel_path);
                    if alt_path.exists() {
                        resolved_path = alt_path.to_string_lossy().to_string();
                        break;
                    }
                    current = dir.parent();
                }
            }
        }
 
        if !Path::new(&resolved_path).exists() {
            let tool_name = if first_arg == "pax" { "PAX manager" } else { "DOC tool" };
            let install_dir = if first_arg == "pax" { "lib/pax/src directory" } else { "lib/doc directory" };
            eprintln!("{} not found at {}. Please ensure it is installed in the {}.", tool_name, resolved_path, install_dir);
            return;
        }
        run_file(&resolved_path, disable_jit, jit_threshold, disable_inline, check_only, dump_bytecode_only);
    } else {
        run_file(first_arg, disable_jit, jit_threshold, disable_inline, check_only, dump_bytecode_only);
    }
}

fn run_file(
    filename: &str,
    disable_jit: bool,
    jit_threshold: u32,
    disable_inline: bool,
    check_only: bool,
    dump_bytecode_only: bool,
) {
    let source = match fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Could not read file {}: {}", filename, e);
            std::process::exit(1);
        }
    };

    let current_dir = Path::new(filename)
        .parent()
        .unwrap_or(Path::new("."));

    let mut parser = Parser::new(&source);
    let program_raw = parser.parse_program();
    if parser.has_error {
        if check_only {
            eprintln!("\n[XCX] Syntax and semantic analysis failed due to syntax errors.");
        }
        std::process::exit(1);
    }
    let mut interner = parser.into_interner();

    let mut expander = Expander::new(&mut interner);

    if let Ok(cwd) = std::env::current_dir() {
        let lib_path = cwd.join("lib");
        if lib_path.exists() {
            expander.add_include_path(lib_path);
        }
    }

    let mut program = match expander.expand(program_raw, current_dir) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Expansion error: {}", e);
            if check_only {
                eprintln!("\n[XCX] Syntax and semantic analysis failed due to expansion errors.");
            }
            std::process::exit(1);
        }
    };

    let mut checker = Checker::new(&interner);
    let mut symbols = SymbolTable::new();
    let errors = checker.check(&mut program, &mut symbols);

    if !errors.is_empty() {
        let mut reporter = Reporter::new(&source);
        for err in &errors {
            reporter.error(err.span.line, err.span.col, err.span.len, &err.kind.to_diagnostic_message());
        }
        if check_only {
            eprintln!("\n[XCX] Syntax and semantic analysis failed with {} error(s).", errors.len());
        }
        std::process::exit(1);
    }

    if check_only {
        println!("[XCX] Syntax and semantic analysis passed successfully.");
        std::process::exit(0);
    }

    let mut compiler = Compiler::new();
    compiler.disable_inline = disable_inline;
    let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);

    if dump_bytecode_only {
        println!("=== CONSTANTS DUMP ===");
        for (idx, val) in constants.iter().enumerate() {
            println!("{:04}: {:?}", idx, val);
        }
        println!("=== BYTECODE DUMP FOR MAIN CHUNK ===");
        for (ip, op) in main_chunk.bytecode.iter().enumerate() {
            println!("{:04}: {:?}", ip, op);
        }
        for (f_idx, f_chunk) in functions.iter().enumerate() {
            println!("=== BYTECODE DUMP FOR FUNCTION {} ({}) ===", f_idx, f_chunk.name);
            println!("max_locals: {}", f_chunk.max_locals);
            for (ip, op) in f_chunk.bytecode.iter().enumerate() {
                println!("{:04}: {:?}", ip, op);
            }
        }
    }

    if dump_bytecode_only {
        std::process::exit(0);
    }

    let ctx = SharedContext {
        constants,
        functions,
        http_req: None,
    };

    let mut vm_inner = VM::new();
    vm_inner.disable_jit.store(disable_jit, std::sync::atomic::Ordering::Release);
    vm_inner.jit_threshold = jit_threshold;
    let vm = Arc::new(vm_inner);
    let vm2 = vm.clone();

    start_memory_watchdog();

    let handle = std::thread::Builder::new()
        .name("xcx-executor".to_string())
        .stack_size(64 * 1024 * 1024) // 64MB
        .spawn(move || {
            vm2.run(Arc::new(main_chunk), ctx, &[]);
        })
        .expect("Failed to spawn VM thread");

    handle.join().expect("VM thread panicked");

    if std::env::var("XCX_WATCHDOG_DEBUG").is_ok() {
        unsafe extern "C" {
            fn mi_stats_print(out: *mut core::ffi::c_void);
        }
        unsafe { mi_stats_print(std::ptr::null_mut()) };
    }
    
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let error_count = vm.error_count.load(std::sync::atomic::Ordering::SeqCst);
    if error_count > 0 {
        eprintln!("[XCX] Process failed with {} errors.", error_count);
        std::process::exit(1);
    }
}

/// Default memory ceiling for a running XCX process (committed private
/// bytes). Chosen above the heaviest legitimate benchmark peaks (~300 MB)
/// and far below the 18 GB runaway-leak incidents that motivated it.
const MEMORY_WATCHDOG_DEFAULT_MB: usize = 8 * 1024;

/// Polls the process' committed private memory every 250 ms and terminates
/// the process the moment it exceeds the ceiling. A runaway leak must not be
/// allowed to degrade the host machine — by the time a graceful unwind could
/// be attempted, the address space itself is the hazard, so this exits hard.
/// Override the ceiling with `XCX_MAX_RAM_MB` (0 disables the watchdog).
#[cfg(windows)]
fn start_memory_watchdog() {
    let limit_mb: usize = std::env::var("XCX_MAX_RAM_MB")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(MEMORY_WATCHDOG_DEFAULT_MB);
    if limit_mb == 0 {
        return;
    }

    std::thread::Builder::new()
        .name("xcx-mem-watchdog".to_string())
        .spawn(move || {
            let debug = std::env::var("XCX_WATCHDOG_DEBUG").is_ok();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let used_mb = unsafe { private_bytes_mb() };
                if debug {
                    eprintln!("[watchdog] used={}MB", used_mb);
                }
                if used_mb > limit_mb {
                    flush_all_output();
                    eprintln!(
                        "[XCX] Fatal: memory limit exceeded ({} MB in use, limit {} MB). Terminating to protect the host.",
                        used_mb, limit_mb
                    );
                    std::process::exit(1);
                }
            }
        })
        .expect("Failed to spawn memory watchdog thread");
}

#[cfg(not(windows))]
fn start_memory_watchdog() {}

#[cfg(windows)]
unsafe fn private_bytes_mb() -> usize {
    #[repr(C)]
    struct ProcessMemoryCountersEx {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        /// PRIVATE_USAGE — committed private bytes, the leak metric.
        private_usage: usize,
    }

    unsafe extern "system" {
        // K32 variant: modern kernel32 export of GetProcessMemoryInfo (the
        // psapi.dll forwarder is unreliable to link directly here).
        fn K32GetProcessMemoryInfo(
            process: *mut core::ffi::c_void,
            counters: *mut ProcessMemoryCountersEx,
            cb: u32,
        ) -> i32;
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn GetLastError() -> u32;
    }

    let mut counters = ProcessMemoryCountersEx {
        cb: std::mem::size_of::<ProcessMemoryCountersEx>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
        private_usage: 0,
    };
    // PRIVATE_USAGE (== PagefileUsage semantics) = committed private bytes —
    // the metric that grows with a leak regardless of what the working set
    // trims. The EX struct is required: the plain 64-byte buffer is rejected
    // with ERROR_INSUFFICIENT_BUFFER (122).
    let ok = unsafe {
        K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb)
    };
    if polls_debug() {
        let err = unsafe { GetLastError() };
        eprintln!(
            "[watchdog-dbg] ok={} err={} private_usage={}MB cb={}",
            ok, err, counters.private_usage / (1024 * 1024), counters.cb
        );
    }
    if ok != 0 {
        counters.private_usage / (1024 * 1024)
    } else {
        0
    }
}

fn polls_debug() -> bool {
    // Raw per-poll diagnostics only under the explicit debug flag.
    std::env::var("XCX_WATCHDOG_DEBUG").is_ok()
}

fn flush_all_output() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}