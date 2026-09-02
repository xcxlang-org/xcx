# XCX 4.3 - Changelog (4.2 -> 4.3)
**Type: RELEASE**

---

## JSON: nesting-depth limit lifted (json_recursion_limit)

- **[FIX]** `json.parse()` rejected valid JSON nested deeper than 128 levels with `halt.fatal: Invalid JSON (R305)` — the default recursion limit of `serde_json::from_str`. All user-JSON decoding (strict and relaxed paths in `handle_json_parse`, plus the compile-time `is_parseable_json` predicate) now goes through a depth-unlimited deserializer (`from_str_unbounded` in `src/runtime/builtin/json/parse.rs`; `serde_json` feature `unbounded_depth`). The practical depth ceiling is the 64MB executor stack. A 500-deep build-serialize-reparse round trip now completes in both JIT and `--no-jit` modes.
- **[CHG]** Removed two `unused mut` warnings in the `nan_ops.rs` layout tests; the test build is warning-free again.

---

## JSON: raw literals `<<< ... >>>` produce a json value in every expression position

- **[FIX]** A `<<< ... >>>` literal was only a json value in declaration position (`json: x <<< ... >>>;` and `json: x = <<< ... >>>;`, which the parser desugars to `json.parse(...)`). In every other expression position — reassignment to an existing variable (`x = <<< ... >>>;`) and call arguments (`push(<<< ... >>>)`) — it silently degraded to a plain String, and subsequent JSON methods (`.set()`, `.bind()`, ...) failed at runtime with the misleading message `Method Set not found on String`.
- **[CHG]** The AST compiler (`src/compiler/compile_expr/leaf.rs`) and the HIR compiler (`src/hir/compile_expr.rs`) now emit a `JsonParse` opcode after loading the literal string when the literal's content is valid JSON (strict or relaxed — the same acceptance as the runtime parser, exposed as `is_parseable_json` in `src/runtime/builtin/json/parse.rs`). Literals whose content is not valid JSON (for example containing embedded expressions, which the language does not support) keep the previous raw-string behavior, so programs relying on it are unaffected.
- **[FIX]** JIT/interpreter divergence in `xcx_jit_json_parse` (`src/vm/core/jit_helpers.rs`): called with an already-parsed json value it returned `false`. It now returns the value itself (identity, refcount-incremented), matching the interpreter path, which reaches the same result via `to_string` + re-parse. The divergence surfaced when desugared `json.parse(<literal>)` calls started receiving json-typed arguments.
- All five reproduction cases now produce identical output in JIT and `--no-jit` modes. Declaration semantics for invalid literal content are unchanged (still an R305 halt at runtime). `cargo test --release`: 200 passed / 0 failed / 0 ignored.

---

## VM: JIT stability and fallback to the interpreter

- **[CHG]** The `disable_jit` field of the `VM` struct changed from `bool` to `std::sync::atomic::AtomicBool`, allowing safe modification at runtime from JIT context (without taking a mutex).
- **[NEW]** Dynamic JIT disable on first compilation error. If `JIT::compile_method` or `JIT::compile` returns an error (e.g. `finalize_definitions` reports `EACCES` on systems with a W^X policy), the VM permanently disables the JIT for the whole session (`disable_jit.store(true)`) and continues in interpreter mode. Previous behavior: the VM logged the error and continued in an undefined state.
- **[FIX]** Removed the `error_count` increment on JIT compilation failure in `executor.rs` and `jit_helpers.rs`. A compilation error (which results in a correct fallback) was previously counted as an execution error, which could incorrectly affect the process exit code.

Affects: FreeBSD 14+ (W^X kernel), and any system where the JIT cannot finalize executable-memory metadata.

---

## VM: SSRF — unified interpreter and JIT behavior

