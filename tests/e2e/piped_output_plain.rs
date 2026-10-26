use std::process::{Command, Stdio};

/// Regression: `>!` output redirected to a pipe or file must be plain text.
/// In 4.4 every printed line carried the terminal erase sequence (ESC[K),
/// so scripts and CI had to strip escape codes from captured output.
/// Interactive terminals keep the redraw-friendly line endings (verified
/// manually; not assertable from a piped test harness).
#[test]
fn piped_output_is_plain_text() {
    let dir = std::env::temp_dir().join("xcx_piped_plain_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("piped_plain.xcx");
    std::fs::write(&file, ">! \"hello\";\n>! 42;\n").expect("write fixture");

    for flag in [None, Some("--no-jit")] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_xcx"));
        cmd.arg(&file).stdout(Stdio::piped());
        if let Some(f) = flag {
            cmd.arg(f);
        }
        let out = cmd.output().expect("run xcx with piped stdout");
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();

        assert!(out.status.success(), "run failed ({:?})", flag);
        assert!(
            !stdout.contains('\x1b'),
            "stdout carries an escape sequence ({:?}): {:?}",
            flag,
            stdout
        );
        assert!(
            stdout.contains("hello") && stdout.contains("42"),
            "output lost its values ({:?}): {:?}",
            flag,
            stdout
        );
        assert!(
            stdout.matches('\n').count() >= 2,
            "lines must still end with newlines ({:?}): {:?}",
            flag,
            stdout
        );
    }
}
