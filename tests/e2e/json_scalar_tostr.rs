use std::process::Command;

/// Regression (ISSUE-003a): `.toStr()` on scalar JSON values (number,
/// boolean) and on scalar elements fetched from a JSON array must serialize
/// the value, not return the missing-method `false`. The JIT used to take a
/// Json-typed fast path whose guard returned `false` for the raw scalar
/// receivers that `json` values legitimately materialize as.
fn run_scalar_tostr(marker: &str, engine_flag: Option<&str>) -> (String, bool) {
    let src = "json: n <<< 80 >>>;\n\
               json: flag <<< true >>>;\n\
               json: arr <<< [80, 443] >>>;\n\
               >! \"NUM=\" + n.toStr();\n\
               >! \"FLAG=\" + flag.toStr();\n\
               >! \"ELEM=\" + arr.get(0).toStr();\n";

    let dir = std::env::temp_dir().join("xcx_json_scalar_tostr_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join(format!("scalar_tostr_{}.xcx", marker));
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

fn assert_scalar_tostr_output(stdout: &str) {
    assert!(
        stdout.contains("NUM=80"),
        "scalar json number .toStr() must serialize to 80; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("FLAG=true"),
        "scalar json boolean .toStr() must serialize to true; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("ELEM=80"),
        ".toStr() on a scalar array element must serialize to 80; stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("NUM=false"),
        "scalar .toStr() fell back to the missing-method false; stdout: {}",
        stdout
    );
}

#[test]
fn scalar_json_tostr_serializes_under_jit() {
    let (stdout, success) = run_scalar_tostr("jit", None);
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert_scalar_tostr_output(&stdout);
}

#[test]
fn scalar_json_tostr_serializes_under_no_jit() {
    let (stdout, success) = run_scalar_tostr("nojit", Some("--no-jit"));
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert_scalar_tostr_output(&stdout);
}
