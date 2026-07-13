# JIT Global String Append Invalidation Bugfix

A bug was discovered and resolved where modifying a global string via the `StrAppendVar` optimization inside JIT-compiled code did not invalidate the JIT's cached global variables, causing subsequent global reads to yield obsolete string values. A regression test for global string append copy-on-write isolation was added.

## What was changed
- Added `test_global_string_append_copy_on_write` integration test to `src/vm/core/tests.rs` to verify that when globals `a` and `b` reference the same string, appending to `a` (via COW) leaves `b` unmodified.
- Fixed `StrAppendVar` compilation in `src/jit/compiler_method.rs` by calling `ctx.reload_globals()` immediately following the FFI helper call. This forces Cranelift to synchronize its register/SSA state with updated global memory.

## Why
- Essential for correctness of string manipulation under the JIT compilation path.
- Restores type safety and cache consistency when mutating global state from JIT.

## Which files were modified
- [tests.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/core/tests.rs)
- [compiler_method.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/compiler_method.rs)

## Benchmark Results Before and After
- No performance regressions.
- Correctness restored: `test_global_string_append_copy_on_write` passes successfully on release profile.
