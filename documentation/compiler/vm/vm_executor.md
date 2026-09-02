# VM — Executor and VM Struct

This document covers the execution engine: the `VM` struct, `SharedContext`, `Executor`, the main dispatch loop, and the JIT FFI helpers. (A former trace-recording subsystem under `vm/trace/` is no longer part of the engine; see `documentation/work/2026-08-17_phase3b_tracejit_fiberjit_removal.md`.)

---

## Module Layout

```
src/vm/core/
├── vm.rs              — VM struct, SharedContext, OpResult
├── executor.rs        — Executor struct, dispatch_jit_call, jit_helpers
├── dispatch.rs        — handle_method_call / handle_method_call_custom
├── runtime_ops.rs     — RuntimeOps (get_member, table_init, etc.)
├── jit_helpers.rs     — #[no_mangle] extern "C" functions exposed to JIT
├── step/
│   ├── arith.rs       — arithmetic step handlers
│   ├── cast.rs        — cast step handlers
│   ├── collection.rs  — collection step handlers (SetRange, RowGet, etc.)
│   ├── compare.rs     — comparison step handlers
│   ├── control.rs     — control flow step handlers
│   ├── logic.rs       — And / Or / Not step handlers
│   ├── member.rs      — GetIndex / SetIndex / GetMember step handlers
│   ├── module.rs      — module opcode handlers (net, crypto, io, json, etc.)
│   └── mod.rs
└── mod.rs
```

---

## `VM` Struct

```rust
pub struct VM {
    pub globals:        parking_lot::RwLock<Vec<Value>>,
    pub global_names:   Arc<RwLock<HashMap<String, usize>>>,
    pub error_count:    AtomicUsize,
    pub jit:            Mutex<crate::jit::JIT>,
    pub disable_jit:    AtomicBool,
    pub jit_threshold:  u32,
    pub start_instant:  std::time::Instant,
}
```

The `VM` is created once per program execution and wrapped in `Arc<VM>` so it can be shared between the main executor and any spawned fiber executors. It holds the global variable array and the global error counter.

`VM::new()` pre-allocates 65536 global slots, sets `disable_jit = false`, and `jit_threshold = 50`.

### `get_global(idx) -> Value`

Used in tests and by the executor to read a global by index (returns `false` past the 65536-slot boundary). `set_global` handles refcounting.

### `run(chunk, ctx, args)`

Main entry point. Eagerly attempts a JIT compilation of the main chunk (when JIT is enabled and not yet compiled), then creates an `Executor` and calls `executor.run_frame(chunk, params, self)`. Runs on the dedicated `xcx-executor` thread (64MB stack) as spawned from `main`.

### `error_count`

`AtomicUsize`. Incremented by `HaltError`, division by zero, JIT error handlers, and method dispatch failures. After execution, `main` checks this and exits with code 1 if non-zero.

---

## `SharedContext`

```rust
pub struct SharedContext {
    pub constants: Arc<Vec<Value>>,
    pub functions: Arc<Vec<Arc<Chunk>>>,
    pub http_req:  Option<Arc<std::sync::Mutex<Option<tiny_http::Request>>>>,
}
```

Immutable data shared across all executors in the same program run. Wrapped in `Arc<SharedContext>`.

- `constants` — all compile-time constant values (strings, numbers, skeletons, etc.). Indexed by `LoadConst::idx`.
- `functions` — all compiled `Chunk`s except the top-level program chunk. Indexed by `Call::func_idx`.
- `http_req` — set to `Some` when the executor is running as an HTTP route handler; used by `HttpRespond` and `HttpRequest` opcodes.

---

## `OpResult`

```rust
pub enum OpResult {
    Continue,
    Return(Option<Value>),
    Yield(Option<Value>),
    YieldWithTarget(u8, Option<Value>),
    Halt,
}
```

Returned by every step handler. `Continue` proceeds to the next instruction. `Halt` stops execution. `Return`/`Yield`/`YieldWithTarget` carry (optionally) a value up to the parent frame during function returns and fiber yields.

