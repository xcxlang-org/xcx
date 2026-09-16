//! Shared harness for the XCX regression suites (`tests/*.rs`).
//!
//! Rules baked in here:
//! - every program is written to `target/xcx_regression/<name>.xcx` and run
//!   through the real CLI binary (subprocess), so tests exercise exactly what
//!   users run;
//! - JIT/interpreter parity compares the full stdout of the *same file* run
//!   in both modes;
//! - a failing test copies its program to `target/xcx_test_fail/` and panics
//!   with the source, the file path and both output streams attached — no
//!   fixture-directory hunting;
//! - performance guards never assert absolute wall-clock thresholds; they use
//!   self-timed programs (`perf.ns()`) and ratio assertions, which hold on
//!   any hardware.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const EXE: &str = env!("CARGO_BIN_EXE_xcx");
const RUN_TIMEOUT: Duration = Duration::from_secs(90);

pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }
}

fn regression_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("xcx_regression");
    std::fs::create_dir_all(&dir).expect("create target/xcx_regression");
    dir
}

fn fail_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("xcx_test_fail");
    std::fs::create_dir_all(&dir).expect("create target/xcx_test_fail");
    dir
}

pub fn write_source(name: &str, source: &str) -> PathBuf {
    let file = regression_dir().join(format!("{name}.xcx"));
    std::fs::write(&file, source).expect("write regression program");
    file
}

/// Spawns a program without waiting for it to exit (server-style tests).
/// The caller owns the child and must kill/wait it.
pub fn spawn_program(name: &str, source: &str) -> (PathBuf, Child) {
    let file = write_source(name, source);
    let child = Command::new(EXE)
        .arg(&file)
        .current_dir(file.parent().unwrap())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn xcx binary");
    (file, child)
}

/// Runs the file and captures exit code + both streams. Panics on timeout.
pub fn run_file(file: &PathBuf, extra: &[&str]) -> Run {
    let mut child = Command::new(EXE)
        .args(extra)
        .arg(file)
        .current_dir(file.parent().unwrap())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn xcx binary");

    let mut out_pipe = child.stdout.take().expect("stdout pipe");
    let mut err_pipe = child.stderr.take().expect("stderr pipe");
    let out_t = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = String::new();
        let _ = out_pipe.read_to_string(&mut buf);
        buf
    });
    let err_t = std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = String::new();
        let _ = err_pipe.read_to_string(&mut buf);
        buf
    });

    let deadline = Instant::now() + RUN_TIMEOUT;
    let status = loop {
        match child.try_wait().expect("poll child") {
            Some(status) => break status,
            None => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("xcx run timed out after 90 s: {}", file.display());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    };

    let stdout = out_t.join().unwrap_or_default();
    let stderr = err_t.join().unwrap_or_default();
    Run {
        code: status.code(),
        stdout,
        stderr,
    }
}

pub fn run_source(name: &str, source: &str, extra: &[&str]) -> Run {
    let file = write_source(name, source);
    run_file(&file, extra)
}

pub fn run_jit(name: &str, source: &str) -> Run {
    run_source(name, source, &[])
}

pub fn run_nojit(name: &str, source: &str) -> Run {
    run_source(name, source, &["--no-jit"])
}

/// Strips the terminal erase-to-end-of-line sequence the CLI appends to
/// every printed line on Windows, so textual assertions see clean values.
pub fn normalize(output: &str) -> String {
    output.replace("\u{1b}[K", "")
}

/// Runs the program in JIT mode, asserting success. Returns trimmed stdout.
pub fn expect_success(name: &str, source: &str) -> String {
    let file = write_source(name, source);
    let run = run_file(&file, &[]);
    if !run.ok() {
        report_failure(name, &file, source, &run);
    }
    normalize(&run.stdout).trim().to_string()
}

/// Runs the *same file* through JIT and interpreter mode and asserts an
/// identical exit status and identical stdout. Returns the JIT stdout.
pub fn assert_parity(name: &str, source: &str) -> String {
    let file = write_source(name, source);
    let jit = run_file(&file, &[]);
    let nojit = run_file(&file, &["--no-jit"]);
    if !jit.ok() || !nojit.ok() || normalize(&jit.stdout) != normalize(&nojit.stdout) {
        report_failure(&format!("{name} [parity]"), &file, source, &jit);
    }
    if !nojit.ok() {
        report_failure(&format!("{name} [parity/nojit]"), &file, source, &nojit);
    }
    normalize(&jit.stdout).trim().to_string()
}

fn report_failure(label: &str, file: &PathBuf, source: &str, run: &Run) -> ! {
    let sanitized: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let kept = fail_dir().join(format!("{sanitized}.xcx"));
    let _ = std::fs::write(&kept, source);
    panic!(
        "regression '{label}' failed (exit {:?})\n\
         program: {}\n\
         copy for manual re-run: {}\n\
         --- stdout ---\n{}\n\
         --- stderr ---\n{}",
        run.code,
        file.display(),
        kept.display(),
        run.stdout,
        run.stderr
    )
}
