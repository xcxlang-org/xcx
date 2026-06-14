# VM — Opcodes and Chunk

The bytecode compiler produces a flat array of `OpCode` values packed into a `Chunk`. This document covers every opcode, the `Chunk` structure, `MethodKind`, and the loop-detection helper.

---

## Module Layout

```
src/vm/opcode/
├── mod.rs       — re-exports
├── opcode.rs    — OpCode enum, MethodKind enum, TypeTag, calculate_has_loops
├── chunk.rs     — Chunk struct
└── decode.rs    — Decoder stub (currently identity; reserved for future use)
```

---

## `Chunk`

```rust
pub struct Chunk {
    pub bytecode:     Arc<Vec<OpCode>>,
    pub spans:        Arc<Vec<crate::error::Span>>,
    pub is_fiber:     bool,
    pub max_locals:   usize,
    pub has_loops:    bool,
    pub call_count:   Arc<AtomicUsize>,
    pub jit_ptr:      Arc<AtomicPtr<c_void>>,
    pub jit_segments: Arc<RwLock<HashMap<usize, usize>>>,
    pub name:         String,
    pub arity:        usize,
    pub uses_heap:    Arc<AtomicBool>,
    pub used_locals:  Arc<Vec<u8>>,
}
```

Each function and the top-level program are compiled into a `Chunk`. Key fields:

- `bytecode` — the instruction array. Indexed directly by instruction pointer (IP).
- `spans` — one `Span` per bytecode instruction; same length as `bytecode`. Used for runtime error reporting.
- `is_fiber` — true for fiber (coroutine) chunks, which have different call/return semantics.
- `max_locals` — number of local register slots required. The executor allocates exactly this many slots per call frame.
- `has_loops` — computed by `calculate_has_loops`; used by the JIT to decide whether to record a trace.
- `call_count` — incremented on each call. When it crosses `JIT_WARMUP_THRESHOLD` (5), the JIT compiles the chunk.
- `jit_ptr` — atomic pointer to the JIT-compiled native function; `null` if not yet compiled.
- `jit_segments` — maps bytecode IP ranges to JIT-compiled segment offsets, for partial compilation.
- `used_locals` — bitmask of which local slots are actually referenced, produced by `jit::analysis::analyze_chunk_locals`. Used by the JIT to skip zeroing unused slots.

`Chunk` is `Clone`; all fields use `Arc` so clones share the same underlying data.

---

## `TypeTag`

Used in `OpCode::Input` to specify what type the runtime should parse from stdin:

```rust
pub enum TypeTag {
    Int, Float, String, Bool, Date, Array, BoolArray, Set, Map,
    Table, Function, Row, Json, Fiber, Database, Unknown
}
```

---

## `MethodKind`

A u8-indexed enum of all built-in methods callable via `MethodCall`. The full list:

