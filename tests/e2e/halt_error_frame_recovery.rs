use std::process::Command;

/// Regression (ISSUE-007): when `halt.error` aborts a frame, top-level
/// recovery must hand the call site the callee's declared return type's
/// default (`0` for `-> i`, `""` for `-> s`) — never an untyped boolean
/// `false` leaking from an untouched register.
fn run_halt_recovery(tag: &str, engine_flag: Option<&str>) -> (String, bool) {
    let src = "func divide(i: a, i: b -> i) {\n\
               if (b == 0) then;\n\
               halt.error >! \"Division by zero!\";\n\
               return 0;\n\
               end;\n\
               return a / b;\n\
               };\n\
               func greet_or_fail(i: mode -> s) {\n\
               if (mode == 0) then;\n\
               halt.error >! \"greet failed\";\n\
               return \"x\";\n\
               end;\n\
               return \"hi\";\n\
               };\n\
               >! \"INT_OK=\" + divide(10, 2);\n\
               >! \"INT_REC=\" + divide(5, 0);\n\
               s: g = greet_or_fail(0);\n\
               >! \"STR_REC=[\" + g + \"]\";\n";

    let dir = std::env::temp_dir().join("xcx_halt_error_recovery_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join(format!("halt_recovery_{}.xcx", tag));
    std::fs::write(&file, src).expect("write temp xcx source");

    let exe = env!("CARGO_BIN_EXE_xcx");
    let mut cmd = Command::new(exe);
    if let Some(flag) = engine_flag {
        cmd.arg(flag);
    }
    cmd.arg(&file);
    let out = cmd.output().expect("failed to run xcx binary");

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    (stdout, out.status.success())
}

fn assert_halt_recovery_output(stdout: &str) {
    assert!(
        stdout.contains("INT_OK=5"),
        "normal call path broken; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("INT_REC=0"),
        "recovered `-> i` call must yield the typed default 0; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("STR_REC=[]"),
        "recovered `-> s` call must yield the typed default empty string; stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("INT_REC=false"),
        "recovered call returned the untyped false (type hole); stdout: {}",
        stdout
    );
}

#[test]
fn halt_error_recovery_returns_typed_default() {
    let (stdout, success) = run_halt_recovery("jit", None);
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert_halt_recovery_output(&stdout);
}

#[test]
fn halt_error_recovery_returns_typed_default_no_jit() {
    let (stdout, success) = run_halt_recovery("nojit", Some("--no-jit"));
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert_halt_recovery_output(&stdout);
}
