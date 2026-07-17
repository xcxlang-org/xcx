# XCX 4.2 - Changelog (4.1 -> 4.2)
**Type: RELEASE**

---

## New compilation layer: HIR

- **[NEW]** Introduced an intermediate representation, HIR. Function ASTs are now lowered to HIR first, and only then compiled to bytecode.
- **[NEW]** Function inlining at the HIR level. The inlining policy excludes fibers, recursive functions, `return` inside a loop, deeply nested code, and functions that are too costly.
- **[NEW]** Full compilation of `TableLiteral`, `DatabaseLiteral`, `DateLiteral`, and `Tuple` literals in HIR, removing the default Int initialization that used to cause method dispatch errors.
- **[CHG]** `loop_stack` rebuilt as a typed `LoopFrame` structure, now consistent between the AST and HIR compilers.

---

## JIT: new optimizations

- **[NEW]** Fast path bounds check for arrays (`int[]`, `bool[]`): the `index < len` check is now done directly in the JIT code, with a runtime fallback for out-of-range access.
- **[NEW]** Dedicated fast path for `BoolArray`: reads and writes to the bit buffer are now inlined directly, bypassing FFI and `RwLock`.
- **[FIX]** Type inference failed to recognize `array:b` constants as `BoolArray` (they ended up as `Unknown`), which blocked the fast paths above and forced a slow dynamic dispatch.
- **[NEW]** Constant tracking in registers (`register_const`), unlocking further arithmetic optimizations; also fixed `clear_block_state`, which previously failed to clear this state.
- **[NEW]** Division/modulo by a constant power of 2 is now emitted without divide-by-zero/overflow guards.
- **[NEW]** Typed `JumpIfFalse`: for a statically known `Bool`, the comparison now reduces to a simple `icmp` instead of a full tag+bits comparison.
- **[CHG]** Global variable reload after a function call is now conditional (only when the callee actually uses globals), instead of always happening.

---

## JSON: caching and fast access paths

- **[NEW]** Cache for parsed `json.parse()` strings (thread-local, up to 128 entries).
- **[NEW]** Fast path for simple keys (no `.`, `[`, `]`) in getters, `has()`, and `keys()`/`len()`, skipping the generic `json_pointer`.
- **[NEW]** Fast path for integer indexing in `get()` on a JSON array, avoiding the int -> string -> parse conversion.
- **[NEW]** Direct key scanning in `JsonBindLocal` and in `get_path_value_xcx`, bypassing `normalize_json_path`/`pointer()`.
- **[CHG]** Pre-allocation of column keys in `TableObj::to_json`: down from 3000 `Arc` allocations to 3, for 1000 rows x 3 columns.
- **[FIX]** The thread-unsafe `dirty: AtomicBool` flag was replaced with a `version`/`cached_version` counter pair, eliminating the risk of reading a stale or corrupted JSON string when accessed from the JIT and VM at the same time.

---

## String concatenation: new `StrAppend*` opcodes

- **[NEW]** `StrAppendVar` / `StrAppendLocal` / `StrAppendMember` / `StrAppendElement`: the `var = var + expr` pattern (global, local, JSON field, array element) now mutates the string buffer in place when the `Arc` ownership is unique, instead of doing three allocations per iteration. A safe fallback (COW) kicks in when ownership isn't unique.
- **[NEW]** Compile-time flattening of concatenation chains (`res = res + "a" + "b"`) into a sequence of `StrAppend*` calls, in both AST and HIR.
- Results (100k iterations): global variable 1300 ms -> 2.2-3.5 ms; array element 86 ms -> ~3-4 ms; general append under JIT 10790 ms -> 5.5 ms.

---

## VM / Executor: performance

- **[CHG]** Dynamic stack size: 1 MB under `--no-jit`, 8 MB when JIT is active (for safety with deep recursion).
- **[CHG]** The raw `globals_raw` pointer is now set once at executor initialization, instead of being read through a lock on every access.
- **[INT]** `handle_call` and `handle_call_no_jit` merged into `handle_call_inner`.

---

## Table queries

- **[NEW]** `table.join`: Hash-Join O(N+M) for key-based joins. Benchmark at 500x500 rows: 215 ms -> 10 ms.
- **[NEW]** `table.where`: a cache on `RowObj` (`row_cache`) avoids hundreds of thousands of `Arc` allocations, automatically invalidated on `insert`/`delete`/`update`/`clear`.
- **[NEW]** `table.count()`/`.len()`/`.size()` with an active `sql_where` are now computed directly in the database (`SELECT COUNT(*)`), instead of in memory.

---

## Networking: HTTP

- **[NEW]** TCP/TLS connection pooling: a global agent (`HTTP_AGENT`) is used instead of a new TLS handshake on every request. 100 sequential HTTPS requests: 195 ms/request -> 63 ms/request.

---

## CLI

- **[NEW]** `--help` output reorganized into sections (`Usage`, `Options`, `Execution`, `Dev tools`).
- **[NEW]** Short flags `-h`/`-v`; they now work anywhere in the argument list.
- **[NEW]** Combining options with the `|` operator (e.g. `--no-jit | --bytecode`).

---

## Fixed

- **[FIX]** `@step` loops with a bare backward jump and no `LoopNext` could cause a hang or invalid native code; the start address is now explicitly added to the hotspot blacklist.
- **[FIX]** `JsonPush` on a JSON object (instead of an array) used to panic; it now returns `0` and increments the error counter.
- **[FIX]** `table.where(...)` did not propagate extra arguments as captures to the filter function.
- **[FIX]** An incorrect bitcast in mixed `Float + Int` arithmetic (and its variants) caused checksum discrepancies; the JIT now requires both operands to be statically float, and the VM uses `.cast_float()` instead of a raw bit read.
- **[FIX]** A missing SSA block switch in `GetIndex` for `BoolArray` produced incomplete Cranelift code.
- **[FIX]** A register allocation bug for closures/captures in `table.where(...)` caused dispatch errors in both the JIT and the interpreter.

---

## Performance

**Methodology note:** with XCX 4.2, a new, improved benchmark suite has been introduced (including more stable `lcg`/`sieve` measurements and more accurate `json` sampling). For continuity, this release reports results from both the old and the new suite; from the next changelog onward, comparisons will use the new suite exclusively.

### Old suite (for continuity with previous changelogs)

Measured on Ryzen 7 5800X / 32 GB RAM / Windows 11.

| Benchmark | XCX 4.1 (no JIT) | XCX 4.1 (JIT) | XCX 4.2 (no JIT) | XCX 4.2 (JIT) |
|---|---|---|---|---|
| Loop 100M | 7340 ms | 116.27 ms | 7481.67 ms | 86.36 ms |
| Fib(30) | 180 ms | 12.87 ms | 183.10 ms | 12.41 ms |
| Sieve 100K | 22 ms | 2.29 ms | 21.49 ms | 2.23 ms |
| JSON parse | 64 ms | 21.46 ms | 54.61 ms | 20.70 ms |

### New suite (will be the baseline for comparisons from the next version onward)

| Variant | fib(30) (ms) | lcg(100m) (ms) | sieve (ms) | json (ms) |
|---|---|---|---|---|
| XCX 4.1, JIT | 13.119 | 107.706 | 201.003 | 0.272 |
| XCX 4.2, JIT | 12.303 | 106.480 | 97.814 | 0.238 |
| XCX 4.1, `--no-jit` | 183.094 | 4207.295 | 1938.896 | 0.280 |
| XCX 4.2, `--no-jit` | 186.819 | 4328.303 | 1876.484 | 0.267 |