| Value | Name | Applies to |
|---|---|---|
| 0 | `Push` | Array |
| 1 | `Pop` | Array |
| 2 | `Len` | String, Array, Set, Map |
| 3 | `Count` | Table, Array, Set |
| 4 | `Size` | Array, Set, Map, Table |
| 5 | `IsEmpty` | Array, Set, Map, Table |
| 6 | `Clear` | Array, Set, Map, Table |
| 7 | `Contains` | Array, Set, Map |
| 8 | `Get` | Array, Map, Table, Store |
| 9 | `Insert` | Set, Map, Table, Database |
| 10 | `Update` | Map, Table, Database |
| 11 | `Delete` | Map, Set, Table, Database |
| 12 | `Find` | Array, Table |
| 13 | `Join` | Table |
| 14 | `Show` | Table (print formatted grid) |
| 15 | `Sort` | Array |
| 16 | `Reverse` | Array |
| 17 | `Add` | Set |
| 18 | `Remove` | Array, Set |
| 19 | `Has` | Set, Map, String, Array |
| 20 | `Length` | String |
| 21 | `Upper` | String |
| 22 | `Lower` | String |
| 23 | `Trim` | String |
| 24 | `IndexOf` | String, Array |
| 25 | `LastIndexOf` | String |
| 26 | `Replace` | String |
| 27 | `Slice` | String, Array |
| 28 | `Split` | String |
| 29 | `StartsWith` | String |
| 30 | `EndsWith` | String |
| 31 | `ToInt` | String, Float, Bool, Date |
| 32 | `ToFloat` | String, Int, Bool |
| 33 | `Set` | Map (upsert) |
| 34 | `Keys` | Map, Json |
| 35 | `Values` | Map, Json |
| 36 | `Where` | Table, Array |
| 37 | `Year` | Date |
| 38 | `Month` | Date |
| 39 | `Day` | Date |
| 40 | `Hour` | Date |
| 41 | `Minute` | Date |
| 42 | `Second` | Date |
| 43 | `Format` | Date, String |
| 44 | `Exists` | Store |
| 45 | `Append` | Store, String |
| 46 | `Inject` | Json → Table |
| 47 | `ToStr` | Any (calls `to_string`) |
| 48 | `ToJson` | Any (converts to JSON) |
| 49 | `Next` | Fiber |
| 50 | `Run` | Fiber |
| 51 | `IsDone` | Fiber |
| 52 | `Close` | Database, Fiber |
| 53 | `Begin` | Database (transaction) |
| 54 | `Commit` | Database |
| 55 | `Rollback` | Database |
| 56 | `Query` | Database |
| 57 | `QueryRaw` | Database |
| 58 | `Sync` | Database, Store |
| 59 | `Drop` | Database, Table |
| 60 | `Fetch` | Database |
| 61 | `Save` | Store |
| 62 | `Truncate` | Database, Table |
| 63 | `Exec` | Database |
| 64 | `IsOpen` | Database |
| 65 | `First` | Table, Array |
| 66 | `Key` | Map |
| 67 | `Ready` | IO (terminal ready-check) |
| 68 | `Put` | Store, Map |
| 69 | `Stringify` | Json, Any |
| 70 | `Start` | Fiber |
| 71 | `Result` | Net response |
| 72 | `Table` | Database (create table handle) |
| 73 | `Execute` | Database |
| 74 | `Status` | Net response |
| 75 | `Ms` | Date (milliseconds) |

`MethodKind::from_u8` maps the u8 discriminant back to the enum. Unknown values return `None`.

---

## `OpCode` — Full Reference

All operand registers are `u8` indices into the locals array unless otherwise noted. `dst` is always written; `src`, `src1`, `src2` are read-only.

### Data Movement

| Opcode | Fields | Description |
|---|---|---|
| `Move` | `dst, src` | Copy local `src` to `dst`. Handles refcounting. |
| `LoadConst` | `dst, idx: u32` | Load constant at `ctx.constants[idx]` into `dst`. |
| `GetVar` | `dst, idx: u32` | Load global variable at `globals[idx]` into `dst`. |
| `SetVar` | `idx: u32, src` | Store `src` into `globals[idx]`. |
| `SetName` | `src, name_idx: u32` | Associates a debug name with a value (no runtime effect on execution). |
| `EnvGet` | `dst, src` | Read the environment variable named `src` into `dst`. |
| `EnvArgs` | `dst` | Load command-line arguments array into `dst`. |

### Arithmetic

All arithmetic writes result to `dst` and reads operands from `src1`, `src2`. Type dispatch is performed at runtime via `Value::add`, `.sub`, etc.

| Opcode | Operation |
|---|---|
| `Add` | `src1 + src2` (int, float, string concat, date+int) |
| `Sub` | `src1 - src2` (int, float, date-int, date-date→days) |
| `Mul` | `src1 * src2` |
| `Div` | `src1 / src2` (halts on division by zero) |
| `Mod` | `src1 % src2` (halts on modulo by zero) |
| `Pow` | `src1 ^ src2` |
| `Neg` | `dst = -src` |
| `PowInt` | Integer-specialized power |
| `PowFloat` | Float-specialized power |
| `IntConcat` | `src1 ++ src2` — integer concatenation (joins digits: 12 ++ 34 = 1234) |

