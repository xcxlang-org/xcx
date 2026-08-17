# Dead-code re-verification pass

**Date:** 2026-08-17
**Type:** Read-only verification. No code was modified, deleted, or commented out.
**Subject:** Independent re-verification of every dead-code claim in `2026-08-16_codebase_audit_dead_code_debt_regression.md` before any removal happens.
**Language docs:** all 14 files in `documentation/language/` confirmed unchanged (mtime 2026-08-14) and read before this pass.

## Verification method (independent of the original audit)

Every item was re-searched from scratch across the **entire workspace** (all directories, all file types; `target/`, `.git/`, and `.md`/log files excluded from the reference census) using word-boundary text search. To cover channels a plain text census can miss, the following were enumerated exhaustively:

- **Identifier pasting:** only 3 `macro_rules!` exist in the crate (`declare_jit_symbols` in `src/jit/symbol_macros.rs:1`, local `read!`/`write!` in `src/jit/analysis.rs:214,223`). All take identifiers literally; no `paste`/`concat_idents`/`quote`/`syn` anywhere. Textual search is therefore complete for identifier references.
- **Conditional compilation:** every `#[cfg]`/`cfg!` in `src/` enumerated. Only OS gates (`windows`/`unix`), `#[cfg(test)]` on test modules, and `#[cfg(feature = "jit")]` at `src/vm/mod.rs:7,20,25` plus `src/runtime/builtin/json/inject.rs:3`. The `jit` feature is default-on; with it off, the entire trace subsystem is compiled **out** (reinforcing its deadness). No dead item is cfg-gated into liveness under any build.
- **String/name dispatch:** the FFI symbol registry (`declare_jit_symbols!` invocation, `src/jit/symbols/mod.rs:7–56+`) contains only `xcx_jit_*` FFI function names — none of the audited functions. Runtime method dispatch by name (`dispatch.rs` `handle_method_call_custom`) routes to `handle_row_custom` / `handle_json_custom` / `RuntimeOps::get_member` — no audited item is reached by name lookup.
- **Trait objects / function pointers:** all audited items are plain functions or inherent methods; none are stored in vtables, maps, or passed as pointers.
- **External consumers:** exactly one `Cargo.toml` exists in the workspace — no nested crate compiles against the `xcx` library, so nothing outside the crate can reach the `pub` items.
- **Entry points:** `main.rs` (CLI: run/check/bytecode/version/help), REPL (`!exec`, `!globals`, `!jit`, `!reset`), and all 199 tests reference none of the audited items (proven by the workspace census: every hit is inside the item's own definition file).

Caveat that applies to the whole report: because `lib.rs` exposes all modules `pub`, every item is technically part of the crate's public library API. "Dead" here means "unreferenced by any code in this repository, reachable from no entry point it defines." If the library is ever published as a dependency, removal becomes a semver-visible API change.

## Verdict table

Legend: CONFIRMED DEAD = re-verified, zero references anywhere in the workspace outside the definition. DISPUTED = a reference the audit missed was found, or the audit's characterization is wrong. UNCERTAIN = intent decision required before removal.

### A. "Entirely dead modules" (9 claims)

| # | Item | Audit verdict | Re-verification verdict | Evidence |
|---|---|---|---|---|
| 1 | `record_op` + file `src/vm/trace/recording_helper.rs` (514 ln) | dead | **CONFIRMED DEAD** | Sole hit is its definition (line 7). It is the only caller of `Recorder::record`/`stop` and of `Hotspot::blacklist` — all dead with it. Removal also removes `pub mod recording_helper;` (`src/vm/trace/mod.rs:5`). |
| 2 | `RegisterManager::compress_registers` + file `src/compiler/register_manager.rs` (275 ln) | dead | **CONFIRMED DEAD** | `RegisterManager` appears only at its own definition/impl (lines 4, 6); `compress_registers` only at line 7. Module declared at `src/compiler/mod.rs:8`. |
| 3 | `print_val`, `run_cmd` + file `src/runtime/builtin/io/print.rs` (50 ln) | dead | **CONFIRMED DEAD** | Each name: 1 hit (own definition). NOTE for removal: `src/runtime/builtin/io/mod.rs:1` (`pub mod print;`) and `:4` (`pub use print::*;` glob re-export) must be removed together. |
| 4 | `Arena`, `with_arena`, `alloc_in_arena`, `CURRENT_ARENA` + file `src/vm/core/arena.rs` (65 ln) | dead | **CONFIRMED DEAD** | All four names referenced only inside `arena.rs`. No other file mentions the allocator. Module declared at `src/vm/core/mod.rs:6`. (The arena **value format** is a separate, partly live concern — see §C/TAG_ARENA.) |
| 5 | `patch_jump`, `patch_jump_to` + file `src/compiler/patch.rs` (23 ln) | dead | **CONFIRMED DEAD** | Definitions only (lines 5, 15). Module declared at `src/compiler/mod.rs:7`. |
| 6 | `StackGuard` + file `src/vm/stack/stack_guard.rs` (26 ln) | dead | **CONFIRMED DEAD** | Hits: own struct/impl (lines 3, 8) plus the re-export `pub use stack_guard::StackGuard;` at `src/vm/stack/mod.rs:5`. The re-export is a link reference, not a call — nothing constructs a `StackGuard`. Removal touches `src/vm/stack/mod.rs:2` and `:5`. |
| 7 | `LoopContext` + file `src/jit/loop_context.rs` (19 ln) | dead | **CONFIRMED DEAD** | Own struct/impl only. Module declared at `src/jit/mod.rs:11`. |
| 8 | `set_op` + file `src/vm/utils/set.rs` (24 ln) | dead | **CONFIRMED DEAD** | Definition only. Set algebra executes via `OpCode::SetUnion`/`SetIntersection` in `src/vm/core/step/collection.rs:197+`. Module declared at `src/vm/utils/mod.rs:4`. |
| 9 | `parse_query_expr` + file `src/frontend/parser/parse_query.rs` (8 ln) | dead | **CONFIRMED DEAD** | Definition only (stub returning `None`). Module declared (private) at `src/frontend/parser/mod.rs:17`. |

### B. Trace-JIT subsystem (8 claims)

| # | Item | Audit verdict | Re-verification verdict | Evidence |
|---|---|---|---|---|
| 10 | `JIT::compile` (`src/jit/jit.rs:50–~481`) | dead | **CONFIRMED DEAD** | Zero callers. Every `.compile(` hit in the crate is `XCXCompiler::compile` (bytecode compiler, different type: `main.rs:247`, `repl.rs:178`, 25 sites in `src/vm/core/tests.rs`). `compile_method`/`compile_fiber_segment` are separate methods with their own callers. `JIT::new` (jit.rs:34) is the only live part of the type (`vm.rs:44`). |
| 11 | `analyze_trace_locals` / `analyze_trace_globals` / `analyze_trace_global_ints` / `analyze_trace_non_ptr_regs` (`src/jit/analysis.rs:319,327,826,875`) | dead | **CONFIRMED DEAD** | All references are `src/jit/jit.rs:16` (import) and `:99,:103,:106,:108` — inside dead `JIT::compile`. Transitively dead. |
| 12 | `Recorder` methods (`start`/`record`/`stop`) (`src/vm/trace/recorder.rs`) | dead | **CONFIRMED DEAD** | `Recorder` is constructed once (`executor.rs:53`) but `recorder.*` member access exists **only** inside dead `recording_helper.rs`. `start()` has zero callers anywhere, so `is_recording` is permanently `false`. Removal touches `executor.rs:16,53`. |
| 13 | `TraceOp` enum (`src/vm/trace/trace_op.rs`, 131 ln) | dead | **CONFIRMED DEAD** | Referenced only inside the dead subsystem (recorder, recording_helper, dead `JIT::compile`, dead analyze fns) plus the `#[cfg(feature="jit")]` re-export `src/vm/mod.rs:21`. |
| 14 | `Trace` struct (`src/vm/trace/trace.rs`) | dead | **CONFIRMED DEAD** | Used only by the never-populated `VM::traces` / `trace_cache` containers, dead `JIT::compile`, and the REPL read below. |
| 15 | `VM::traces` (`src/vm/core/vm.rs:29`) | dead (never populated) | **CONFIRMED DEAD** | Full-crate search for `traces`: declaration (vm.rs:29), initialization empty (vm.rs:43), and exactly one read — REPL `!jit` diagnostics (`repl.rs:126–131`). Zero writes/inserts. The REPL "Loop Traces / JIT-Compiled" rows therefore always display 0. |
| 16 | `Executor::trace_cache` (`src/vm/core/executor.rs:17`) | dead | **CONFIRMED DEAD** | Never populated, never read for compilation; only swapped out/in around fiber execution (`fiber/ops.rs:104,109`). Removal touches those two lines plus `executor.rs:17,54`. |
| 17 | `Hotspot::on_guard_failure` + `guard_failures` field (`src/vm/trace/hotspot.rs:47–57,7`) | dead | **CONFIRMED DEAD** | `on_guard_failure`: definition only. `guard_failures`: field decl, init, and its use inside `on_guard_failure` — nothing else. |
| 18 | *(audit's characterization)* "rest of Hotspot (tick/resize/reset/blacklist) is live and drives method-JIT thresholds" | live | **DISPUTED** | See §"Disputed findings" #1 — the entire Hotspot mechanism is non-functional at runtime; method-JIT thresholds are driven by `call_count`, not Hotspot. |

### C. Vestigial runtime features (5 claims)

| # | Item | Audit verdict | Re-verification verdict | Evidence |
|---|---|---|---|---|
| 19 | `from_closure` (`src/vm/value/heap_object.rs:22`) | dead | **CONFIRMED DEAD** | Definition only. |
| 20 | `as_closure` (`src/vm/value/heap_object.rs:180`) | dead | **CONFIRMED DEAD** | Definition only (body lines 182–183 are its own internals). |
| 21 | `from_arena_string` (`src/vm/value/heap_object.rs:32`) | dead | **CONFIRMED DEAD** | Definition only. |
| 22 | `ClosureObj` / `TAG_CLOSURE` arms | unreachable | **CONFIRMED DEAD** (expanded) | No producer of `TAG_CLOSURE` values exists (`from_closure` is the only one, itself dead). Arms at `ref_count.rs:25,46` and `value.rs:324` are unreachable. Additionally found dead and unnamed by the audit: the whole file `src/vm/object/closure_obj.rs` (`ClosureObj::new` has zero callers), the whole file `src/vm/frame/upvalue_cell.rs` (`UpvalueCell`'s only consumer is `closure_obj.rs`; re-exported at `src/vm/frame/mod.rs:9` and `src/vm/mod.rs:19`), and `Value::is_closure` (`value.rs:289`, zero callers). |
| 23 | `TAG_ARENA` branches in `value.rs:277–287` "never true" / "no code path ever constructs a TAG_ARENA value" | dead branches, value.rs-local cleanup | **DISPUTED (scope)** — see §"Disputed findings" #2 | Runtime-unreachability holds **for the production binary**, but the audit missed a second producer: `from_arena_json` (`heap_object.rs:36`) is called from the `#[cfg(test)]` module at `src/jit/nan_ops.rs:252`. And the TAG_ARENA consumer support is live code spread far beyond `value.rs` (~44 references across 8 files). |

### D. Scattered dead functions (18 audit rows, 26 functions)

| # | Item (location) | Re-verification verdict | Evidence |
|---|---|---|---|
| 24 | `current_frame` (`frame_stack.rs:30`) | **CONFIRMED DEAD** | definition-only hit |
| 25 | `current_frame_mut` (`frame_stack.rs:35`) | **CONFIRMED DEAD** | definition-only hit |
| 26 | `lookup_symbol` (`symbol_table.rs:80`) | **CONFIRMED DEAD** | definition-only hit |
| 27 | `_copy_globals` (`symbol_table.rs:108`) | **CONFIRMED DEAD** | definition-only hit |
| 28 | `has_errors` (`error/reporter.rs:27`) | **CONFIRMED DEAD** | definition-only hit |
| 29 | `Precedence::for_token` (`precedence.rs:25`) | **CONFIRMED DEAD** | definition-only hit; `current_precedence` (`pratt.rs:24`) is the live path |
| 30 | `peek_precedence` (`pratt.rs:44`) | **CONFIRMED DEAD** | definition-only hit |
| 31 | `def_local_int` (`codegen_ctx.rs:301`) | **CONFIRMED DEAD** | definition-only hit |
| 32 | `def_local_bool` (`codegen_ctx.rs:308`) | **CONFIRMED DEAD** | definition-only hit |
| 33 | `def_local_float` (`codegen_ctx.rs:315`) | **CONFIRMED DEAD** | definition-only hit |
| 34 | `def_global_nanboxed` (`codegen_ctx.rs:362`) | **CONFIRMED DEAD** | definition-only hit |
| 35 | `load_const_nanboxed` (`codegen_ctx.rs:394`) | **CONFIRMED DEAD** | definition-only hit |
| 36 | `call_ffi_value_nanboxed` (`codegen_ctx.rs:633`) | **CONFIRMED DEAD** | definition-only hit |
| 37 | `emit_div_poly` (`emit_arith.rs:676`) | **CONFIRMED DEAD** | definition-only hit; live path is `emit_div_int`/`emit_div_float` |
| 38 | `emit_mod_poly` (`emit_arith.rs:680`) | **CONFIRMED DEAD** | definition-only hit; live path is `emit_mod_int` |
| 39 | `emit_random_int` (`emit_call.rs:429`) | **CONFIRMED DEAD** | definition-only hit; live path is FFI symbol `xcx_jit_random_int` |
| 40 | `emit_random_float_call` (`emit_call.rs:457`) | **CONFIRMED DEAD** | definition-only hit; live path is FFI `xcx_jit_random_float` |
| 41 | `emit_env_get` (`emit_misc.rs:76`) | **CONFIRMED DEAD** | definition-only hit; live path is FFI `xcx_jit_env_get` |
| 42 | `emit_env_args` (`emit_misc.rs:92`) | **CONFIRMED DEAD** | definition-only hit; live path is FFI `xcx_jit_env_args` |
| 43 | `emit_loop_next_generic` (`emit_control.rs:126`) | **CONFIRMED DEAD** | definition-only hit |
| 44 | `emit_method_yield` (`emit_control.rs:338`) | **CONFIRMED DEAD** | definition-only hit |
| 45 | `decode_intcc` (`abi.rs:37`) | **CONFIRMED DEAD** | definition-only hit (`decode_floatcc` at :50 is used) |
| 46 | `shallow_clone` (`json_val.rs:274`) | **CONFIRMED DEAD** | definition-only hit |
| 47 | `flush_stdin_device` (`input.rs:24`) | **CONFIRMED DEAD** | definition-only hit; re-exported only via the `pub use input::*` glob |
| 48 | `inject_json_into_table` (`table.rs:255`) | **CONFIRMED DEAD** | definition-only hit; live `.inject()` runs via `OpCode::JsonInject`/`JsonInjectLocal` |
| 49 | `handle_database_member_access` (`db/ddl.rs:59`) | **CONFIRMED DEAD** | definition-only hit; live member access is `dispatch.rs:89` → `RuntimeOps::get_member` |

## Disputed findings (detail)

### 1. The audit's "rest of Hotspot is live" statement is wrong — the entire Hotspot mechanism is non-functional at runtime

The audit (§3.2) said: *"rest of Hotspot (tick/resize/reset/blacklist) is live and drives method-JIT thresholds."* Re-verification found this incorrect on every count:

- **Method-JIT thresholds are not driven by Hotspot.** Method compilation triggers on `chunk.call_count` / `f.call_count` reaching `vm.jit_threshold` (or the fixed `5` in the FFI-entry path): `executor.rs:148–170` (`check_jit_warmup`) and `jit_helpers.rs:160–170`. These paths never touch Hotspot.
- **`Hotspot::resize` has zero callers** (the only `resize` hits are `hotspot.rs:22` — a `Vec::resize` inside the method's own body — and a comment in `input.rs:165`). Consequently `counts` starts as `Vec::new()` and is never grown.
- **`Hotspot::tick`'s only caller** is the fiber-resume path (`fiber/ops.rs:87`). Its guard is `ip < self.counts.len()` on a vector that is permanently empty — the only other `counts` interactions are swap-out/swap-in with another empty `Vec::new()` around fiber execution (`fiber/ops.rs:103,108`). `tick` therefore **always returns false at runtime**.
- **`Hotspot::reset` and `Hotspot::blacklist`** have no live callers either (their only callers are dead `on_guard_failure` and dead `recording_helper.rs:217`).
- `executor.rs:31–32,52` constructs the Executor's `Hotspot` and sets `threshold` — values that can never be consulted. The REPL "Hotspot Limit" row (`repl.rs:140`) prints `vm.jit_threshold`, not the Hotspot struct.

**Implication — this is the important part:** `compile_fiber_segment` (`fiber/ops.rs:89`, implementation `src/jit/compiler_fiber.rs`, 372 lines) is **statically called but dynamically unreachable**: its only call site sits behind `if self.hotspot.tick(fib_ip)`, which can never be true. The comment at `fiber/ops.rs:99–100` ("fibers use compile_fiber_segment, not the trace recorder") documents an intent that does not match behavior — **fiber segments are never JIT-compiled in the current binary**; fibers always fall through to the interpreter.

This makes the Hotspot/compile_fiber_segment cluster **UNCERTAIN** rather than plainly dead: either
(a) the missing `resize` call is a **bug** (fiber JIT was meant to engage — then `tick`, `resize`, and `compiler_fiber.rs` must be *fixed and kept*, and the "dormant" trace-JIT removal plan shrinks), or
(b) fiber hotspot compilation was abandoned (then `Hotspot` in full, the `tick` call site, and `compile_fiber_segment`/`compiler_fiber.rs` join the dead set — another ~400 lines).

Deciding (a) vs (b) requires the maintainer's intent; it cannot be resolved from the code. Note that today's passing test suite does not distinguish them (no test asserts fiber JIT engagement; `jit_multi_return_with_jit`-style tests cover method JIT).

### 2. TAG_ARENA: audit's runtime claim holds for production, but its inventory and cleanup scope were wrong

Two corrections to the audit:

1. **Missed producer:** the audit stated "no code path ever constructs a `TAG_ARENA` value." That is wrong as written: `from_arena_json` (`src/vm/value/heap_object.rs:36`) constructs one and is called at `src/jit/nan_ops.rs:252` — inside the `#[cfg(test)]` module (nan_ops tests, added 2026-08-16 per the work doc). The claim is true only for the **production binary**; `cargo test` does construct arena-tagged values.
2. **Understated blast radius:** the audit framed TAG_ARENA cleanup as optional value.rs-only branch removal. In reality TAG_ARENA consumer support is live, executed code in at least 8 files (~44 references): `value.rs` (predicates 277–289, `tag()` 311–312, dec_ref skips 349/355, string-append 115–116, `as_str` 467), `heap_object.rs` (`arena_inner_tag` 42, `arena_ptr` 48, `as_string` 55–58, 125–126, 297–299), `runtime/ffi_helpers/json_ffi.rs` (6 sites), `vm/utils/json.rs:70–71`, `vm/core/jit_helpers.rs:615–616`, `vm/core/step/module.rs:136–137`. Removing TAG_ARENA support means rewriting all of these plus updating the nan_ops tests — a cross-cutting change, not a local cleanup.

Related new dead items found while verifying: `Value::is_arena` (`value.rs:277`) has zero callers (the audit listed the `is_*` TAG_ARENA *branches* but not that this standalone predicate is itself dead), and `from_arena_json` is production-dead with a single test caller (removal requires updating `nan_ops.rs:252` or keeping a minimal constructor for the predicate tests).

## New dead items discovered during re-verification (not in the original audit)

| Item | Location | Evidence |
|---|---|---|
| `Value::is_closure` | `src/vm/value/value.rs:289` | zero callers |
| `Value::is_arena` | `src/vm/value/value.rs:277` | zero callers |
| `src/vm/object/closure_obj.rs` (whole file) | `ClosureObj` + `ClosureObj::new` | only consumers are dead `from_closure`/`as_closure`; declared at `src/vm/object/mod.rs:6` |
| `src/vm/frame/upvalue_cell.rs` (whole file, incl. `Drop` impl) | `UpvalueCell` | only consumer is dead `closure_obj.rs`; re-exported at `src/vm/frame/mod.rs:9`, `src/vm/mod.rs:19` |
| `from_arena_json` (production-dead) | `src/vm/value/heap_object.rs:36` | single caller is a `#[cfg(test)]` test (`nan_ops.rs:252`) |
| `Hotspot::resize`, `Hotspot::reset`, `Hotspot::blacklist` | `src/vm/trace/hotspot.rs:20,39,54` | zero live callers (only dead `on_guard_failure` / dead `recording_helper`) |
| Hotspot struct + fiber JIT path (dynamically dead) | `src/vm/trace/hotspot.rs`, `fiber/ops.rs:86–95`, `src/jit/compiler_fiber.rs` | `tick` can never fire (see Disputed #1) — pending intent decision |

## Summary counts

| Verdict | Count |
|---|---|
| CONFIRMED DEAD (claims from the audit, independently re-verified) | **46 of 48** discrete claims (all 9 dead modules; 7 of 8 trace-subsystem claims; 4 of 5 vestigial-feature claims incl. 26 scattered functions) |
| DISPUTED | **2** — (1) audit's "rest of Hotspot is live" characterization (wrong; mechanism non-functional, `compile_fiber_segment` dynamically unreachable); (2) TAG_ARENA "no code path ever constructs" (test-only producer missed) and cleanup scope (8-file blast radius, not value.rs-local) |
| UNCERTAIN | **1** — fate of the Hotspot/fiber-JIT cluster (`compile_fiber_segment`, `compiler_fiber.rs`): bug (fix and keep) vs vestige (remove) requires a maintainer decision |
| New dead items found beyond the audit | 7 (listed above) |

## Go/no-go recommendation for the planned removal

**GO — for the confirmed set**, i.e. everything except the two disputed/uncertain clusters: all 9 dead modules, `JIT::compile` + the four `analyze_trace_*` functions, `Recorder`/`TraceOp`/`Trace` plumbing, `VM::traces`, `trace_cache`, `on_guard_failure`/`guard_failures`, the closure vestiges (`from_closure`, `as_closure`, `closure_obj.rs`, `upvalue_cell.rs`, `is_closure`), and all 26 scattered functions. Removal notes that must be respected (each verified above): delete the accompanying `mod` declarations and re-exports (`io/mod.rs:1,4`; `stack/mod.rs:2,5`; `compiler/mod.rs:7,8`; `vm/utils/mod.rs:4`; `vm/core/mod.rs:6`; `jit/mod.rs:11`; `parser/mod.rs:17`; `vm/trace/mod.rs`; `vm/mod.rs:20–21`; `vm/mod.rs:19` for `UpvalueCell`; `vm/object/mod.rs:6`); touch `executor.rs:15–17,31–32,52–54` (hotspot/recorder/trace_cache fields), `fiber/ops.rs:103–109` (swap block), and `repl.rs:122–145` (the `!jit` "Loop Traces/JIT-Compiled" rows become meaningless and should be dropped or redesigned).

**NO-GO — for two clusters, pending decisions:**

1. **Hotspot + `compile_fiber_segment`/`compiler_fiber.rs`:** do not remove until the maintainer decides whether fiber-segment JIT is supposed to work. The current binary never JIT-compiles fiber segments — if that is a bug, the correct action is to restore the missing `resize` wiring, which turns `tick`, `resize`, and `compiler_fiber.rs` into live code that must be kept. This decision also changes what "trace subsystem removal" means.
2. **TAG_ARENA support:** do not treat as a value.rs-local cleanup. It is an 8-file cross-cutting change with a test dependency (`nan_ops.rs:252`). Schedule it separately, with the nan_ops predicate tests updated in the same change.

One process caveat: the confirmed removals delete `Recorder` fields and swap logic that `cargo test`'s passing suite does not exercise behaviorally, but re-running `cargo test --release` and the benchmark suite (per AGENTS.md rule 3) after each removal batch is mandatory before merge, not optional.

## Documentation impact

This document corrects two statements in `2026-08-16_codebase_audit_dead_code_debt_regression.md`, which should no longer be relied on as written:
- §3.2's "rest of Hotspot (tick/resize/reset/blacklist) is live and drives method-JIT thresholds" — wrong (see Disputed #1).
- §3.4's "no code path ever constructs a `TAG_ARENA` value" and its framing of TAG_ARENA cleanup as value.rs-local — wrong in scope (see Disputed #2).

The 2026-08-16 audit's other findings (dead modules, scattered functions, technical debt, benchmark results, for-step regression) were re-verified and stand.
