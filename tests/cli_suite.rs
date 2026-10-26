//! CLI stability suite (`tests/cli_tests/**/*.xcx`) driven by the real
//! binary, so `cargo test` runs the whole stability suite in one go.
//!
//! Verdict logic mirrors `tests/cli_tests/runner.py` 1:1 — both runners must
//! agree on every `.xcx` file. Header directives (`--- TEST:`, `--- Runner: …`)
//! and the `] PASS` / `] FAIL` stdout markers are the contract shared by the
//! two runners; keep them in sync when changing either side.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT_SECS: u64 = 30;

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

fn parse_meta(path: &Path) -> Meta {
    let text = fs::read_to_string(path).unwrap_or_default();
    let lower = text.to_lowercase();

    let mut meta = Meta {
        id: path.file_stem().map(|s| s.to_string_lossy().to_uppercase()).unwrap_or_default(),
        area: String::new(),
        priority: String::new(),
        name: String::new(),
        expect_compile_error: lower.contains("expect_compile_error") || lower.contains("should not compile"),
        expect_fatal_exit: false,
        expect_regression: false,
        is_server_test: false,
    };

    for line in text.lines() {
        let t = line.trim();
        for (key, pat) in [
            ("id", "--- TEST:"),
            ("area", "--- Area:"),
            ("priority", "--- Priority:"),
            ("name", "--- Name:"),
        ] {
            if let Some(rest) = t.strip_prefix(pat) {
                let val = rest.trim().to_string();
                match key {
                    "id" if !val.is_empty() => meta.id = val,
                    "area" => meta.area = val,
                    "priority" => meta.priority = val,
                    "name" => meta.name = val,
                    _ => {}
                }
            }
        }
        if t.starts_with("serve") && t["serve".len()..].trim_start().starts_with(':') {
            meta.is_server_test = true;
        }
    }

    meta.expect_fatal_exit = lower.contains("expect_fatal_exit")
        || (lower.contains("halt.fatal") && lower.contains("should not execute") && !meta.expect_compile_error);
    meta.expect_regression = meta.priority.eq_ignore_ascii_case("regression");
    meta
}

/// runner.py `COMPILE_ERROR_PATTERNS`: [Sxxx], [Dxxx] codes plus the
/// human-readable compiler failure phrases.
fn looks_like_compile_error(rc: Option<i32>, stderr: &str) -> bool {
    let Some(rc) = rc else { return false };
    if rc == 0 || stderr.contains("Compiled") {
        return false;
    }
    let has_error_code = |letter: char| {
        let b = stderr.as_bytes();
        let mut i = 0;
        while i + 1 < b.len() {
            if b[i] == b'[' && b[i + 1] == letter as u8 {
                let mut j = i + 2;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 2 && j < b.len() && b[j] == b']' {
                    return true;
                }
            }
            i += 1;
        }
        false
    };
    has_error_code('S')
        || has_error_code('D')
        || stderr.to_lowercase().contains("semantic analysis failed")
        || stderr.to_lowercase().contains("compilation failed")
        || stderr.to_lowercase().contains("syntax error")
        || stderr.to_lowercase().contains("parse error")
}

fn count_assertions(stdout: &str) -> (usize, usize) {
    let pass = stdout.lines().filter(|l| l.contains("] PASS")).count();
    let fail = stdout.lines().filter(|l| l.contains("] FAIL")).count();
    (pass, fail)
}

struct RunOutcome {
    rc: Option<i32>,
    stdout: String,
    stderr: String,
    duration_ms: u128,
    timed_out: bool,
}

fn run_test_file(exe: &str, path: &Path, work_dir: &Path) -> RunOutcome {
    let t0 = Instant::now();
    let child = Command::new(exe)
        .arg(path)
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child: Child = match child {
        Ok(c) => c,
        Err(e) => {
            return RunOutcome {
                rc: None,
                stdout: String::new(),
                stderr: format!("failed to spawn {}: {}", exe, e),
                duration_ms: t0.elapsed().as_millis(),
                timed_out: false,
            };
        }
    };

    // Drain both pipes on helper threads; a child blocked on a full pipe
    // would otherwise stall until the timeout kill and fake a TIMEOUT.
    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = out_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = err_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    });

    let deadline = t0 + Duration::from_secs(TIMEOUT_SECS);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => break None,
        }
    };

    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();
    RunOutcome {
        rc: status.map(|s| s.code().unwrap_or(1)),
        stdout,
        stderr,
        duration_ms: t0.elapsed().as_millis(),
        timed_out,
    }
}

/// The verdict: Ok(detail) = suite-pass (PASS / compile✓ / fatal✓),
/// Err(detail) = failure, None-like via SKIP variant below.
enum Verdict {
    Pass(String),
    Fail(String),
    Skip(String),
}

