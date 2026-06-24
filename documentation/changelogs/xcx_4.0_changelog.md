# XCX 4.0 Changelog

---

## New VM Value Representation

The internal `Value` representation has been changed from a NaN-boxed single 64-bit word (XCX 3.1) to two separate fields `{ bits: u64, tag: u64 }` (16 bytes). The tag is now an explicit integer — zero bitwise operations needed to read the type in the interpreter. Added tags `TAG_CLOSURE` (14) and `TAG_ARENA` (15).

## NaN-Boxing in JIT

JIT registers use a NaN-boxed single 64-bit word in Cranelift — fewer CPU registers, simpler block signatures. `pack_value`/`unpack_value` adapters ensure binary compatibility with the 16-byte `Value` struct on the VM and FFI side. Bit conflict eliminated by shifting the base to `0x7FF0_0000_0000_0000`.

## Tracing JIT (Cranelift)

XCX 3.1 had a JIT in a single file (1780 lines) with a simplified 3-argument signature (`locals_ptr`, `globals_ptr`, `consts_ptr`). XCX 4.0 rewrites the JIT from scratch — 20 files, 7-argument signature, full architecture with type inference, liveness analysis, NaN-boxing and tiered compilation. Hot paths detected by Hotspot — after 50 visits to a given execution point (instruction pointer), trace recording begins. JIT provides up to **34× speedup** over the interpreter on intensive loops.

## JIT Tracing Scope

The JIT records and optimizes a full spectrum of operations — not just arithmetic and loops. A trace can include array operations, iteration over tables and sets, JSON operations with constant path folding (`JsonBindLocalConst`), function calls, fiber operations and random selection. Dedicated fused TraceOps `IncLocalLoopNext` and `IncVarLoopNext` combine increment, condition test and backward jump into a single instruction — eliminating the overhead of three separate opcodes per range loop iteration.

## Guard Failures and Blacklist

After 3 failed guard failures for a given execution point, the trace is marked as unstable and added to the blacklist, causing subsequent compilation attempts to be skipped. The mechanism prevents infinite re-compilation of unstable paths and ensures JIT stability under polymorphic data conditions.

## Tiered Compilation for Functions

Function calls have a separate threshold: calls 1–4 use the pure interpreter, from the 5th call the function is compiled by JIT. A separate mechanism from hotspot loop tracing.

## Recursion in JIT

Recursive calls compiled to direct `call` instructions in native code — bypassing copies through RAM. Fib(30): 45ms → 16ms compared to XCX 3.1.

## Zero-Copy JSON

Internal type `JsonVal` replaces `serde_json::Value` on hot paths. `Object` represented as `Vec<(Arc<String>, JsonVal)>` instead of `HashMap` — linear search for small objects is faster than hashing. `to_string_buf` writes directly to a buffer without intermediate allocations. JSON operations do not allocate intermediate strings — read via raw pointer. JSON benchmark: 112ms → 24ms compared to XCX 3.1.

## Arena Allocator

New per-thread bump allocator (`TAG_ARENA`) — short-lived objects allocated in 4KB chunks without Arc/RwLock overhead. No destructor is called — objects live as long as the arena. Eliminates reference counting cost for temporary values.

## Built-in Method Specialization

Dedicated JIT paths for String, Array, Set, Map, Date — bypassing the generic dispatcher. When types are statically known, direct Cranelift IR is emitted without FFI.

## Constant Folding

Computations on two constant values are performed at JIT compile time — no machine code is generated for the operation itself.

## Memory Optimization

VM stack pre-allocation reduced from 128MB to 1MB per fiber. Reference counting generated only for registers that actually hold pointers — `reg_is_never_ptr` analysis eliminates inc_ref/dec_ref for registers always holding Int/Float/Bool.

## Set Iterators

Iterating over `set:N` no longer allocates a new array on every loop — lazy cache eliminates allocation overhead. Migration from `BTreeSet` to `IndexSet` (`indexmap`) enables O(1) index access for native iterators.

## Closures (Internal Infrastructure)

Internal closure infrastructure — `ClosureObj` with `upvalues: Vec<Arc<UpvalueCell>>`, each cell is `Arc<RwLock<Value>>`. New opcodes: `MakeClosure`, `CloseUpvalue`, `LoadUpvalue`, `StoreUpvalue`. XCX syntax does not yet expose closures to the user — the infrastructure is prepared for future language extensions.

## While Loop Optimization Fix

Shadowing bug in `compile_while` — the `is_less` variable was declared and assigned twice, causing the strict `<` condition to always fall through to the `LessEqual` path. While loops using the `<` operator were not using the dedicated JIT optimization. Fixed by removing the duplicate declaration and assignment.

## Type Inference Fix at Join Points

Bug in `type_inference.rs` — when merging CFG paths, register types were absorbed into `TypeTag::Unknown` because the merge logic did not distinguish unvisited blocks from conflict paths. Introduced `visited: Vec<bool>` — first visits clone types directly, subsequent visits perform merge and degrade to `Unknown` only on actual conflict. Effect: JIT emits more native Cranelift instructions instead of polymorphic FFI calls.

## Dispatch Frame Refactoring

