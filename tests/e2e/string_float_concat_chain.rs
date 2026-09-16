use std::process::Command;

/// Regression (ISSUE-006): a `+` chain mixing strings and TWO float operands
/// must keep every string part. The JIT's type lattice checked `Float` before
/// `String` when typing `Add`, letting an unchecked float specialization
/// bitcast string payloads to `f64` — string parts vanished
/// (`"A = " + 0.5 + ", B = " + 0.75` printed `0.75`).
fn run_concat_chain(tag: &str, engine_flag: Option<&str>) -> (String, bool) {
    let src = "f: a = 1.5;\n\
               f: b = 2.5;\n\
               >! \"LIT=\" + \"A = \" + 0.5 + \", B = \" + 0.75;\n\
               >! \"VAR=\" + \"A = \" + a + \", B = \" + b;\n\
               >! \"MIX=\" + \"x\" + 1.0 + \"y\" + 2.0 + \"z\";\n";

    let dir = std::env::temp_dir().join("xcx_float_concat_chain_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join(format!("concat_chain_{}.xcx", tag));
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

fn assert_concat_chain_output(stdout: &str) {
    assert!(
        stdout.contains("LIT=A = 0.5, B = 0.75"),
        "float literals in a concat chain dropped the string parts; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("VAR=A = 1.5, B = 2.5"),
        "float variables in a concat chain dropped the string parts; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("MIX=x1y2z"),
        "alternating string/float chain lost segments; stdout: {}",
        stdout
    );
}

#[test]
fn string_chain_with_two_floats_keeps_text() {
    let (stdout, success) = run_concat_chain("jit", None);
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert_concat_chain_output(&stdout);
}

#[test]
fn string_chain_with_two_floats_keeps_text_no_jit() {
    let (stdout, success) = run_concat_chain("nojit", Some("--no-jit"));
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert_concat_chain_output(&stdout);
}