fn evaluate(meta: &Meta, run: &RunOutcome) -> Verdict {
    if meta.is_server_test {
        return Verdict::Skip("HTTP server test — requires external HTTP client".into());
    }
    if run.timed_out {
        return Verdict::Fail(format!("TIMEOUT after {}s", TIMEOUT_SECS));
    }

    if meta.expect_compile_error {
        if looks_like_compile_error(run.rc, &run.stderr) {
            return Verdict::Pass("Compiler correctly rejected file".into());
        }
        if run.rc == Some(0) || run.stderr.contains("Compiled") {
            return Verdict::Fail("Compiler accepted file that should be rejected".into());
        }
        return Verdict::Pass(format!("Compiler rejected file (exit {:?})", run.rc));
    }

    if meta.expect_fatal_exit {
        let fail_in_stdout = run.stdout.contains("] FAIL");
        if run.rc.is_some() && run.rc != Some(0) && !fail_in_stdout {
            return Verdict::Pass(format!("halt.fatal correctly terminated VM (exit {:?})", run.rc));
        }
        if fail_in_stdout {
            return Verdict::Fail("FAIL appeared in stdout — halt.fatal did not work".into());
        }
        return Verdict::Fail(format!("exit code = {:?} — expected nonzero (halt.fatal did not terminate VM)", run.rc));
    }

    let (pass_count, fail_count) = count_assertions(&run.stdout);
    if meta.expect_regression && fail_count > 0 {
        return Verdict::Pass("Known regression — FAIL result expected".into());
    }
    if fail_count > 0 {
        return Verdict::Fail(format!("{} assertions FAIL, {} PASS", fail_count, pass_count));
    }
    if pass_count > 0 {
        return Verdict::Pass(format!("All {} assertions PASS", pass_count));
    }
    if run.rc != Some(0) {
        if looks_like_compile_error(run.rc, &run.stderr) {
            return Verdict::Fail("Unexpected compile error (test should not have syntax errors)".into());
        }
        return Verdict::Fail(format!("Nonzero exit code ({:?}) without assertions in stdout", run.rc));
    }
    Verdict::Pass("Exit 0 (no assertions in stdout)".into())
}

#[test]
fn cli_stability_suite() {
    let exe = env!("CARGO_BIN_EXE_xcx");
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("cli_tests");

    let mut files: Vec<PathBuf> = walkdir(&base);
    files.sort();
    assert!(!files.is_empty(), "no .xcx tests found under {}", base.display());

    let work_dir = std::env::temp_dir().join(format!("xcx_cli_suite_{}", std::process::id()));
    fs::create_dir_all(work_dir.join("tests_tmp")).expect("create suite work dir");

    let mut passed = 0usize;
    let mut skipped = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for path in &files {
        let meta = parse_meta(path);
        let run = run_test_file(exe, path, &work_dir);
        let rel = path.strip_prefix(&base).unwrap_or(path).display().to_string();
        match evaluate(&meta, &run) {
            Verdict::Pass(detail) => {
                passed += 1;
                println!("  PASS  {:<14} {:>5}ms  {} — {}", meta.id, run.duration_ms, rel, detail);
            }
            Verdict::Skip(detail) => {
                skipped += 1;
                println!("  SKIP  {:<14} {:>5}ms  {} — {}", meta.id, run.duration_ms, rel, detail);
            }
            Verdict::Fail(detail) => {
                let mut report = format!(
                    "{} ({}) — {}\n    {}\n    stdout (last 8 lines):\n{}",
                    meta.id,
                    rel,
                    meta.name,
                    detail,
                    tail(&run.stdout, 8, "      ")
                );
                if !run.stderr.trim().is_empty() {
                    report.push_str(&format!("    stderr (last 4 lines):\n{}", tail(&run.stderr, 4, "      ")));
                }
                failures.push(report);
                println!("  FAIL  {:<14} {:>5}ms  {} — {}", meta.id, run.duration_ms, rel, detail);
            }
        }
    }

    let _ = fs::remove_dir_all(&work_dir);

    println!("\n  CLI suite: {} passed, {} skipped, {} failed (of {})", passed, skipped, failures.len(), files.len());
    if !failures.is_empty() {
        panic!("CLI stability suite failed:\n\n{}\n", failures.join("\n"));
    }
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            out.extend(walkdir(&p));
        } else if p.extension().map(|e| e == "xcx").unwrap_or(false) {
            out.push(p);
        }
    }
    out
}

fn tail(s: &str, n: usize, indent: &str) -> String {
    let lines: Vec<&str> = s.trim_end().lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..]
        .iter()
        .map(|l| format!("{}{}\n", indent, l))
        .collect()
}
