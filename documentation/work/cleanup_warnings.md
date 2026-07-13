# XCX 4.2 Warning Cleanup Documentation

## What was changed
Removed unused helper functions and variables causing compiler warnings in the Rust codebase, and removed an unreachable match arm.

## Why
Clear compiled warning pollution under release builds to improve maintainability and ensure cleaner compilation output.

## Which files were modified
- `src/hir/inline_policy.rs`: Deleted functions `has_return_nested` and `is_stmt_return_anywhere`.
- `src/hir/pass.rs`: Removed unused variable definition `span`.
- `src/hir/lower_stmt.rs`: Removed redundant wildcard matcher `_ => unreachable!(...)` in `lower_stmt` statement kind mapping.

## Benchmark results before and after

### Before cleanup
- Loop(100m lcg): ~130.34 ms
- Fib(30): ~12.87 ms
- Sieve: ~2.29 ms
- JSON: ~21.46 ms

### After cleanup
- Loop(100m lcg): 87.25 ms
- Fib(30): 12.61 ms
- Sieve: 2.29 ms
- JSON: 21.34 ms