- **[FIX]** The `call()` function in `src/runtime/builtin/net/client.rs` (the interpreter path for `net.get`) now calls `is_safe_url(&url)` instead of a manual `url.contains("169.254.")` check. The previous implementation blocked only link-local addresses (`169.254.*`) instead of the full set of protected address ranges.
- **[FIX]** When `is_safe_url()` returns an error prefixed with `HALT.FATAL` or `HALT.ERROR`, the interpreter now raises a `panic!()` with the appropriate message. Previous behavior: returning `OpResult::Continue` with an error map — the VM kept running instead of terminating the process.
- **[CHG]** SSRF behavior is now consistent between the JIT path (`xcx_jit_net_call`) and the interpreter path (`call`): both terminate the process when a disallowed address is detected.

---

## SSRF: private-range check narrowed to RFC 1918

- **[FIX]** `is_safe_url` blocked the entire `172.*` range; only `172.16.0.0/12` is private (RFC 1918). Public `172.0–15.x` and `172.32–255.x` addresses are no longer falsely rejected with `halt.error` — the check now tests the second octet (16–31). Covers interpreter and JIT (shared function).
- **[CHG]** Removed the unreferenced duplicate of `is_safe_url` in `src/vm/utils/network.rs`; the live interpreter/JIT paths use `src/runtime/builtin/net/client.rs`. Three unit tests added there.

---

## JIT: register-initialization tracking and multi-return-path support

- **[FIX] `defined_locals` tracking in `CodegenCtx`:** added a `defined_locals: [bool; 256]` array. Local registers are marked initialized (`true`) only when loaded in `preload_locals` (if they belong to the computed `needs_init` set) and on direct definition in `def_local`, `def_local_nanboxed`, and `reload_local`.
- **[FIX] `cleanup_all` and `should_skip_dec_ref` filtering:**
  - `cleanup_all` skips registers whose `defined_locals[r]` is `false`. This prevents accidentally loading and defining Cranelift variables in the entry block (Block 0) for uninitialized registers while processing early `return` paths.
  - `should_skip_dec_ref` returns `true` for registers with `defined_locals[r] == false`, preventing generation of `dec_ref` instructions for stale/random stack values on first assignment to a register.
- **[FIX] Register synchronization after FFI in `reload_local`:** after calling the `StrAppendLocal` helper, `reload_local` now sets the `mark_used`, `mark_dirty`, `defined_locals[r] = true` flags and `known_types[r] = TypeTag::String`. Previously the missing `dirty` marking meant subsequent `spill_all` operations did not write the updated string pointer and tag back to VM memory (`locals_ptr`).
- **[FIX] Constant type annotations in `emit_load_const`:** pointers to heap-allocated constant objects (including strings) are now recorded as `ctx.known_types[dst] = TypeTag::String` in `emit_load_const`. Previously `known_types` remained `TypeTag::Unknown`.

---

## I/O & Terminal: dedicated `.xcx` file execution (`.terminal !run`)

- **[FIX] Invoking the interpreter for `.xcx` files:** added the helper `execute_run(cmd: &str)` in `src/runtime/builtin/io/terminal.rs`. When the first parameter of the command points to a `.xcx` file, `.terminal !run` launches the current compiler executable directly (`std::env::current_exe()`) passing the file and arguments, instead of relying on OS file associations (which on Windows triggered the "Open with" dialog).
- **[FIX] Standard output and exit status handling:**
  - `OpCode::TerminalRun` and `xcx_jit_terminal_run` (JIT path) print the captured `stdout` buffer of the child process directly to the terminal (`write_buffered` + `flush_buffered`).
  - The functions return the output string `Value::from_string(stdout)` on success (or `Value::from_bool(true)` when the output is empty) and `Value::from_bool(false)` on process failure, enabling correct conditional evaluation (e.g. `if (NOT .terminal !run target)` in the `PAX` package manager).

---

## Crypto: random token length representation

- **[FIX]** `crypto.token(len)` now generates `len * 2` hexadecimal characters instead of `len` characters. The `len` parameter represents the token length in bytes, so the hexadecimal string representation is now correctly `2 * len` characters long.

---

## JIT: parameter type-inference precision

