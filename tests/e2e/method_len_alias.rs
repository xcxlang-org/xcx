use std::process::Command;

/// Regression test: `.len()` must behave as a documented alias of
/// `.size()` / `.count()` on arrays (collections.md), compiling and running
/// through the default (JIT) pipeline.
#[test]
fn array_len_alias_works() {
    let src = "array:i: arr;\n\
               arr.push(7);\n\
               arr.push(8);\n\
               i: ok = 0;\n\
               if (arr.len() == 2 && arr.size() == 2 && arr.count() == 2) then;\n\
               ok = 1;\n\
               end;\n\
               if (ok == 1) then;\n\
               >! \"LEN_ALIAS_OK\";\n\
               end;\n";

    let dir = std::env::temp_dir().join("xcx_len_alias_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("len_alias.xcx");
    std::fs::write(&file, src).expect("write temp xcx source");

    let exe = env!("CARGO_BIN_EXE_xcx");
    let out = Command::new(exe)
        .arg(&file)
        .output()
        .expect("failed to run xcx binary");

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "xcx exited with failure; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("LEN_ALIAS_OK"),
        "len alias did not produce the expected result; stdout: {}",
        stdout
    );
}

/// Same expectation for the interpreter (`--no-jit`) path.
#[test]
fn array_len_alias_works_no_jit() {
    let src = "array:i: arr;\n\
               arr.push(5);\n\
               i: ok = 0;\n\
               if (arr.len() == 1) then;\n\
               ok = 1;\n\
               end;\n\
               if (ok == 1) then;\n\
               >! \"LEN_ALIAS_OK_NO_JIT\";\n\
               end;\n";

    let dir = std::env::temp_dir().join("xcx_len_alias_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("len_alias_no_jit.xcx");
    std::fs::write(&file, src).expect("write temp xcx source");

    let exe = env!("CARGO_BIN_EXE_xcx");
    let out = Command::new(exe)
        .arg("--no-jit")
        .arg(&file)
        .output()
        .expect("failed to run xcx binary");

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "xcx exited with failure; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("LEN_ALIAS_OK_NO_JIT"),
        "len alias did not produce the expected result; stdout: {}",
        stdout
    );
}
