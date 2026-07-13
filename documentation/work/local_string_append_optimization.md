# Local String Append Optimization (StrAppendLocal)

Support in-place string concatenation for local variables under the pattern `local_str = local_str + expr`.

## What was changed

- **Compiler (AST & HIR)**:
  - Added compile-time pattern matching in `compile_stmt.rs` and `src/hir/compile_hir.rs` for `local_str = local_str + expr` assignments when `local_str` is declared/inferred as `Type::String`.
  - It now emits the new opcode `OpCode::StrAppendLocal { local_idx, src }` instead of the generic sequence `Add` + `Move`.

- **VM Interpreter (`src/vm/core/step/mod.rs`)**:
  - Implemented the execution logic for `StrAppendLocal`.
  - Added a Copy-On-Write (COW) safety check: if `Arc::strong_count <= 1` and `Arc::weak_count == 0`, the suffix is appended directly to the existing buffer in-place. Otherwise, it falls back to allocating and cloning a new buffer.

- **JIT Compiler (`compiler_method.rs`, `analysis.rs`, `jit_helpers.rs`, `symbols/mod.rs`)**:
  - Registered and implemented the FFI helper `xcx_jit_str_append_local` with identical COW safety semantics.
  - Configured Cranelift assembly generation to spill registers before calling the helper and perform `reload_local` immediately after to preserve local variable cohesion.

- **Tests (`src/vm/core/tests.rs`)**:
  - Added `test_local_string_append_copy_on_write` to assert correct COW isolation behavior when multiple variables reference the same string header.

## Why

- Eliminates the O(n²) allocation and memory copy overhead on sequential local string concatenations.
- Reaches parity with the previously implemented `StrAppendVar` optimization designed for global variables.

## Which files were modified

- [src/hir/compile_hir.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/hir/compile_hir.rs)
- [src/compiler/compile_stmt.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/compiler/compile_stmt.rs)
- [src/vm/core/step/mod.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/core/step/mod.rs)
- [src/vm/core/tests.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/core/tests.rs)
- [src/vm/core/jit_helpers.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/core/jit_helpers.rs)
- [src/jit/compiler_method.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/compiler_method.rs)
- [src/jit/analysis.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/analysis.rs)
- [src/jit/symbols/mod.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/symbols/mod.rs)
- [src/jit/builder.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/builder.rs)
- [TODO.md](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/TODO.md)
- [documentation/changelogs/xcx_4.2_changelog.md](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/documentation/changelogs/xcx_4.2_changelog.md)

## Benchmark Results Before and After

| Scenariusz / Tryb | Przed | Po |
|---|---|---|
| Local String Concat (Interpreter) | ~1300 ms | 3.6 ms |
| Local String Concat (JIT) | ~1300 ms | 2.5 ms |
