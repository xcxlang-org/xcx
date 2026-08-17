use std::process::Command;

/// Spawns the real xcx binary against the link-local probe program and
/// asserts the SSRF guard fires. This runs as a separate process because
/// the guard raises halt.fatal as a panic on the executor thread, which
/// aborts the process when it crosses JIT FFI frames — in-process, that
/// would kill the test harness itself (the reason the original unit test
/// was ignored). The guard validates the URL before any network I/O, so
/// the test performs no actual network access.
#[test]
fn ssrf_link_local_is_fatal_in_subprocess() {
    let probe = concat!(env!("CARGO_MANIFEST_DIR"), "\\tests\\ssrf_link_local_probe.xcx");
    let output = Command::new(env!("CARGO_BIN_EXE_xcx"))
        .arg(probe)
        .output()
        .expect("failed to spawn xcx binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "link-local request must not succeed"
    );
    assert!(
        stderr.contains("SSRF"),
        "expected SSRF diagnostic on stderr, got: {}",
        stderr
    );
}
