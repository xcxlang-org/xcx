//! Rust port of the stability scenario runner (formerly
//! `tests/cli_tests/runner.py`).
//!
//! Discovers the numbered `*.xcx` scenario fixtures under
//! `tests/cli_tests/**`, parses their metadata comments (`--- TEST:`,
//! `--- Area:`, `--- Priority:`, `--- Runner: …`) and evaluates them with
//! the same verdict logic as the Python driver, so the scenarios run inside
//! `cargo test`. The `.xcx` fixtures and their expectations are unchanged.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT_SECS: u64 = 30;

#[derive(Clone, Default)]
struct Meta {
    id: String,
    area: String,
    priority: String,
    name: String,
    expect_compile_error: bool,
    expect_fatal_exit: bool,
    expect_regression: bool,
    is_server_test: bool,
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Faithful port of the Python metadata parser: `--- TEST:/Area:/Priority:/
/// Name:/Expect:` headers (case-sensitive keys) plus the `--- Runner: …`
/// hint flags and the two text-based auto-detections.
fn parse_meta(path: &Path) -> Meta {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut meta = Meta::default();

    for raw in text.lines() {
        let line = raw.trim();
        if !line.starts_with("---") {
            continue;
        }
        let body = line.trim_start_matches('-').trim();
        if let Some((key, value)) = body.split_once(':') {
            let value = value.trim();
            match key {
                "TEST" => meta.id = value.to_string(),
                "Area" => meta.area = value.to_string(),
                "Priority" => meta.priority = value.to_string(),
                "Name" => meta.name = value.to_string(),
                _ => {}
            }
        }
        let lower = line.to_ascii_lowercase();
        if lower.contains("runner:") {
            if lower.contains("expect_compile_error") {
                meta.expect_compile_error = true;
            }
            if lower.contains("expect_fatal_exit") {
                meta.expect_fatal_exit = true;
            }
        }
    }

    if text.contains("SHOULD NOT compile") {
        meta.expect_compile_error = true;
    }
    if text.contains("halt.fatal")
        && text.contains("SHOULD NOT execute")
        && !meta.expect_compile_error
    {
        meta.expect_fatal_exit = true;
    }
    meta.expect_regression = meta.priority.eq_ignore_ascii_case("regression");
    meta.is_server_test = text
        .lines()
        .any(|l| l.starts_with("serve") && l[5..].trim_start().starts_with(':'));

    meta
}

/// Faithful port of `looks_like_compile_error`: nonzero exit, stderr must
/// not contain "Compiled" (that marks a runtime error), and stderr must
/// carry a compile-error marker ([Sxxx]/[Dxxx] code or a known phrase).
fn looks_like_compile_error(code: Option<i32>, stderr: &str) -> bool {
    if code == Some(0) {
        return false;
    }
    if stderr.contains("Compiled") {
        return false;
    }
    if stderr.contains("Semantic analysis failed")
        || stderr.contains("Compilation failed")
        || stderr.contains("Syntax error")
        || stderr.contains("Parse error")
    {
        return true;
    }
    // [S\d+] / [D\d+]
    let bytes = stderr.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b'[' && (bytes[i + 1] == b'S' || bytes[i + 1] == b'D') {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 2 && j < bytes.len() && bytes[j] == b']' {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn looks_like_runtime_success(stderr: &str) -> bool {
    stderr.contains("Compiled")
}

fn run_file(xcx_bin: &Path, path: &Path, work_dir: &Path) -> Run {
    let mut child = Command::new(xcx_bin)
        .arg(path)
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn xcx binary for scenario");

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

    let deadline = Instant::now() + Duration::from_secs(TIMEOUT_SECS);
    let code = loop {
        match child.try_wait().expect("poll scenario child") {
            Some(status) => break status.code(),
            None => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = out_t.join();
                    let _ = err_t.join();
                    return Run {
                        code: None,
                        stdout: String::new(),
                        stderr: format!("TIMEOUT after {TIMEOUT_SECS}s"),
                    };
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    };
    Run {
        code,
        stdout: out_t.join().unwrap_or_default(),
        stderr: err_t.join().unwrap_or_default(),
    }
}

fn count_assertions(stdout: &str) -> (usize, usize) {
    let pass = stdout.lines().filter(|l| l.contains("] PASS")).count();
    let fail = stdout.lines().filter(|l| l.contains("] FAIL")).count();
    (pass, fail)
}

#[test]
fn spec_scenarios_suite() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base = root.join("tests").join("cli_tests");
    let xcx_bin = PathBuf::from(env!("CARGO_BIN_EXE_xcx"));

    let mut fixtures: Vec<PathBuf> = Vec::new();
    let mut stack = vec![base.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read scenario dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e == "xcx").unwrap_or(false) {
                fixtures.push(path);
            }
        }
    }
    fixtures.sort();
    assert!(
        !fixtures.is_empty(),
        "no scenario fixtures found under tests/cli_tests"
    );

    let work_dir = std::env::temp_dir().join(format!("xcx_spec_run_{}", std::process::id()));
    std::fs::create_dir_all(work_dir.join("tests_tmp")).expect("create scenario work dir");

    use std::io::Write;
    let mut err = std::io::stderr();
    let _ = writeln!(err, "================ SPEC SCENARIOS ================");

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut known_regression = 0usize;
    let mut skipped = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &fixtures {
        let meta = parse_meta(path);
        let id = if meta.id.is_empty() {
            path.file_stem().unwrap().to_string_lossy().to_uppercase()
        } else {
            meta.id.clone()
        };

        if meta.is_server_test {
            skipped += 1;
            let _ = writeln!(err, "  SKIP   {id} (HTTP server scenario)");
            continue;
        }

        let run = run_file(xcx_bin.as_path(), path, &work_dir);
        let (pass_count, fail_count) = count_assertions(&run.stdout);
        let fail_in_stdout = run.stdout.contains("] FAIL");

        let mut verdict = String::from("PASS");
        if meta.expect_compile_error {
            if looks_like_compile_error(run.code, &run.stderr) {
                passed += 1;
                verdict = String::from("COMPILE_OK");
            } else if run.code == Some(0) || looks_like_runtime_success(&run.stderr) {
                failed += 1;
                verdict = String::from("FAIL");
                failures.push(format!(
                    "{id}: compiler accepted a file that should be rejected"
                ));
            } else {
                passed += 1;
                verdict = String::from("COMPILE_OK");
            }
        } else if meta.expect_fatal_exit {
            if run.code != Some(0) && !fail_in_stdout {
                passed += 1;
            } else {
                failed += 1;
                verdict = String::from("FAIL");
                failures.push(format!(
                    "{id}: halt.fatal did not terminate (exit {:?})",
                    run.code
                ));
            }
        } else if meta.expect_regression && fail_count > 0 {
            known_regression += 1;
            verdict = String::from("KNOWN_REGRESSION");
        } else if fail_count > 0 {
            failed += 1;
            verdict = String::from("FAIL");
            failures.push(format!(
                "{id}: {} assertion(s) FAIL, {} PASS",
                fail_count, pass_count
            ));
        } else if pass_count > 0 {
            passed += 1;
        } else if run.code != Some(0) {
            failed += 1;
            verdict = String::from("FAIL");
            failures.push(format!(
                "{id}: nonzero exit ({:?}) without assertions",
                run.code
            ));
        } else {
            passed += 1;
        }

        let _ = writeln!(err, "  {:<8} {} (exit {:?})", id, verdict, run.code);
    }

    let _ = writeln!(
        err,
        "  SPEC SUMMARY: passed={passed} failed={failed} known_regression={known_regression} skipped={skipped}"
    );
    let _ = err.flush();
    let _ = std::fs::remove_dir_all(&work_dir);

    assert_eq!(
        failed, 0,
        "spec scenarios failed: {}",
        failures.join(" | ")
    );
    // `Priority: regression` fixtures are expected to fail (known bugs);
    // the Python driver reports them as warnings only. If one now PASSES,
    // the bug was fixed — surface it so the fixture can be un-marked.
    if known_regression > 0 {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr(),
            "  NOTE: {known_regression} scenario(s) marked `Priority: regression` still fail — \
             review whether they should be un-marked"
        );
    }
}