### Comparison

All write `bool` to `dst`.

| Opcode | Operation |
|---|---|
| `Equal` | `src1 == src2` |
| `NotEqual` | `src1 != src2` |
| `Greater` | `src1 > src2` |
| `Less` | `src1 < src2` |
| `GreaterEqual` | `src1 >= src2` |
| `LessEqual` | `src1 <= src2` |
| `Has` | `src2 HAS src1` — membership test |

### Logic

| Opcode | Fields | Description |
|---|---|---|
| `And` | `dst, src1, src2` | Boolean AND |
| `Or` | `dst, src1, src2` | Boolean OR |
| `Not` | `dst, src` | Boolean NOT |

### Control Flow

| Opcode | Fields | Description |
|---|---|---|
| `Jump` | `target: u32` | Unconditional jump to instruction index `target`. |
| `JumpIfFalse` | `src, target: u32` | Jump if `src` is falsy. |
| `JumpIfTrue` | `src, target: u32` | Jump if `src` is truthy. |
| `Halt` | — | Immediately stop execution. |
| `Return` | `src` | Return value in `src` from current function. |
| `ReturnVoid` | — | Return with no value. |

### Loop Opcodes

Specialized combined increment-and-jump instructions for loop optimization:

| Opcode | Fields | Description |
|---|---|---|
| `LoopNext` | `target` | Jump back to loop header (forward-iteration). |
| `LoopPrev` | `target` | Jump back to loop header (reverse-iteration). |
| `IncLocal` | `reg` | Increment local `reg` by 1. |
| `IncVar` | `idx` | Increment global `idx` by 1. |
| `IncLocalLoopNext` | `reg, target` | Increment local and jump if still in range. |
| `DecLocalLoopPrev` | `reg, target` | Decrement local and jump if still in range. |
| `IncVarLoopNext` | `g_idx, target, limit_reg, step_reg` | Global variable loop increment and jump. |
| `DecVarLoopPrev` | `g_idx, target, limit_reg, step_reg` | Global variable loop decrement and jump. |
| `ArrayLoopNext` | `arr_reg, idx_reg, val_reg, target, exit_ip` | Array iteration loop step. |
| `SetLoopNext` | `set_reg, idx_reg, val_reg, target, exit_ip` | Set iteration loop step. |

### Function Calls

| Opcode | Fields | Description |
|---|---|---|
| `Call` | `dst, func_idx: u32, base: u8, arg_count: u8` | Call function `func_idx`. Arguments are locals `[base..base+arg_count]`. Result stored in `dst`. |
| `MethodCall` | `dst, kind: MethodKind, base, arg_count` | Call a built-in method on the value at `locals[base]`. |
| `MethodCallNamed` | `dst, kind: MethodKind, base, arg_count, names_idx: u32` | Dynamic method dispatch using a string name from the constant pool. Used for row field access and JSON path access. |
| `CallClosure` | `dst, base, arg_count` | Call the closure in `locals[base]`. |
| `MakeClosure` | `dst, func_idx: u16, capture_count: u16, capture_start: u8` | Create a closure from function `func_idx` and capture `capture_count` locals starting at `capture_start`. |

### I/O

| Opcode | Fields | Description |
|---|---|---|
| `Print` | `src` | Print `locals[src]` to stdout with newline (`\x1b[K\r\n` on Unix). |
| `Input` | `dst, ty: TypeTag` | Read from stdin, parse as `ty`, store in `dst`. |
| `InputKey` | `dst` | Read a single keypress (non-blocking). |
| `InputKeyWait` | `dst` | Read a single keypress (blocking). |
| `InputReady` | `dst` | Check whether input is available; write bool to `dst`. |

### Terminal Control

