use std::process::Command;

/// Regression: bundled packages (mathlib) ship in the installation's
/// `lib/` directory and must resolve from any project without a local
/// `lib/` copy. In 4.4 `include "math.xcx"` only searched the project,
/// so the shipped mathlib was unusable without copying it in.
///
/// The test seeds the installation layout next to the test binary
/// (`<exe_dir>/../lib/mathlib/src/math.xcx`) and runs a project from a
/// different directory that includes it with no local lib/.
#[test]
fn bundled_mathlib_resolves_from_installation_lib() {
    let exe = env!("CARGO_BIN_EXE_xcx").to_string();
    let exe_dir = std::path::Path::new(&exe)
        .parent()
        .expect("exe has a parent dir");
    let install_lib = exe_dir.parent().unwrap().join("lib");
    let mathlib_src = install_lib.join("mathlib").join("src");
    std::fs::create_dir_all(&mathlib_src).expect("create installation lib dir");
    let bundled = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("lib")
        .join("mathlib")
        .join("src")
        .join("math.xcx");
    std::fs::copy(&bundled, mathlib_src.join("math.xcx")).expect("seed bundled mathlib");

    let dir = std::env::temp_dir().join("xcx_bundled_lib_test");
    std::fs::create_dir_all(&dir).expect("create temp project dir");
    let file = dir.join("bundled_include.xcx");
    std::fs::write(
        &file,
        "include \"mathlib/src/math.xcx\";\n>! is_pow2(64);\n>! is_pow2(96);\n",
    )
    .expect("write fixture");

    let out = Command::new(&exe)
        .arg(&file)
        .current_dir(&dir)
        .output()
        .expect("run xcx on bundled-include fixture");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();

    assert!(
        out.status.success(),
        "include from the installation lib failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("true") && stdout.contains("false"),
        "unexpected output: {}",
        stdout
    );
}