Three methods `run_frame`, `handle_call` and `handle_call_no_jit` contained duplicated frame allocation, stack zeroing and cleanup logic. Extracted three helper methods: `prepare_frame`, `check_jit_warmup`, `cleanup_frame`. Fixed latent bug where `call_depth` dropped below 0 causing panic on underflow. Stack zeroing limited to slots from the parameter boundary to `max_locals` — eliminating redundant zero-fill for active parameter slots.

## Lock-Free String Access Verified

Audit confirmed that `StringObj` is fully lock-free — `Arc<StringObj>` without `RwLock`, zero locks on all string operations (manipulation, conversion, slice, search, print).

## Terminal Fix (Linux)

JIT printed `\n` instead of `\r\n` in `xcx_jit_print` — cursor did not return to the start of the line causing a staircase effect. Fixed. Raw mode did not clear screen remnants on smaller views — added `\x1b[K]` to prints and `\x1b[J]` to flush. XCX 4.0 runs correctly on Linux with identical performance to Windows (difference ~40-50ms in total benchmarks).

## `perf` Module

New high-resolution monotonic timer module for performance benchmarking. Unlike `date.now()`, values returned by `perf` are guaranteed to be monotonic (never go backward) and are not affected by system clock adjustments (NTP/DST).

| Method | Returns | Description |
|---|---|---|
| `perf.ms()` | `i` | Elapsed milliseconds since VM start |
| `perf.us()` | `i` | Elapsed microseconds since VM start |
| `perf.ns()` | `i` | Elapsed nanoseconds since VM start |

```xcx
i: start = perf.ms();
--- code to benchmark
i: elapsed = perf.ms() - start;
>! "Elapsed: " + s(elapsed) + " ms";
```

If the OS lacks a monotonic timer, `halt.fatal` is raised on first invocation. If native microsecond/nanosecond precision is unavailable, the module automatically falls back to the next lower precision level.

## Code Modularization

Codebase grew from **19 `.rs` files** (XCX 3.1) to **269 `.rs` files** (XCX 4.0), from ~18k to ~34k lines of code. In 3.1, VM and execution logic lived in a monolithic `vm.rs`, JIT in a separate `jit.rs` (1780 lines) with a simplified architecture. In 4.0, the largest file is `compiler_method.rs` at 1172 lines — each module has a single responsibility. Compiler split into 25+ files, VM divided into `core/`, `frame/`, `value/`, `object/`, `trace/`, JIT into 20 files with separate modules for code emission, analysis and type inference.

## REPL Improvements

The REPL has been fully reworked using `rustyline 14.0`, replacing the old raw terminal mode handling.

**Vi mode** — navigation and editing now follow Vim keybindings (`EditMode::Vi`) out of the box.

**Syntax highlighting** — ANSI color output in the REPL: string literals in green, comments (`--`) in dark gray, keywords (`func`, `if`, etc.) in cyan.

**Tab completion and hints** — keywords and built-in module names (`math.`, `date.`, `store.`, etc.) are suggested as light gray inline hints while typing and can be completed with Tab.

**Multiline input** — the REPL now supports multiline expressions, allowing blocks (`if`, `for`, `func`, fiber definitions) to be written and evaluated interactively.

## Table Literal Register Overflow Fix

Large table literals caused a compiler panic when the total number of cells (`rows × columns`) exceeded 255. The root cause was the flat `u8` register allocation scheme — each cell in a `table:` literal was mapped to a unique register, overflowing the 256-register limit.

**Two new opcodes** — `TableBegin` allocates an empty table matching the target schema, `TableInitRow` appends a single row by reading `col_count` values from a fixed register base. This allows registers to be recycled across rows instead of allocated per-cell.

**Selective compilation strategy** — tables with `rows × columns ≤ 200` continue to use the existing `TableInit` path (no performance regression for small tables). Above the threshold, the compiler emits `TableBegin` once, then loops over rows resetting the register base index for each iteration.

**`@auto` columns** — `TableInitRow` automatically calculates and inserts auto-increment values, skipping those columns in the register mapping.

Liveness analysis and register manager updated to correctly track `TableBegin` and `TableInitRow` register dependencies.

## REPL Input Engine

The REPL input engine has been rebuilt on top of `crossterm`, replacing raw line-by-line text reading.

**Multiline editing** — free navigation across a multiline buffer using arrow keys (up, down, left, right) without any special mode switching.

**`!exec` command** — automatic block-end detection has been removed in favor of an explicit `!exec` command that manually triggers execution of the written script.

**Updated `!help` and welcome screen** — added `!exec` usage instructions and a CONTACT & SUPPORT section with links to email, GitHub and the website.

## CLI Flags

Added `--no-jit` (disables JIT globally).

---

## Results

The table shows the impact of individual layers — 4.0 interpreter, 4.0 JIT, and the 3.1 reference point:

| Benchmark | XCX 3.1 | XCX 4.0 (no JIT) | XCX 4.0 (JIT) |
|---|---|---|---|
| Loop 100M | 520ms | 7341ms | **119ms** |
| Fib(30) | 45ms | 260ms | **14.28ms** |
| Sieve 100K | 5ms | 23ms | **2.55ms** |
| JSON 1000×100 | 112ms | 66ms | **22.74ms** |