- **[FIX]** Introduced bidirectional type-constraint propagation in `infer_param_types`. Previously the inference mechanism assigned a static `TypeTag::Int` to parameters based solely on the occurrence of arithmetic operations (`Add`, `Sub`, `Mul`, etc.) or comparisons (`Less`, `Greater`, etc.). This caused errors (e.g. in `nested_func_expr_arg`) where floating-point values passed to nested functions were incorrectly treated as integers, leading to register corruption / returning `0` in the JIT.
- **[REF]** The new mechanism propagates types backwards and forwards based on constant types (`LoadConst`), casts (`Cast*`), and type-specific instructions (e.g. loop operations and increments). This safeguards floating-point typing correctness while fully preserving JIT optimizations for integer operations (e.g. in the `fib(30)` benchmark).

---

## Store: recursive directory deletion

- **[FIX]** Modified `store.delete()` in the interpreter path (`read_write.rs`) and the JIT helper `xcx_jit_store_delete` (`jit_helpers.rs`). Both previously called `std::fs::remove_file` unconditionally, which failed and returned `false` when deleting a directory. The path is now checked — directories are removed recursively with `std::fs::remove_dir_all`, files with `std::fs::remove_file`.

---

## Terminal: preventing spurious error-count growth (fix `terminal_error_count`)

- **[FIX]** Modified the AST (`call.rs`) and HIR (`compile_expr_special.rs`) compilers to handle the `terminal` namespace. Methods such as `.write()`, `.clear()`, `.raw()`, `.normal()`, `.cursor()`, `.move()`, `.exit()`, `.run()` are now correctly detected and compiled directly to their dedicated VM opcodes, instead of going through the generic `xcx_jit_method_call_custom` JIT/VM helper with a `TAG_STR` ("terminal") receiver, which previously bumped the internal `error_count` counter on every call.

---

## Input: filtering key-release events in raw mode (fix `double_input_event`)

- **[FIX]** Modified input handling in `input()`, `read_key()`, and `wait_key()` in `input.rs` to filter out `KeyEventKind::Release` events (key releases) when reading keyboard events via `crossterm`. Previously, the missing filter caused a single physical keypress on Windows to generate two events with the same key code (press and release), double-triggering event registration in the VM.

---

## JIT: receiver refcount elision for the `GetVar` -> `MethodCall` pattern

- **[FIX]** Fixed a reference leak on method calls over global collections. The compiler emits every such call as `GetVar G -> r` + `MethodCall { dst: r, base: r }`; `emit_get_var` incremented the pointer's refcount, but the specialized fast paths (`Update`/`Set`/`Push`/`Get`) overwrote the receiver register with the result without a matching `dec_ref` — every `x.push(...)` / `x.update(...)` / `x.get(...)` on a global collection leaked one reference (in long-running loops: unbounded `strong_count` growth).
- **[NEW]** Static `getvar_inc_elidable` analysis (`src/jit/analysis.rs`) detects a `GetVar` immediately followed by its consuming `MethodCall` (through a window of argument-setup ops that neither spill registers nor run user code). The `unowned_recv_regs` set in `CodegenCtx` marks the register as an un-owned borrow; `def_local` clears the bit on every redefinition, so a borrow never outlives its consuming instruction. The `[GetVar without inc]` + `[MethodCall without dec]` pairs are exactly refcount-neutral.
- **[REF]** Performance effect (Main Suite, 20 warmup / 100 runs): sieve 99.19 -> 34.63 ms in development measurements (baseline.json: 97.814 ms); decomposition: marking loop 67–85 ms -> ~28 ms, counting loop ~30 ms -> ~17 ms. Other metrics within noise: fib(30) 11.30 -> 11.16 ms, lcg(100M) 109.33 -> 108.52 ms, json 0.126 -> 0.120 ms. Removing the `inc_ref` FFI call from sieve's inner loop (~23M executions in marking, ~10M in counting) was the dominant cost of that benchmark. `cargo test --release`: 199 passed (including two new layout tests in `nan_ops.rs`).

---

## JIT/VM: removal of the unreachable trace JIT and fiber-segment JIT subsystems