| Opcode | Fields | Description |
|---|---|---|
| `TerminalRaw` | `dst` | Enable raw terminal mode. |
| `TerminalNormal` | `dst` | Disable raw terminal mode. |
| `TerminalClear` | `dst` | Clear the terminal screen. |
| `TerminalCursor` | `dst, on: bool` | Show or hide the cursor. |
| `TerminalMove` | `dst, x_src, y_src` | Move cursor to `(x, y)`. |
| `TerminalWrite` | `dst, src` | Write `src` to terminal without newline. |
| `TerminalRun` | `dst, cmd_src` | Execute a shell command; result to `dst`. |
| `TerminalExit` | `dst` | Exit the process. |

### Halt

| Opcode | Fields | Description |
|---|---|---|
| `HaltAlert` | `src` | Emit alert message (non-fatal; execution continues). |
| `HaltError` | `src` | Print error and halt the current execution. Increments `error_count`. |
| `HaltFatal` | `src` | Print fatal error and call `process::exit(1)`. |

### Collections — Initialization

| Opcode | Fields | Description |
|---|---|---|
| `ArrayInit` | `dst, base, count: u32` | Create an array from `count` values starting at `locals[base]`. |
| `SetInit` | `dst, base, count: u32` | Create a set from `count` values. |
| `MapInit` | `dst, base, count: u32` | Create a map from alternating key/value pairs; `count` is the number of pairs. |
| `TableInit` | `dst, skeleton_idx, base, row_count, col_count` | Initialize a table from a skeleton constant and row data in locals. |

### Collections — Operations

| Opcode | Fields | Description |
|---|---|---|
| `GetIndex` | `dst, container, index` | `dst = container[index]` — works on arrays, sets (by position), maps, strings. |
| `SetIndex` | `container, index, src` | `container[index] = src` — mutable index assignment. |
| `GetMember` | `dst, container, name_idx: u32` | Access a named member; `name_idx` is a constant pool index. |
| `SetMember` | `container, name_idx: u32, src` | Assign to a named member. |
| `SetUnion` | `dst, src1, src2` | Mathematical set union. |
| `SetIntersection` | `dst, src1, src2` | Mathematical set intersection. |
| `SetDifference` | `dst, src1, src2` | Set difference `src1 \ src2`. |
| `SetSymDifference` | `dst, src1, src2` | Symmetric difference. |
| `SetRange` | `dst, start, end, step, has_step` | Build a set from a numeric range. |

### Array Specifics

| Opcode | Fields | Description |
|---|---|---|
| `ArraySize` | `dst, src` | Write array length to `dst`. |
| `ArrayGet` | `dst, arr_reg, idx_reg, fail_ip` | Bounds-checked array index; jumps to `fail_ip` on out-of-bounds. |
| `ArrayGetIndex` | `dst, arr_reg, idx_reg, fail_ip` | Alternative bounds-checked get. |
| `ArrayPush` | `arr_reg, val_reg` | Append `val_reg` to array `arr_reg`. |
| `ArrayUpdate` | `arr_reg, idx_reg, val_reg, fail_ip` | Bounds-checked element assignment. |
| `ArraySetIndex` | `arr_reg, idx_reg, val_reg, fail_ip` | Same as `ArrayUpdate`. |

### Table Specifics

| Opcode | Fields | Description |
|---|---|---|
| `RowGet` | `dst, row_reg, col_idx: u16` | Read column `col_idx` from row `row_reg`. |
| `TableIter` | `tbl_reg, idx_reg, row_reg, limit_reg, target: u32` | Loop step for table iteration. Advances `idx_reg`, loads the row, jumps back to `target`. |
| `TablePushRow` | `tbl_reg, row_reg` | Deep-copy row `row_reg` into table `tbl_reg`. |
| `TableCloneSkeleton` | `dst, src` | Clone the schema of table `src` into a new empty table at `dst`. |
| `TableSize` | `dst, src` | Write row count to `dst`. |

### Fiber

| Opcode | Fields | Description |
|---|---|---|
| `FiberIsDone` | `dst, src` | Write `true` if fiber `src` is exhausted. |
| `FiberNext` | `dst, src` | Resume fiber `src` and store yielded value in `dst`. |
| `Yield` | `src` | Yield the value `src` from the current fiber. |
| `YieldWithTarget` | `dst, src` | Yield the value `src` from the current fiber, storing caller's injected response in `dst`. |

