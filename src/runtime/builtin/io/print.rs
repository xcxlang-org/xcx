use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;

#[cfg(windows)]
const BIN_NAME: &str = "xcx-compiler.exe";
#[cfg(not(windows))]
const BIN_NAME: &str = "xcx-compiler";

/// Prints a value to stdout.
pub fn print_val(src: u8, locals: &[Value]) -> OpResult {
    super::write_buffered(&locals[src as usize].to_string());
    OpResult::Continue
}

/// Runs a terminal command.
pub fn run_cmd(dst: u8, cmd_src: u8, locals: &mut [Value], _executor: &mut Executor, _vm_arc: &Arc<VM>) -> OpResult {
    let cmd = locals[cmd_src as usize].to_string();
    
    // To avoid recursive cargo run deadlocks, we try to use the built binary directly.
    let status = if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let base = std::path::PathBuf::from(manifest_dir).join("target");
        let debug_bin = base.join("debug").join(BIN_NAME);
        let release_bin = base.join("release").join(BIN_NAME);
        
        if debug_bin.exists() {
            std::process::Command::new(debug_bin).arg(&cmd).status()
        } else if release_bin.exists() {
            std::process::Command::new(release_bin).arg(&cmd).status()
        } else {
            std::process::Command::new("cargo").args(["run", "--release", "--", &cmd]).status()
        }
    } else if let Ok(exe) = std::env::current_exe() {
        let exe_name = exe.file_name().unwrap_or_default().to_string_lossy();
        if exe_name.contains("xcx-compiler") || exe_name.contains("xcx") {
             std::process::Command::new(exe).arg(&cmd).status()
        } else {
             std::process::Command::new("cargo").args(["run", "--release", "--", &cmd]).status()
        }
    } else {
        std::process::Command::new("cargo").args(["run", "--release", "--", &cmd]).status()
    };
    let success = status.map(|s| s.success()).unwrap_or(false);
    let res = Value::from_bool(success);
    
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}