- **[CHG]** Removed the trace-recording subsystem (`vm/trace/`), the `JIT::compile` trace compiler, and the fiber-segment JIT (`src/jit/compiler_fiber.rs`, the `jit_segments` map), along with all `Executor`/`VM` trace plumbing, REPL trace rows, and 16 orphaned emitters. Both subsystems were dead (unreachable from any execution path) — per-function method JIT remains the only JIT mode. No functional behavior change; the engine's dead overhead is gone.

---

## VM: verified dead-code removal

- **[CHG]** Removed 13 dead files (~1,480 lines) along with the closure-vestige chain: `ClosureObj`, `UpvalueCell`, `TAG_CLOSURE`, the `MakeClosure` opcode, the `is_closure`/`is_arena` flags, and `collect_backedges`.
- **[CHG]** Removed 36 scattered dead functions (including the `xcx_eq`..`xcx_ge` family, `use_*_nanboxed`, `emit_div_int`).
- **[CHG]** Removed the `TAG_ARENA` value format across 9 files (the nan_ops predicate test was updated).
- **[CHG]** Removed the `Chunk::has_loops` field and `calculate_has_loops` — after the trace recorder's removal the field had zero readers (10 construction sites updated).
- **[REF]** Net ~2,900 lines of verified dead code removed. `--release` build with zero warnings; 200 tests passed / 0 failed / 0 ignored; full benchmark gate at parity or better vs `baseline.json` (sieve −1.7%, FUNC geometric mean −7.9%, TOTAL −1.6% on that interim gate). The only above-baseline entry is the known-open `for step` regression (+35%).

---

## Diagnostics and tests: technical-debt cleanup

- **[FIX]** `DEBUG:` prints replaced with span-aware messages: "Method call on non-object receiver" and "Unknown receiver tag" in `dispatch.rs`, and the JSON fallback path — all now append `current_span_info(ip)`.
- **[FIX]** Silenced error messages restored: `halt.alert` again prints a warning to stderr and continues execution (matching `errors_halt.md`); Map/Set method fallbacks print "Method … not supported" with a span instead of halting silently.
- **[FIX]** `DatabaseInit` now threads the real `ip` into `handle_database_init` — database initialization errors report the actual source position instead of `0`.
- **[CHG]** Bytecode/global dumps in tests (`[TEST BYTECODE]` etc.) are printed only with `XCX_TEST_DUMP=1`; removed the duplicated `left_rc` declaration line in `vm/utils/table.rs` (the last standing compiler warning — the build is now fully clean).
- **[NEW]** SSRF link-local coverage restored as a spawned-process test (`tests/ssrf_link_local.rs`), replacing the ignored in-process test (which aborted the release test binary via a panic across a JIT FFI frame).

---

## Performance

Main Suite (official config: 20 warmup / 100 runs). The 4.2 reference values are the baseline suite introduced with 4.2.

| Variant | fib(30) | lcg(100M) | sieve | json |
|---|---|---|---|---|
| XCX 4.2, JIT | 12.303 ms | 106.480 ms | 97.814 ms | 0.238 ms |
| **XCX 4.3, JIT** | **10.560 ms** | **104.852 ms** | **36.542 ms** | **0.134 ms** |
| XCX 4.2, `--no-jit` | 186.819 ms | 4328.303 ms | 1876.484 ms | 0.267 ms |
| **XCX 4.3, `--no-jit`** | **204.118 ms** | **4528.251 ms** | **2125.645 ms** | **0.154 ms** |

- **JIT mode improved on all four tests:** fib −14.2%, lcg −1.5%, **sieve −62.7%** (the receiver-refcount elision above), json −43.7%.
- **Interpreter mode (`--no-jit`) regressed on compute-heavy tests:** fib +9.3%, lcg +4.6%, sieve +13.3% (json improved −42.3%). This continues the interpreter-regression trend present since 4.0; addressing it is the main focus of the 4.4 cycle.
- Full comparison against 26 languages/runtimes (XCX 4.3 ranks 10th by geometric mean): see the table in [`README.md`](../README.md).
