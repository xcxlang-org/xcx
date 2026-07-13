# Refactor: Rename Compiler Package and Target Binary to xcx

## What was changed
1. Renamed the package name from `xcx-compiler` to `xcx` in `Cargo.toml`.
2. Updated all imports and references to `xcx_compiler` in `src/main.rs` to `xcx`.
3. Updated unit and integration test imports and references to `xcx_compiler` in `tests/xcx_runner.rs` and `tests/json_concurrency.rs` to `xcx`.
4. Adjusted output binary target search paths (lookup from `xcx-compiler.exe` / `xcx-compiler` to `xcx.exe` / `xcx`) in:
   - `tests/xcx_runner.rs`
   - `tests/cli_tests/runner.py`
   - `linux/build_linux_pkg.sh`
   - `macOS/build_macOS_pkg.sh`
   - `src/runtime/builtin/io/print.rs`

## Why
Using `xcx-compiler` as the output binary name was inconvenient and annoying for the user who preferred the shorter and standard `xcx` name (e.g. command execution in terminal).

## Modified Files
- `Cargo.toml`
- `src/main.rs`
- `src/runtime/builtin/io/print.rs`
- `tests/xcx_runner.rs`
- `tests/json_concurrency.rs`
- `tests/cli_tests/runner.py`
- `linux/build_linux_pkg.sh`
- `macOS/build_macOS_pkg.sh`

## Verification Results
- **Compilation**: Successfully compiled using the new `xcx` name target under `cargo build --release`.
- **Integration & Unit tests**: All 159 tests successfully compiled and passed under `cargo test --release`.
