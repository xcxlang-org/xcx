use std::fs;
use std::env;
use std::sync::Arc;
use std::path::Path;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use xcx_compiler::frontend::parser::Parser;
use xcx_compiler::frontend::parser::expander::Expander;
use xcx_compiler::sema::{Checker, SymbolTable};
use xcx_compiler::compiler::Compiler;
use xcx_compiler::vm::{VM, SharedContext, SHUTDOWN};
use xcx_compiler::error::Reporter;

struct TerminalCleanup;

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        if xcx_compiler::runtime::builtin::io::terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

fn main() {
    let _cleanup = TerminalCleanup;
    ctrlc::set_handler(move || {
        SHUTDOWN.store(true, std::sync::atomic::Ordering::SeqCst);
        println!("\n[XCX] Shutdown signal received. Cleaning up...");
        if xcx_compiler::runtime::builtin::io::terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
            let _ = crossterm::terminal::disable_raw_mode();
        }
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    let mut args: Vec<String> = env::args().collect();
    let mut disable_jit = false;
    
    if let Some(pos) = args.iter().position(|r| r == "--no-jit") {
        disable_jit = true;
        args.remove(pos);
    }

    if args.len() < 2 {
        let mut repl = xcx_compiler::repl::Repl::new(disable_jit);
        repl.run();
        return;
    }

    let first_arg = &args[1];
    if first_arg == "--version" || first_arg == "version" {
        let mut version = env!("CARGO_PKG_VERSION");
        if version.ends_with(".0") {
            version = &version[..version.len() - 2];
        }
        println!("xcx {} ({}/{})", version, env::consts::OS, env::consts::ARCH);
        return;
    }

    if first_arg == "--help" || first_arg == "help" || first_arg == "-h" {
        println!("Usage:");
        println!("  xcx                     Start REPL");
        println!("  xcx <file.xcx>          Run file");
        println!("  xcx --version           Show version");
        println!("  xcx --help              Show help");
        println!("  xcx --no-jit <file.xcx> Run file with JIT compiler disabled");
        println!("\nInside REPL:");
        println!("  !help                   Show REPL commands");
        return;
    }

    if first_arg == "pax" || first_arg == "doc" {
        let rel_path = if first_arg == "pax" { "lib/pax.xcx" } else { "lib/doc/doc.xcx" };
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
            let install_dir = if first_arg == "pax" { "lib directory" } else { "lib/doc directory" };
            eprintln!("{} not found at {}. Please ensure it is installed in the {}.", tool_name, resolved_path, install_dir);
            return;
        }
        run_file(&resolved_path, disable_jit);
    } else {
        run_file(first_arg, disable_jit);
    }
}

fn run_file(filename: &str, disable_jit: bool) {
    let source = match fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Could not read file {}: {}", filename, e);
            return;
        }
    };

    let current_dir = Path::new(filename)
        .parent()
        .unwrap_or(Path::new("."));

    let mut parser = Parser::new(&source);
    let program_raw = parser.parse_program();
    if parser.has_error {
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
            return;
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
        std::process::exit(1);
    }

    let mut compiler = Compiler::new();
    let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);


    let ctx = SharedContext {
        constants,
        functions,
        http_req: None,
    };

    let mut vm_inner = VM::new();
    vm_inner.disable_jit = disable_jit;
    let vm = Arc::new(vm_inner);
    let vm2 = vm.clone();
    
    // Use a larger stack size for the VM thread to accommodate deep native recursion in JIT.
    let handle = std::thread::Builder::new()
        .name("xcx-executor".to_string())
        .stack_size(64 * 1024 * 1024) // 64MB
        .spawn(move || {
            vm2.run(Arc::new(main_chunk), ctx, &[]);
        })
        .expect("Failed to spawn VM thread");

    handle.join().expect("VM thread panicked");
    
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let error_count = vm.error_count.load(std::sync::atomic::Ordering::SeqCst);
    if error_count > 0 {
        eprintln!("[XCX] Process failed with {} errors.", error_count);
        std::process::exit(1);
    }
}