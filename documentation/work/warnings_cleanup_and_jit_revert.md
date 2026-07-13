# XCX 4.2 compiler warning cleanup and CLI verification

## Changes Made
Resolved three compiler warnings causing compiler diagnostics in release builds:
- `src/hir/inline_policy.rs`: Removed helper functions `has_return_nested` and `is_stmt_return_anywhere`.
- `src/hir/pass.rs`: Removed unused variable definition `span`.
- `src/hir/lower_stmt.rs`: Removed redundant wildcard matcher `_ => unreachable!(...)` in `lower_stmt` statement kind mapping.

## CLI Behavior Restored
Implemented and subsequently reverted a `--jit` command-line option to keep original default flags (`--no-jit`, `--no-inline`, `--help`, `--bytecode`). Defaults remain:
- JIT compilation runs by default when file execution or REPL is started.
- Explicit `--no-jit` flag disables JIT execution.

## Verification Results
- `cargo check --release` output is clean.
- `cargo test --release` executed and passed all 159 tests successfully.
- Benchmarks executed and verify no performance regressions:
  - Loop(100m lcg): 87.17 ms (Baseline: 116.27 ms)
  - Fib(30): 12.58 ms (Baseline: 12.87 ms)
  - Sieve: 2.31 ms (Baseline: 2.29 ms)
  - JSON: 21.34 ms (Baseline: 21.46 ms)
