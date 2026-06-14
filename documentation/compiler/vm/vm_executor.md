# VM — Executor, Trace Recording, and VM Struct

This document covers the execution engine: the `VM` struct, `SharedContext`, `Executor`, the main dispatch loop, trace recording infrastructure, and the JIT FFI helpers.

---

## Module Layout

```
src/vm/core/
├── vm.rs              — VM struct, SharedContext, OpResult
├── executor.rs        — Executor struct, dispatch_jit_call, jit_helpers
├── dispatch.rs        — handle_method_call / handle_method_call_custom
├── runtime_ops.rs     — RuntimeOps (get_member, table_init, etc.)
├── jit_helpers.rs     — #[no_mangle] extern "C" functions exposed to JIT
├── arena.rs           — Arena bump allocator
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
    pub globals:      parking_lot::RwLock<Vec<Value>>,
    pub error_count:  AtomicUsize,
    pub disable_jit:  bool,
    // ... internal fields
}
```

The `VM` is created once per program execution and wrapped in `Arc<VM>` so it can be shared between the main executor and any spawned fiber executors. It holds the global variable array and the global error counter.

`VM::new()` initializes with an empty globals vector and `disable_jit = false`.

### `get_global(idx) -> Value`

Used in tests and by the executor to read a global by index.

### `run(chunk, ctx, args)`

Main entry point. Creates an `Executor`, calls `executor.run_chunk(chunk, ctx, args)`. Runs on the dedicated `xcx-executor` thread (64MB stack) as spawned from `main`.

### `error_count`

`AtomicUsize`. Incremented by `HaltError`, division by zero, JIT error handlers, and method dispatch failures. After execution, `main` checks this and exits with code 1 if non-zero.

---

## `SharedContext`

```rust
pub struct SharedContext {
    pub constants: Vec<Value>,
    pub functions: Vec<Arc<Chunk>>,
    pub http_req:  Option<HttpRequest>,
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
    Halt,
    FiberYield,
    FiberDone,
}
```

Returned by every step handler. `Continue` proceeds to the next instruction. `Halt` stops execution. `FiberYield` / `FiberDone` are used during fiber execution to signal the parent executor.

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
    pub hotspot:              Hotspot,
    pub recorder:             Recorder,
    pub trace_cache:          Vec<Option<Arc<RwLock<Trace>>>>,
    pub terminal_raw_enabled: bool,
    pub fiber_next_ip:        usize,
    pub current_bytecode_ptr: usize,
    pub stack:                Vec<Value>,   // 64K Value slots pre-allocated
    pub stack_ptr:            usize,
    pub call_depth:           usize,
}
```

One `Executor` instance per running call stack (including fibers). Fibers get their own `Executor` created when the fiber is first resumed.

`stack` is the locals array for the current call frame. The executor uses a flat frame layout: `stack[0..max_locals]` are the locals for the innermost call; on a function call, the new frame is pushed at `stack_ptr`.

`JIT_WARMUP_THRESHOLD = 5` — a chunk must be called at least 5 times before JIT compilation is attempted.

### Main Dispatch Loop

The executor runs a `loop` over the bytecode array. On each iteration:

1. Check `SHUTDOWN`.
2. Load `chunk.bytecode[ip]`.
3. Optionally record the op for trace recording (if `recorder.is_recording`).
4. If `hotspot.tick(ip)` returns `true` (loop back-edge hit threshold), attempt trace compilation.
5. If a compiled trace exists in `trace_cache[ip]`, call the JIT-compiled function via `dispatch_jit_call`.
6. Otherwise dispatch to the appropriate step handler.

Step handlers are split into modules and called as free functions taking `(op, locals, &mut exec, vm_arc)`. This keeps the main dispatch function manageable and allows the compiler to inline hot paths.

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

---

## Trace Recording (`vm/trace/`)

### `Hotspot`

```rust
pub struct Hotspot {
    pub counts:         Vec<u32>,
    pub threshold:      u32,          // default: 50
    pub blacklist:      HashSet<usize>,
    pub guard_failures: HashMap<usize, u32>,
}
```

Tracks per-IP execution counts. `tick(ip)` increments the count and returns `true` when the count crosses `threshold`. IPs in `blacklist` are never triggered.

`on_guard_failure(ip)` increments the guard failure count for an IP. After 3 guard failures, the IP is blacklisted permanently. Guard failures occur when a JIT-compiled trace is invalidated by a type mismatch at a guard check.

### `Recorder`

```rust
pub struct Recorder {
    pub recording_trace: Option<RwLock<Trace>>,
    pub is_recording:    bool,
    pub start_ip:        Option<usize>,
}
```

When trace recording is active (`is_recording = true`), `record(op: TraceOp)` appends the typed trace operation to the current trace. `start(trace)` begins recording; `stop()` returns the completed trace.

### `Trace` and `TraceOp`

A `Trace` is a sequence of `TraceOp` values recorded during a hot loop execution. `TraceOp` is a type-specialized version of `OpCode` — for example, instead of a generic `Add`, the trace records either `AddInt` or `AddFloat` based on the observed operand types. This type information is what allows the JIT to emit specialized native code without runtime type checks.

`TraceOp::to_opcode()` maps a `TraceOp` back to the corresponding `OpCode` (used for fallback when JIT compilation fails or is invalidated).

### `recording_helper.rs`

`Executor::record_op(op, locals, ip)` is called on each instruction while recording is active. It inspects the current operand types (by reading `locals[src].tag`) and emits the appropriate typed `TraceOp`. For example:

```
OpCode::Add { dst, src1, src2 } →
    if both are TAG_INT → TraceOp::AddInt
    if either is TAG_FLOAT → TraceOp::AddFloat
    etc.
```

Loop back-edges emit a `TraceOp::LoopNext` or similar to mark the trace boundary.

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
| `xcx_jit_json_parse(out, bits, tag)` | Parse JSON string; uses a thread-local LRU cache of up to 32 entries keyed by string pointer. Flat JSON (no nested arrays/objects) uses shallow clone; nested uses deep clone. |

### I/O

| Symbol | Description |
|---|---|
| `xcx_jit_print(bits, tag)` | Print value to stdout with ANSI erase-to-EOL sequence |
| `xcx_jit_wait(ms)` | Sleep for `ms` milliseconds; flushes buffered output first |

### Halt

| Symbol | Description |
|---|---|
| `xcx_jit_halt_alert(bits, tag)` | Emit ALERT message |
| `xcx_jit_halt_error(exec_ptr, bits, tag)` | Emit ERROR; increment error count |
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

The `--no-jit` flag sets `vm.disable_jit = true`, skipping trace compilation entirely. The REPL path creates a `Repl` instance instead of running a file.

The `pax` subcommand looks for `lib/pax.xcx` relative to the executable directory (walks up parent directories) and runs it as a normal XCX file.

`TerminalCleanup` is a zero-size RAII guard that calls `crossterm::terminal::disable_raw_mode()` on drop — ensuring terminal state is always restored even on panic.