---

## `SHUTDOWN` Flag

```rust
pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);
```

Set to `true` by the Ctrl-C handler in `main`. The executor checks this on each iteration of the dispatch loop (or on each loop-back jump) to gracefully stop long-running programs.

---

## `Executor`

```rust
pub struct Executor {
    pub vm:                   Arc<VM>,
    pub ctx:                  Arc<SharedContext>,
    pub current_spans:        Option<Arc<Vec<Span>>>,
    pub fiber_yielded:        bool,
    pub terminal_raw_enabled: bool,
    pub fiber_next_ip:        usize,
    pub current_bytecode_ptr: usize,
    pub stack:                Vec<Value>,
    pub stack_ptr:            usize,
    pub call_depth:           usize,
    pub in_fiber:             bool,
    pub globals_raw:          *mut Value,
    pub row_cache:            HashMap<usize, Vec<Value>>,
}
```

One `Executor` instance per running call stack (including fibers). Fibers get their own `Executor` created when the fiber is first resumed.

`stack` is the locals array for the current call frame. The executor uses a flat frame layout: `stack[0..max_locals]` are the locals for the innermost call; on a function call, the new frame is pushed at `stack_ptr`.

**Stack sizing:** `stack` is pre-allocated once in `Executor::new` at a size chosen by whether JIT is active: `64K` `Value` slots (1MB) when `vm.disable_jit` is set, favoring cache locality for the interpreter, or `512K` slots (8MB) when JIT is active, to accommodate the deeper native call chains that direct JIT-to-JIT calls can produce before falling back to the interpreter.

**`globals_raw`:** A raw pointer into the backing storage of `vm.globals`, captured once in `Executor::new` (`vm.globals.read().as_ptr() as *mut Value`) instead of re-acquiring the `RwLock` read guard on every global access. This is safe only because the globals vector is fixed-size for the lifetime of the `VM` — `Executor::new` asserts this invariant with `debug_assert_eq!(vm.globals.read().len(), 65536, ...)`, since any reallocation of the backing `Vec` would silently invalidate `globals_raw`.

**`row_cache`:** A map from a table's heap address (`Arc::as_ptr(&t_rc) as usize`) to a vector of previously-constructed `RowObj` values, one per row index, used by `table.where(...)` (see `compiler/runtime/runtime_collections.md`) to avoid reallocating a `RowObj` for every row on every filter invocation. `insert`, `delete`, `update`, and `clear` on a table invalidate its entry in `row_cache` so that stale row handles are never read after a structural mutation.

**JIT Warmup Threshold:** A chunk's call count must cross `vm.jit_threshold` (defaulting to 50, customizable via CLI option `--threshold`) before JIT compilation is attempted. The `xcx_jit_call_recursive` FFI fallback path uses its own fixed warmup of 5 calls, and `VM::run` additionally attempts an eager JIT compilation of the main chunk before any warmup counting.

### Main Dispatch Loop

The executor runs a `loop` over the bytecode array. On each iteration:

1. Check `SHUTDOWN`.
2. Load `chunk.bytecode[ip]`.
3. Dispatch to the appropriate step handler.

Step handlers are split into modules and called as free functions taking `(op, locals, &mut exec, vm_arc)`. This keeps the main dispatch function manageable and allows the compiler to inline hot paths.

### Function Call Frames (`handle_call` / `handle_call_no_jit`)

