# XCX 4.1 Changelog

---

## JIT-to-JIT Direct Call Dispatch

Cross-function calls between JIT-compiled functions no longer re-enter the interpreter or go through the `xcx_jit_call_recursive` FFI helper. When a JIT-compiled function calls another function (Case 2: `func_idx != ctx.self_func_idx`), the compiler now checks whether the callee already has a compiled JIT pointer. If the pointer is non-null, a Cranelift `call_indirect` is emitted directly — bypassing the interpreter entirely. If the pointer is null (callee not yet compiled), the existing slow-path FFI fallback is used.

**Implementation changes:**
- `codegen_ctx.rs` — `CodegenCtx` extended with `stack_ptr_offset: u32` and `functions: Option<&[Arc<Chunk>]>`. New `set_functions` helper binds callee chunks to the codegen context.
- `compiler_method.rs` / `compiler_fiber.rs` — `compile_method` and `compile_fiber_segment` now receive and forward the `functions` slice. `stack_ptr` offset inside `Executor` is computed via `MaybeUninit` and passed to `CodegenCtx`.
- `emit_call.rs` — fast path loads the callee's `jit_ptr` atomically, checks `call_depth` against the recursion limit, increments depth and advances `stack_ptr` by `callee_chunk.max_locals`, executes `call_indirect`, then restores both on return. SSA variable (`ctx.b.declare_var`) used for the return status instead of Cranelift block parameters.
- `vm.rs`, `executor.rs`, `jit_helpers.rs` — call sites for `jit.compile_method` updated to pass the current `functions` slice from `SharedContext`.

## Eager Callee JIT Compilation

Before generating IR for a function, the compiler now scans its bytecode for `OpCode::Call` instructions and pre-compiles any callees that haven't been compiled yet. This ensures the fast path (direct `call_indirect`) is available immediately on first use rather than falling back to the FFI slow path.

Cycle detection uses an `in_progress: HashSet<usize>` field on the `JIT` struct — when a recursive dependency is detected, compilation returns `null` immediately, preventing infinite recursion in the compiler. Callee JIT pointers are stored with atomic `Release` writes.

## Inlined Collection Size in JIT

`.size()`, `.len()`, and `.count()` on `Array`, `BoolArray`, and `Map` values no longer go through FFI. The JIT now emits a direct 64-bit load from offset 24 of the receiver's base pointer (8-byte `RwLock` header + 16-byte `ptr`/`cap` fields of the inner `Vec`). This was verified by a dedicated offset test (`test_rwlock_array_offsets`).

The FFI symbols `xcx_jit_array_size` and `xcx_jit_map_size` are no longer emitted.

## Pointer Analysis Optimization

`analyze_maybe_ptr_regs` in `analysis.rs` has been rewritten using a 256-bit bitmask (`[u64; 4]`) instead of `[bool; 256]`. The dataflow merge loop now operates on four 64-bit OR operations per successor instead of iterating 256 booleans. `CodegenCtx.may_contain_ptr` updated to `Vec<[u64; 4]>`. The `should_skip_dec_ref` check now uses a direct bitmask test.

## `--threshold` / `--th` Flag

The JIT hotspot threshold — minimum number of executions before a fiber segment is compiled — is now configurable via CLI:

```
xcx script.xcx --threshold=100
xcx script.xcx --th=25
```

Default is 50. Invalid values (non-integer) exit with code 1:

```
Error: --threshold requires a valid unsigned integer.
```

The value is stored in `vm.jit_threshold` and applied to `Hotspot` at executor initialization.

## Array `.slice()` and JSON `.keys()`

Two new built-in methods:

- **`array.slice(start, end)`** — returns a new array with elements in the half-open range `[start, end)`. Parameters are clamped symmetrically, matching the behavior of `.slice()` on strings.
- **`json.keys()`** — returns a `string[]` of all top-level keys in a JSON object.

Both are supported in the type checker (`check_expr_methods.rs`) and the VM (`array/ops.rs`). Documentation updated in `collections.md` and `json_http.md`.

The `.count()` alias for `.size()` / `.len()` was already fully implemented — documentation corrected to reflect this.

## REPL Improvements

