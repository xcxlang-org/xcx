# XCX 3.1 — Changelog (3.0 → 3.1)
**May 2026 · Type: RELEASE**

---

## Compiler / JIT

- **[FIX]** Cross-function trace collision — JIT traces were keyed by bytecode offset (`ip`) alone, causing traces from different functions to alias each other when their offsets coincided. Traces are now keyed by `(func_id, ip)`. The executor flushes its local trace cache on every call-frame transition.
- **[FIX]** Nested calls compiled as NOPs — the method JIT only emitted real call code for self-recursive calls. Calls to any other function (e.g. `hypot → sqrt_f`) were silently compiled as no-ops, leaving downstream registers uninitialised. All call targets now dispatch through `xcx_jit_call_recursive` with a dynamic `func_idx`.
- **[FIX]** Polymorphic float arithmetic bypassed in method JIT — core arithmetic opcodes (`Add`, `Sub`, `Mul`, `Less`, `Equal`, …) are NaN-boxed and handle both integers and doubles. The non-loop method JIT assumed integer-only operands, corrupting float values. Non-loop helper arithmetic now falls back to the interpreter; loop JIT remains active.

---

## Math Library

- **[NEW]** XCX 3.1 ships with a new version of mathlib.

---

## VM

- **[CHG]** Named global-vector capacity constant (`MAX_GLOBALS = 4096`) — previously a hard-coded `1024`, which could silently overflow in programs with large import graphs.
- **[CHG]** Named pool initial-capacity constant (`POOL_INITIAL_CAPACITY = 32`) — replaces scattered `Vec::with_capacity(32)` literals in executor initialisation.
- **[NEW]** Constant pool deduplication — `add_constant` now linear-scans the pool before inserting. Duplicate integers, floats, and booleans reuse the existing slot instead of allocating a new one. String constants continue to use the existing `HashMap` path.
- **[INT]** `FunctionChunk` clone on frame entry is intentional — a refactor to pass `&FunctionChunk` instead conflicts with the `&mut self` borrow required by the executor during frame execution (`E0502`). The clone is cheap due to `Arc` interior sharing.

---

## Diagnostics

- **[NEW]** Multi-file error locations — `Reporter` and `Parser` now carry an optional filename. Errors are prefixed `[file:line:col]`. The `include` expander forwards the included file's path to its sub-parser, so syntax errors in imported files report the correct source location.
- **[NEW]** `Reporter::warn` — emits non-fatal `WARN`-level diagnostics (yellow) from the parser and sema passes.
- **[INT]** Documented error code ranges in `checker.rs`: `S100–S199` type/signature errors, `S200–S299` fiber/generator control errors, `S300–S399` relational/schema errors.

---

## Build

- **[INT]** Zero warnings across the full workspace (`cargo check`, `cargo build`).
- **[INT]** 151 integration tests pass, 0 failures.

---

## Performance

Measured on Ryzen 7 5800X / 32 GB RAM / Windows 11.

| Benchmark     | XCX 3.0  | XCX 3.1  |
|---------------|----------|----------|
| Loop 100M     | 521 ms   | 520 ms   |
| Fibonacci(30) | 60 ms    | 45 ms    |
| Sieve 100K    | 5 ms     | 5 ms     |
| JSON parse    | 118 ms   | 112 ms   |

3.1 focuses on correctness and compiler internals. Runtime performance is largely unchanged from 3.0, with minor improvements to Fibonacci and JSON parsing.