### Set Specifics

| Opcode | Fields | Description |
|---|---|---|
| `SetSize` | `dst, src` | Write set cardinality to `dst`. |
| `SetContains` | `dst, set_reg, val_reg` | Write `true` if `val_reg ∈ set_reg`. |

### Random

| Opcode | Fields | Description |
|---|---|---|
| `RandomInt` | `dst, min, max, step, has_step` | Generate a random integer in `[min, max]` with optional step. |
| `RandomFloat` | `dst, min, max, step, has_step, step_is_float` | Same for floats. |
| `RandomChoice` | `dst, src` | Pick a random element from the set `src`. |

### JSON / Date

| Opcode | Fields | Description |
|---|---|---|
| `JsonParse` | `dst, src` | Parse `src` as a JSON string; store `JsonObj` in `dst`. |
| `JsonBindLocal` | `dst, json_reg, path_reg` | Extract path from JSON; store in local `dst`. |
| `JsonBindLocalConst` | `dst, json_reg, path: String` | Same with a compile-time-constant path. |
| `JsonBindGlobal` | `idx: u32, json_reg, path_reg` | Extract and store in global `idx`. |
| `JsonBindGlobalConst` | `idx: u32, json_reg, path: String` | Same with compile-time path. |
| `JsonFastGetPush` | `json_src, path_src, val_src` | Fast JSON get and push; used in injection pipelines. |
| `DateNow` | `dst` | Write current UTC timestamp (millis) as a date value into `dst`. |

### Store (File System)

All store opcodes follow the pattern `dst, base` where `base` is the register containing the path expression.

| Opcode | Operation |
|---|---|
| `StoreWrite` | Write value to file |
| `StoreRead` | Read file contents |
| `StoreAppend` | Append to file |
| `StoreExists` | Check existence |
| `StoreDelete` | Delete file or directory |
| `StoreList` | List directory contents |
| `StoreIsDir` | Check if path is a directory |
| `StoreSize` | Get file size in bytes |
| `StoreMkdir` | Create directory |
| `StoreGlob` | Glob pattern match |
| `StoreZip` | Create zip archive |
| `StoreUnzip` | Extract zip archive |

### Crypto

| Opcode | Fields | Description |
|---|---|---|
| `CryptoHash` | `dst, pass_src, alg_src` | Hash a password using the specified algorithm. |
| `CryptoVerify` | `dst, pass_src, hash_src, alg_src` | Verify password against hash. |
| `CryptoToken` | `dst, len_src` | Generate a random token of specified length. |

### Networking

| Opcode | Fields | Description |
|---|---|---|
| `HttpCall` | `dst, method_idx: u32, url_src, body_src` | Execute an HTTP request; result JSON in `dst`. |
| `HttpServe` | `func_idx, port_src, host_src, workers_src, routes_src` | Start HTTP server; does not return. |
| `HttpRespond` | `dst, status_src, body_src, headers_src` | Send HTTP response from a route handler. |
| `HttpRequest` | `dst, arg_src` | Access the current HTTP request object. |

### Misc

| Opcode | Fields | Description |
|---|---|---|
| `Typeof` | `dst, src` | Write the type name string of `src` into `dst`. |
| `StringLength` | `dst, src` | Write string byte length into `dst`. |
| `CastFloatToInt` | `dst, src` | Truncating float-to-integer cast. |
| `CastBool` | `dst, src` | Convert to boolean. |
| `GetMember` | `dst, container, name_idx` | Named member access. |

---

## Loop Detection

```rust
pub fn calculate_has_loops(bytecode: &[OpCode]) -> bool
```

Returns `true` if any backward jump exists in the bytecode (i.e. a `Jump`, `JumpIfFalse`, `JumpIfTrue`, or any loop-specialized opcode where `target < current_ip`). Called automatically during `Chunk::new`; the result is stored in `Chunk::has_loops` and used by the JIT to decide whether trace recording is worthwhile.