- `Ctrl+A` / `Ctrl+E` — jump to beginning / end of line.
- Tab always inserts 4 spaces.
- Commands prefixed with `!` evaluate immediately.
- New diagnostic commands: `!globals` (inspect global variable state), `!jit` (JIT info and management), `!reset` (clear interpreter state).
- Fixed a panic in the REPL JIT compiler caused by `constants` not being cleared between sessions (`constants.clear()`).

## SQLite FFI Parity

Audited all SQLite modules (`connection.rs`, `ddl.rs`, `read.rs`, `write.rs`, `delete.rs`) for divergence between JIT and interpreter behavior.

**Halt errors** (argument validation failures, critical DDL errors) increment `error_count` and return `OpResult::Halt` — identical in both execution modes.

**Soft errors** (constraint violations, failed queries, connection errors) return `false` to the destination register and log a warning — they do not increment `error_count`. Previously, some soft errors incremented the counter, causing the JIT's `emit_halt_if_errors` check to halt execution while the interpreter continued. This divergence is now eliminated. `DB-004_transaction_auto_rollback` passes under both JIT and `--no-jit`.

## `ACTIVE_VM` and Safe FFI Error Reporting

A thread-local `ACTIVE_VM: Cell<*const VM>` pointer has been added to `vm.rs`. A RAII guard (`ActiveVmGuard`) manages its lifetime across `run_frame`, `handle_call_no_jit`, and `dispatch_jit_call`. This enables a safe `increment_error_count()` FFI function callable from array and other runtime modules, fixing correct reporting of **R303 (Index out of bounds)** and removing a Cranelift static optimization asymmetry.

## PAX `upgrade` Command

PAX gains a self-update command for the compiler and standard library:

```
xcx pax upgrade xcx          # update the xcx binary
xcx pax upgrade tools        # update stdlib and pax.xcx
xcx pax upgrade xcx --check  # print whether a newer version is available
```

**Binary download** — `build_response_json` now reads responses into a raw byte buffer via `resp.into_reader().read_to_end(&mut buf)` and converts with `unsafe { String::from_utf8_unchecked(buf) }`. This preserves binary content that would otherwise be corrupted or rejected by UTF-8 validation.

**Path bypass** — `validate_path_safety` now permits writes to paths under the XCX installation directory (`std::env::current_exe()` base), while still rejecting `..` traversal.

**Windows EXE lock** — when `std::fs::write` fails on a `.exe` due to an OS file lock, the runtime automatically renames the running binary to `<name>.exe.old` and retries the write.

**String cast fix** — `CastString` on a value that is already a `String` now bypasses `as_string_lossy()` and returns the existing buffer directly, preventing binary data from being mangled by lossy UTF-8 conversion (previously inflating an 18MB binary to 27MB).

A shell installer (`install.sh`) for Linux and macOS is included — downloads the latest stable binary from GitHub Releases, sets up PAX and the `VERSION` file under `~/.local/share/xcx/`, then bootstraps the stdlib via `upgrade tools`.

## Technical Debt Reduction

**Dead parameters and methods removed:**
- `_func_id` parameter removed from `run_frame` and all call sites.
- `dispatch_method`, `dispatch_method_named`, `dispatch_method_inner` merged into a single `dispatch_method(..., names: Option<&[String]>)`.
- `_dst` removed from `handle_call`.
- `_vm_ptr` and `_glbs_ptr` removed from `xcx_jit_call_recursive` FFI — two unnecessary ABI argument emissions eliminated per slow-path call.
- `locals_ptr` and `dst` removed from `xcx_jit_method_dispatch` and `xcx_jit_method_dispatch_named`.
- `_base` / `base` removed from `handle_method_call_custom` and `handle_json_custom`.
- `get_global_idx` compiler helper removed; references in unit tests replaced with direct `compiler.globals` dictionary lookups.

**Constants and deduplication:**
- `JIT_WARMUP_THRESHOLD` constant removed; replaced by `vm.jit_threshold`.
- `RECURSION_LIMIT: usize = 800` defined in `executor.rs`; all hardcoded `800` literals replaced across interpreter, JIT codegen, and FFI helpers.
- `SKELETON_CHUNK_NAME: &str = "skeleton"` defined in `globals.rs`.
- `BUILT_INS: [&str; 5]` constant defined in `compiler.rs`, replacing two duplicate inline arrays.
- `Span::default()` implemented in `span.rs`; dummy `Span { line: 0, col: 0, len: 0 }` constructions replaced.

