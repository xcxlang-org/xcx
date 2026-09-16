//! Runs last (the target name sorts after every other suite) and prints the
//! total regression-test count across the workspace, so `cargo test` ends
//! with one aggregate number instead of per-binary summaries only.
//!
//! The count is a static inventory of `#[test]` functions in `src/` and
//! `tests/` — exactly the tests cargo collects — so it stays current
//! automatically as suites grow.

use std::path::PathBuf;

#[test]
fn total_test_summary() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<(String, usize)> = Vec::new();
    let mut stack = vec![root.join("src"), root.join("tests")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read directory").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                // count attribute LINES only ("#[test]" at line start) so this
                // file's own doc comment and match-string do not self-count
                let n = content
                    .lines()
                    .filter(|l| l.trim_start().starts_with("#[test]"))
                    .count();
                if n > 0 {
                    let rel = path
                        .strip_prefix(&root)
                        .unwrap_or(&path)
                        .display()
                        .to_string()
                        .replace('\\', "/");
                    files.push((rel, n));
                }
            }
        }
    }
    files.sort();
    let total: usize = files.iter().map(|(_, n)| n).sum();
    // libtest captures print!-macro output; a direct write to the process
    // stderr bypasses that capture, so the summary stays visible at the end
    // of a plain `cargo test` run.
    use std::io::Write;
    let mut err = std::io::stderr();
    let _ = writeln!(err, "================ REGRESSION SUMMARY ================");
    for (file, n) in &files {
        let _ = writeln!(err, "  {n:4}  {file}");
    }
    let _ = writeln!(
        err,
        "  TOTAL: {total} tests in {} files — every suite above must report 0 failed",
        files.len()
    );
    let _ = err.flush();
    assert!(total > 0, "test inventory came up empty");
}