Both entry points funnel into a single private `handle_call_inner`, which checks the recursion limit, prepares the new frame (`prepare_frame`), resolves or triggers JIT compilation of the callee via `check_jit_warmup` (only from `handle_call`, which is reached from JIT-compiled code — `handle_call_no_jit` reads the callee's `jit_ptr` directly since it originates from the interpreter loop and does not need to trigger warmup), executes the frame, and tears it down (`cleanup_frame`). Consolidating frame preparation/cleanup into one function removes what was previously duplicated setup and teardown code between the two call paths. When `check_jit_warmup` triggers a fresh JIT compilation and it fails, the JIT is silently disabled for the rest of the run (`vm.disable_jit = true`); nothing is logged and `error_count` is not touched.

### `dispatch_jit_call`

```rust
unsafe fn dispatch_jit_call(
    &mut self,
    jit_ptr: *mut c_void,
    locals_start: usize,
    ...
) -> OpResult
```

Calls the JIT-compiled native function through an `extern "C"` ABI. Passes the executor pointer, the locals slice base, and the shared context. The JIT function may update `fiber_next_ip` and `fiber_yielded` via the executor pointer.

---

## Dispatch (`dispatch.rs`)

### `handle_method_call`

Routes `MethodCall` opcodes to the correct runtime handler based on the receiver's tag:

```
TAG_DB   → handle_database_method
TAG_TBL  → handle_table_method
TAG_ARR  → handle_array_method
TAG_BOOL_ARR → handle_bool_array_method
TAG_MAP  → handle_map_method
TAG_SET  → handle_set_method
TAG_STR  → handle_string_method
TAG_DATE → handle_date_method
TAG_JSON → handle_json_method
TAG_FIB  → handle_fiber_method
TAG_ROW  → handle_row_method
```

Before the tag dispatch, two special methods are handled globally for all types:
- `MethodKind::ToStr` — calls `receiver.to_string()` and wraps it in a `StringObj`.
- `MethodKind::ToJson` — calls `utils::json::value_to_json` and wraps it in a `JsonObj`.

Non-pointer values that receive any other method produce an error and return `Halt`.

### `handle_method_call_custom`

For methods identified by a string name rather than a `MethodKind` enum value. Currently used for:
- `TAG_ROW` — field access by name (`handle_row_custom`).
- `TAG_JSON` — dynamic key access (`handle_json_custom`).
- `TAG_DB` — database table handle access via `RuntimeOps::get_member`.

---

## JIT FFI Helpers (`jit_helpers.rs`)

These are `#[unsafe(no_mangle)] pub unsafe extern "C"` functions called directly by JIT-compiled native code. All values are passed as `(bits: u64, tag: u64)` pairs matching the `Value` layout; results are written to `*mut Value` output pointers.

### Arithmetic

| Symbol | Signature | Description |
|---|---|---|
| `xcx_jit_add` | `(out, a_bits, a_tag, b_bits, b_tag)` | `a + b` |
| `xcx_jit_sub` | same | `a - b` |
| `xcx_jit_mul` | same | `a * b` |
| `xcx_jit_div` | `+ exec_ptr` | `a / b`; calls `xcx_jit_abort_div` and sets error on divide-by-zero |
| `xcx_jit_mod` | `+ exec_ptr` | `a % b`; same error handling |
| `xcx_jit_neg` | `(out, a_bits, a_tag)` | `-a` |

### Comparison

`xcx_jit_eq`, `xcx_jit_ne`, `xcx_jit_gt`, `xcx_jit_lt`, `xcx_jit_ge`, `xcx_jit_le` — all write a `bool` Value to `out`.

### Cast

| Symbol | Description |
|---|---|
| `xcx_jit_cast_string` | Value → string |
| `xcx_jit_cast_int` | Value → int (via `cast_int()`) |
| `xcx_jit_cast_float` | Value → float |
| `xcx_jit_cast_bool` | Value → bool |

### Table / Row

| Symbol | Description |
|---|---|
| `xcx_jit_row_get(out, row_bits, row_tag, col_idx)` | Read column from row; bounds-checked |
| `xcx_jit_table_size(table_bits, table_tag) -> i64` | Row count |
| `xcx_jit_table_get_row(out, table_bits, table_tag, row_idx)` | Create `RowObj` for given index |
| `xcx_jit_table_push_row(table_bits, table_tag, row_bits, row_tag)` | Deep-copy row into table |
| `xcx_jit_table_clone_skeleton(out, src_bits, src_tag)` | Clone table schema without rows |

### JSON

| Symbol | Description |
|---|---|
| `xcx_jit_json_bind(out, json_bits, json_tag, path_bits, path_tag)` | Extract path from JSON |
| `xcx_jit_json_bind_const(out, json_bits, json_tag, path_ptr, path_len)` | Same with compile-time path bytes |
| `xcx_jit_json_parse(out, bits, tag)` | Parse JSON string; uses a thread-local cache of up to 32 entries (strings ≤16KB) keyed by string pointer or content, evicting the oldest. Cache hits return a `JsonVal::clone()` (Arc-sharing for nested nodes). |

### I/O

| Symbol | Description |
|---|---|
| `xcx_jit_print(bits, tag)` | Print value to stdout with ANSI erase-to-EOL sequence |
| `xcx_jit_wait(ms)` | Sleep for `ms` milliseconds; flushes buffered output first |

### Halt

| Symbol | Description |
|---|---|
| `xcx_jit_halt_alert(bits, tag)` | Emit ALERT message |
| `xcx_jit_halt_error(exec_ptr, bits, tag)` | Emit ERROR (does not increment the error counter) |
| `xcx_jit_halt_fatal(bits, tag)` | Print FATAL and call `process::exit(1)` |

### Fiber

| Symbol | Description |
|---|---|
| `xcx_jit_set_fiber_state(exec_ptr, next_ip, is_yield)` | Update executor's fiber position and yield flag |

### Misc

| Symbol | Description |
|---|---|
| `xcx_jit_typeof(out, bits, tag)` | Write type name as string value |
| `xcx_jit_get_member(out, obj_bits, obj_tag, name_ptr, name_len)` | Named member access |
| `xcx_jit_string_concat(out, a_bits, a_tag, b_bits, b_tag)` | String concatenation |
| `xcx_jit_string_length(bits, tag) -> i64` | String byte length |
| `xcx_jit_store_read(out, bits, tag)` | Read file from store |
| `xcx_jit_dec_ref_range(ptr, count)` | Decrement refcounts for `count` values starting at `ptr` |
| `xcx_jit_has_errors(exec_ptr) -> u32` | Returns 1 if error count > 0 |
| `xcx_jit_abort_div(exec_ptr)` | Log divide-by-zero; increment error count |
| `xcx_jit_report_guard_failure(exec_ptr, failing_ip)` | Called by JIT when a type guard fails (currently a no-op; handled at the bytecode layer) |
| `xcx_jit_date_now(out)` | Write current UTC timestamp as date value |

---

## Entry Point (`main.rs`)

The full compilation and execution pipeline:

1. Read source file.
2. `Parser::new(source)` → `parse_program()` → `Program`.
3. `Expander::new(interner)` → `expand(program, dir)` — resolves includes.
4. `Checker::new(interner)` → `check(program, symbols)` — type checking. Exit on errors.
5. `Compiler::new()` → `compile(program, interner)` → `(main_chunk, constants, functions)`.
6. `SharedContext { constants, functions, http_req: None }`.
7. Spawn `xcx-executor` thread with 64MB stack; call `vm.run(main_chunk, ctx, &[])`.
8. Join thread; flush stdout/stderr; exit with code 1 if `error_count > 0`.

The `--no-jit` flag sets `vm.disable_jit = true`, skipping JIT compilation entirely. The REPL path creates a `Repl` instance instead of running a file.

The `pax` subcommand looks for `lib/pax/src/pax.xcx` relative to the working directory, then relative to the executable directory (walking up parent directories) and runs it as a normal XCX file. A sibling `doc` subcommand resolves `lib/doc/doc.xcx` the same way.

`TerminalCleanup` is a zero-size RAII guard that calls `crossterm::terminal::disable_raw_mode()` on drop — ensuring terminal state is always restored even on panic.