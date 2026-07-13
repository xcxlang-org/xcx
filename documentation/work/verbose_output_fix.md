# Fix: Ignore Parent Directory Name when Detecting Test Binary

## What was changed
Optimized `is_verbose_enabled()` in `src/runtime/builtin/io/mod.rs` to inspect only the executable's filename (via `p.file_name()`) instead of checking the entire absolute path for `"test"`, `"runner"`, or `"deps"`.

## Why
When the project compiles or runs inside a folder path containing the string "runner" (such as a GitHub Actions virtual machine workspace located under `/Users/runner/work/`), the absolute path matches the check `path.contains("runner")`. This incorrectly causes the binary CLI tool (`xcx-compiler`) to silence its stdout, preventing any verification output from reaching the test runner, which in turn causes the stability suite tests (e.g. `JSON-002`, `COL-001`) to fail due to missing output.

## Modified Files
- [mod.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/runtime/builtin/io/mod.rs)

## Verification Results
- **Windows locally**: All stability tests pass (`stability_suite::run_xcx_stability_suite ... ok`).
- **Target environment (macOS)**: Resolves false-positive silencing of test outputs when executing stability tests under `/Users/runner/` paths.