**Extraction and deduplication:**
- `executor_field_offsets()` extracted to `codegen_ctx.rs` — `MaybeUninit` offset computation for `call_depth` and `stack_ptr` is no longer duplicated between `compiler_method.rs`, `compiler_fiber.rs`, and `jit.rs`.
- `extract_method_args()` extracted in `step/mod.rs` — `copy_from_slice`-based argument copy previously duplicated across `MethodCall`, `MethodCallCustom`, and `MethodCallNamed`.
- `make_str_key` and `make_map_pair` extracted in `compile_stmt.rs` for `NetRequestStmt` codegen.
- `sqlite_row_to_value` helper extracted to `vm/utils/table.rs`; identical row-mapping logic removed from `runtime_ops.rs` and `table/select.rs`.

**JIT pre-analysis cleanup:**
- Duplicate `analyze_chunk_types` call removed from `compiler_method.rs` (first result was immediately overwritten).
- Dead `creates_ptrs` loop and `_pure_func` flag removed.
- Dead JIT compile-time instrumentation removed (`Instant::now()`, `chunk.name.clone()`, `elapsed()`).
- Dead first `ctx.preload_locals` call removed from `compiler_fiber.rs`.

**Allocations:**
- `collect_all_regs` and the liveness loop now share a pre-allocated `Vec<u8>` buffer, cleared with `.clear()` each iteration — eliminates per-iteration `Vec` allocation in the register compressor hot path.

**Table/index cleanup:**
- Dead comments, disabled `eprintln!` calls removed from `src/runtime/builtin/table/index.rs`.

**Warning-free build:**
- Unused imports, constants, and struct fields removed from `tests/xcx_runner.rs`. `cargo test --release` produces 0 warnings.

## Bugfixes and Corner Cases

- **Block Comment Fallback Check:** Empty or whitespace-only code comments starting with `---` no longer compile as multi-line block comments unless a matching `*---` token is present in the remaining source. This prevents unclosed block remarks from swallowing the remainder of a file.

## Error Code Test Suite

New integration tests in `tests/cli_tests/error_suite/` cover VM responses to semantic compile-time errors and runtime errors (R306, R307 and others). The `disable_jit: true` meta-tag enables precise interpreter-path testing.

## Multiple Variable Declarations

Declaring multiple variables of the same basic type in a single statement (e.g. `i: a, b = 42, c;` or `var x = 1, y = 2.0;`) is now supported. This is implemented via compiler-level desugaring in the parsing stage into a sequence of individual `VarDecl` statements, avoiding any overhead or changes in the execution engine, semantic type checker, or JIT.

**Implementation details:**
- `stmt.rs` — New `StmtKind::MultiVarDecl(Vec<Stmt>)` holding a list of individual `VarDecl` statements.
- `visitor.rs` — Updated to traverse the nested statements of `MultiVarDecl`.
- `parse_decl.rs` — Modified `parse_var_decl` to loop over comma-separated identifiers and initializers for basic/primitive types (`i`, `f`, `s`, `b`, `json`) and inferred type fields (`var`). Returns a single `VarDecl` for a case with a single variable, and wraps in `MultiVarDecl` for multiple. Excludes complex types (`Map`, `Table`, `Database`).
- `check_stmt.rs` — Simply delegates checking to `check_stmt` on each nested statement.
- `compile_stmt.rs` — Sequentially compiles each nested statement to bytecode.
- `globals.rs` — `register_globals_recursive` recurses into `MultiVarDecl` to ensure variables declared in the main script are correctly registered as global variables.

---

| Benchmark | XCX 4.0 (no JIT) | XCX 4.0 (JIT) | XCX 4.1 (no JIT) | XCX 4.1 (JIT) |
|---|---|---|---|---|
| Loop 100M | 7341ms | 119ms | 7340ms | **116.27ms** |
| Fib(30) | 260ms | ~14ms | 180ms | **12.87ms** |
| Sieve 100K | 23ms | ~2.5ms | 22ms | **2.29ms** |
| JSON 1000×100 | 66ms | ~23ms | 64ms | **21.46ms** |
| Cross-func call 1M | — | 47ms | — | **25ms** (~47% reduction) |

*Tested on AMD Ryzen 7 5800X, 32 GB RAM, Windows 11.*