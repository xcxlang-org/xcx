# Codebase audit: dead code, technical debt, baseline regression check

**Date:** 2026-08-16
**Type:** Read-only audit. No source files were modified or deleted.
**Scope:** `src/` (43,269 lines of Rust), workspace root artifacts, benchmark tooling, test status, performance vs `baseline.json`.

## Summary

| Check | Result |
|---|---|
| `cargo build --release` | PASS — exit 0, exactly 1 warning (pre-existing `left_rc`, `src/vm/utils/table.rs:69`). No new warnings. |
| `cargo test --release` | PASS — 199 passed, 0 failed, 1 ignored (documented reason). Exit code 0. |
| Benchmarks vs `baseline.json` | **1 BLOCKER**: `for step` +35% at median (fast mode absent in 24/24 runs). 1 watch item (`str concat` +4.5% median). All other entries at parity or better; above-baseline loop-suite suite-averages are explained by the documented per-process bimodality. |
| Dead code | ~2,400 lines call-graph-verified dead (~5.5% of `src/`), incl. 8 entirely dead modules and a dormant trace-JIT subsystem. |
| Technical debt | 3 `DEBUG:` prints on production paths, 1 silenced `halt.alert` (diverges from language docs), 6 commented-out error messages, 1 FIXME, broken baseline comparison in `run_xcx_only.py`, ignored SSRF security test. |
| **Ready to merge into main workspace** | **NO** — blocked by the `for step` regression and the non-functional baseline comparison in the verification runner. |

---

## 1. Build and test status (verified this session)

**Build.** `cargo build --release` → exit 0. Sole warning:

```
warning: unused variable: `left_rc`
  --> src\vm\utils\table.rs:69:9
```

This is the known pre-existing warning. Zero new warnings. Confirmed across two independent invocations (build and test-compile).

**Tests.** `cargo test --release` → exit 0.

| Test binary | Passed | Failed | Ignored |
|---|---|---|---|
| `xcx` lib unit tests (`src/lib.rs`) | 37 | 0 | 1 |
| `xcx` bin (`src/main.rs`) | 0 | 0 | 0 |
| `tests/json_concurrency.rs` | 1 | 0 | 0 |
| `tests/xcx_runner.rs` | 161 | 0 | 0 |
| Doc-tests | 0 | 0 | 0 |
| **Total** | **199** | **0** | **1** |

The single ignored test is `test_ssrf_protection_link_local` (`src/vm/core/tests.rs:236`), ignored with reason `"Panics across FFI boundaries which causes hard abort in release tests"`. Consequence: the documented SSRF link-local block (`169.254.x.x`, `json_http.md` Security Constraints) has no automated coverage. Flagged as technical debt (§4.7), not a test failure.

---

## 2. Benchmarks vs `baseline.json`

Method: `xcx-benchmarks/run_xcx_only.py` full config (MAIN 20 warmup/100 runs, LOOP 2/8, FUNC 20/100), plus follow-up distribution runs (12–24 processes per benchmark) for every loop-suite entry that measured above baseline. All numbers are ms.

**Important:** the runner's embedded `BASELINE` dict is stale and its keys do not match the parsed metric names for the LOOP and FUNC suites, so its `ALL OK` verdict is not evidence — see §4.6. All comparisons below were computed by hand against `baseline.json` (authoritative per audit instructions).

### Main suite (average of 100 runs)

| Benchmark | baseline.json | measured | delta | verdict |
|---|---|---|---|---|
| FIB (30) | 10.92 | 11.089 | +1.5% | parity (within machine noise; 100-run avg) |
| LCG (100M) | 105.70 | 107.344 | +1.6% | parity |
| SIEVE | 36.76 | 37.196 | +1.2% | parity (reflects the 2026-08-16 getvar elision win, 97.8 → 37.2) |
| JSON | 0.12 | 0.120 | 0.0% | match |

### Func & arith (average of 100 runs)

| Benchmark | baseline.json | measured | delta | verdict |
|---|---|---|---|---|
| CROSS FUNC (1M) | 16.390 | 15.816 | −3.5% | beats |
| INLINE ARITH (1M) | 0.522 | 0.452 | −13.4% | beats |
| ARRAY ALLOC (1M) | 22.316 | 19.963 | −10.6% | beats |
| GEO MEAN | 5.66 | 5.23 | −7.7% | beats |

### Loop suite (suite run: average of 8 runs; plus distribution evidence)

| Benchmark | baseline.json | suite avg | delta | 12-run distribution | verdict |
|---|---|---|---|---|---|
| NESTED | 260.65 | 300.057 | +15.1% | 9× 213.1–214.6, 221.4, 223.7, 425.1 | bimodal (documented); fast mode beats baseline by −18% |
| **STEP** | **40.10** | **64.144** | **+60.0%** | **24 runs: 53.4–62.6, median 53.96** | **REGRESSION — blocker** |
| WHILE | 237.33 | 229.789 | −3.2% | — | beats |
| WHILE_CD | 225.72 | 251.358 | +11.4% | 9× 213.5–214.8, 218.0, 225.7, 427.0 | bimodal; fast mode beats baseline by −5% |
| ARRAY | 5.01 | 5.024 | +0.3% | — | match |
| SET | 4.70 | 5.499 | +17.0% | 4.449–4.970, median ≈ 4.9 | suite avg inflated by tail; median +4% ≈ parity |
| BREAK | 431.38 | 449.813 | +4.3% | 416.5–436.9, median ≈ 422 | parity (median −2.2%) |
| CONTINUE | 574.54 | 553.319 | −3.7% | — | beats |
| ARITH | 2.19 | 2.285 | +4.3% | 2.103–2.136, median 2.11 | parity (median −3.7%) |
| CONCAT | 2.64 | 2.914 | +10.4% | 2.722–3.148, median ≈ 2.76 | slightly above (+4.5% median) — watch |
| TOTAL | 1784.26 | 1864.20 | +4.5% | all-fast-mode reconstruction ≈ 1701 (−4.7%) | see below |

### The blocker: `for step`

`bench_02_for_step.xcx` (three nested `@step 2` loops, 500³ iterations):

- 24 fresh processes: every run landed between 53.4 and 62.6 ms (median 53.96). **No run came near the 40.10 ms baseline; no fast mode exists in the current binary.**
- Other baseline.json entries are reproducible on this machine within ~2% (FIB +1.5%, LCG +1.6%, SIEVE +1.2%, JSON exact), so the 40.10 figure is not a foreign-machine artifact.
- The documented bimodality (`2026-08-16_jit_getvar_receiver_inc_elision.md`) covers `triple for` / `while up` / `while down` and was verified for those benchmarks above (fast ~213 ms modes present in most runs). It does not explain `for step`: there the distribution is tight around ~54 with a single high outlier, and no 40 ms mode appears at all.
- The regression is +35% at median vs baseline. Per the audit constraint ("if any benchmark is below baseline, report it as a blocker"), **this is a blocker**. Root cause was not investigated in this read-only pass. Timing note: `baseline.json` (updated 2026-08-16 18:06) already contains STEP 40.10, the same value carried in the older `run_xcx_only.py` dict, so the 40.10 figure predates today's getvar-elision change; when the step regression was introduced cannot be determined from this workspace (no VCS history available here).

### TOTAL interpretation

The suite TOTAL 1864.20 (+4.5%) overstates the situation: NESTED, WHILE_CD and BREAK suite-averages were inflated by slow-mode draws, and the medians of all three are at or better than baseline. Reconstructing the total from per-benchmark medians/fast modes yields ≈ 1701 ms, i.e. −4.7% vs baseline. The material deficit is concentrated in `for step` (≈ +14 ms) with a minor contribution from `str concat` (≈ +0.12 ms).

### Loop-suite gate noise (methodology debt)

LOOP config is 2 warmup / 8 runs per benchmark, each run a fresh process, while at least three benchmarks are per-process bimodal (~2× apart). An 8-run average of a bimodal benchmark has a variance large enough to flip a gate decision run-to-run. Recommendation: raise LOOP runs (e.g. 20+) and gate on the median, or per-benchmark min-of-modes, before using the loop suite as a merge gate.

---

## 3. Dead code findings

Method: every `fn` name in `src/` was enumerated and cross-referenced textually across the entire repository (`src/`, `tests/`, and all other project directories; nothing is invoked via build scripts or codegen). Candidates with zero references outside their own definition were then individually traced: for each, the live implementation that superseded it was identified. Rust's dead-code lint cannot flag any of this because `lib.rs` exposes every module `pub` and there are no `#[allow(dead_code)]` attributes anywhere. Every item below has **zero call sites anywhere in the repository**.

### 3.1 Entirely dead modules (no reference to any item outside the module itself)

| File | Lines | Content | Superseded by |
|---|---|---|---|
| `src/vm/trace/recording_helper.rs` | 514 | `record_op` — bytecode→TraceOp recorder | never called; part of dormant trace JIT (§3.2) |
| `src/compiler/register_manager.rs` | 275 | `RegisterManager::compress_registers` — old register compaction pass | current compiler emits final registers directly |
| `src/runtime/builtin/io/print.rs` | 50 | `print_val` (`>!`), `run_cmd` (`.terminal !run`) | `OpPrint` handled in `src/vm/core/step/module.rs:215–250` via `write_buffered`; `!run` via `src/runtime/ffi_helpers/terminal_ffi.rs` |
| `src/vm/core/arena.rs` | 65 | `Arena`, `CURRENT_ARENA`, `with_arena`, `alloc_in_arena` | nothing constructs `TAG_ARENA` values (see §3.4) |
| `src/compiler/patch.rs` | 23 | `patch_jump`, `patch_jump_to` | jump targets fixed inline during emission |
| `src/vm/stack/stack_guard.rs` | 26 | `StackGuard` (recursion-depth guard) | 800-frame cap enforced by frame-count check elsewhere |
| `src/jit/loop_context.rs` | 19 | `LoopContext` | unused since trace compiler dormancy |
| `src/vm/utils/set.rs` | 24 | `set_op` | set algebra via `OpCode::SetUnion`/`SetIntersection` in `src/vm/core/step/collection.rs:197+` |
| `src/frontend/parser/parse_query.rs` | 8 | `parse_query_expr` (stub returning `None`) | query/where parsing lives in `parse_expr.rs` / `sema/check/check_query.rs` |

### 3.2 Dormant trace-JIT subsystem (~1,230 lines)

The tracing JIT is unreachable end-to-end while the method JIT (`compile_method` / `compile_fiber_segment`) is the live compilation path:

- `src/jit/jit.rs:50–481` — `JIT::compile`, the trace-compiler entry point. Zero callers (the `JIT` struct itself is live: `vm.rs:30,44`). Only `JIT::new` (lines 34–48) is used.
- `src/vm/trace/recorder.rs` — `Recorder` is constructed in `executor.rs:53`, but `Recorder::start` is never called, so `is_recording` is permanently `false` and `recording_trace` permanently `None`; `record`/`stop` are only reachable from the dead `record_op`.
- `src/vm/trace/trace_op.rs` (131 lines, `TraceOp` enum) and `src/vm/trace/trace.rs` (`Trace` struct) — consumed only by the dead compile path and the never-populated containers below.
- `src/jit/analysis.rs` — `analyze_trace_locals` (line 319), `analyze_trace_globals` (327), `analyze_trace_global_ints` (826), `analyze_trace_non_ptr_regs` (875): called only from dead `JIT::compile`.
- `VM::traces` map (`src/vm/core/vm.rs:29`) — never populated; its only read (`src/repl/repl.rs:126`, REPL `!jit` diagnostics) therefore always sees an empty map.
- `Executor::trace_cache` (`src/vm/core/executor.rs:17`) — never populated or read for compilation; only swapped out/in during fiber switches (`src/runtime/builtin/fiber/ops.rs:104–109`).
- `Hotspot::on_guard_failure` + `guard_failures` field (`src/vm/trace/hotspot.rs:47–57`) — guard failures cannot occur without traces; rest of `Hotspot` (tick/resize/reset/blacklist) is live and drives method-JIT thresholds.

The 2026-08-16 work doc explicitly calls the trace compiler "currently dormant", so this is a deliberate state, not an accident — but it is dead code by this audit's definition (unreachable from any entry point) and should either be reactivated or removed.

### 3.3 Vestigial runtime features (constructed nowhere)

- `from_closure` (`src/vm/value/heap_object.rs:22`) and `as_closure` (line 180) — dead. `ClosureObj` and the `TAG_CLOSURE` refcount arms (`src/vm/value/ref_count.rs:25,46`) are unreachable: the only producer of `TAG_CLOSURE` values is the dead `from_closure`.
- `from_arena_string` (`src/vm/value/heap_object.rs:32`) — dead; the only producer of `TAG_ARENA` values (pairs with the dead arena module).

### 3.4 Unreachable runtime branches (live code, never-true conditions)

`src/vm/value/value.rs:277–287` — every type predicate (`is_string`, `is_array`, `is_set`, `is_map`, `is_table`, `is_func`, `is_json`, `is_fiber`, `is_row`, `is_bool_array`) carries a `TAG_ARENA && arena_inner_tag(...) == ...` alternative. Since no code path ever constructs a `TAG_ARENA` value (§3.3), these branches are never true. Removal is an optional cleanup; flagged separately because editing hot type predicates touches every VM operation.

### 3.5 Scattered dead functions (zero call sites, live replacement identified)

| Location | Function | Live replacement |
|---|---|---|
| `src/vm/frame/frame_stack.rs:30,35` | `current_frame`, `current_frame_mut` | direct `frames.last()` access at call sites |
| `src/sema/symbol/symbol_table.rs:80` | `lookup_symbol` | `lookup` (line 66) |
| `src/sema/symbol/symbol_table.rs:108` | `_copy_globals` (underscore-prefixed deliberately) | — |
| `src/error/reporter.rs:27` | `has_errors` | `errors.is_empty()` at call sites |
| `src/frontend/parser/precedence.rs:25` | `Precedence::for_token` | `current_precedence` (`pratt.rs:24`) |
| `src/frontend/parser/pratt.rs:44` | `peek_precedence` | `current_precedence` |
| `src/jit/codegen_ctx.rs:301,308,315` | `def_local_int`, `def_local_bool`, `def_local_float` | generic `def_local` |
| `src/jit/codegen_ctx.rs:362,394,633` | `def_global_nanboxed`, `load_const_nanboxed`, `call_ffi_value_nanboxed` | non-nanboxed variants |
| `src/jit/emit_arith.rs:676,680` | `emit_div_poly`, `emit_mod_poly` | `emit_div_int` (line 68) / `emit_mod_int` (153) |
| `src/jit/emit_call.rs:429,457` | `emit_random_int`, `emit_random_float_call` | FFI symbols `xcx_jit_random_int/float` (`jit/builder.rs:25–26`) |
| `src/jit/emit_misc.rs:76,92` | `emit_env_get`, `emit_env_args` | FFI symbols `xcx_jit_env_get/args` (`jit/builder.rs:124–125`) |
| `src/jit/emit_control.rs:126` | `emit_loop_next_generic` | specialized loop-next emitters |
| `src/jit/emit_control.rs:338` | `emit_method_yield` | FFI fallback for yield |
| `src/jit/abi.rs:37` | `decode_intcc` | `decode_floatcc`-style direct use; `IntCC` decoded where needed |
| `src/vm/object/json_val.rs:274` | `shallow_clone` | — |
| `src/runtime/builtin/io/input.rs:24` | `flush_stdin_device` | — |
| `src/vm/utils/table.rs:255` | `inject_json_into_table` | `OpCode::JsonInject` handling (`step/module.rs`) |
| `src/runtime/builtin/db/ddl.rs:59` | `handle_database_member_access` | `dispatch.rs` `handle_method_call_custom` → `RuntimeOps::get_member` |

### 3.6 Not dead (explicitly cleared)

- All 161 `OpCode` variants: each is referenced outside `opcode.rs` (no dead opcodes).
- `ARC_STRONG_COUNT_OFFSET` + predicate tests in `src/jit/nan_ops.rs`: kept deliberately per the 2026-08-16 work doc (layout facts from the reverted atomic-inc experiment), with test coverage.
- All `eprintln!` calls carrying R-codes (`R303`, `R401`, …): these are the documented halt/error reporting channel, not debug output.

### 3.7 Volume estimate

Whole dead files 1,004 lines + `JIT::compile` ≈ 430 + dead `analysis.rs` functions ≈ 60 + scattered functions ≈ 250 + recorder/trace plumbing ≈ 200 → **≈ 2,400 lines (~5.5% of `src/`)**.

---

## 4. Technical debt findings

### 4.1 `DEBUG:`-labeled prints on production dispatch paths
- `src/vm/core/dispatch.rs:37` — `eprintln!("DEBUG: receiver is not ptr/date. …")`
- `src/vm/core/dispatch.rs:61` — `eprintln!("DEBUG: unknown tag …")`
- `src/runtime/builtin/json/mod.rs:324` — `eprintln!("DEBUG: Method {:?} not supported for JSON", kind)` **with the correct span-info version commented out on the next line**.

All three sit on reachable interpreter fallback paths that increment `error_count` and halt. They violate the project's error-code convention (cf. `R501` at `executor.rs:342`) and, for the JSON arm, drop the span info every other builtin provides. Proposed action: replace with the standard `R50x`-style message including `current_span_info(ip)` (the correct JSON message already exists, commented out, at `json/mod.rs:325`).

### 4.2 Silenced `halt.alert` — behavior diverges from language docs
`src/vm/core/step/module.rs:312–316`: the `HaltAlert` arm binds the message and then does nothing — the print is commented out (`module.rs:314`). `documentation/language/errors_halt.md` specifies `halt.alert` "Prints a warning to stderr; continues execution". Currently it silently continues. This is either a bug (restore the print) or a docs change (remove the promise); it must not stay divergent.

### 4.3 Commented-out error messages leaving silent halts
- `src/runtime/builtin/map/ops.rs:198–202` — fallback arm halts with **no output at all** (message commented out at line 199).
- `src/runtime/builtin/set/ops.rs:116–120` — same pattern (line 117).
- `src/vm/core/step/module.rs:314` — see §4.2.
- `src/runtime/builtin/json/mod.rs:325`, `src/runtime/builtin/string/…` etc. — see §4.1.

A halt with no diagnostic violates the "fail loudly" rule: the user's program stops with nothing on stderr. Proposed action: restore the span-info messages (they are already written, one line below each site).

### 4.4 FIXME marker in source
`src/vm/core/step/module.rs:38` — `// FIXME: ip = 0? Need to pass ip if needed for error report` (database init passes `ip = 0`, so span info is wrong for that op's error reports). Source-code FIXME markers are prohibited by AGENTS.md; this one also describes a real diagnostic-quality gap.

### 4.5 Unconditional debug dumps inside tests
`src/vm/core/tests.rs:625, 741–744, 754–759, 904–915` — `[TEST BYTECODE]` / `[TEST GLOBALS]` / `[DEBUG]` full bytecode dumps printed unconditionally during `cargo test`. Test-only, but it makes test output unusable for triage and the `[DEBUG]` block at 904–915 dumps every function's bytecode on every run. Proposed action: gate behind an env var or delete.

### 4.6 Benchmark verification runner: comparison partially non-functional
`xcx-benchmarks/run_xcx_only.py`:
1. **Stale embedded baseline.** Its `BASELINE` dict still carries pre-getvar-elision MAIN numbers (fib 11.92, lcg 106.480, sieve 97.814, json 0.15) vs `baseline.json` (10.92 / 105.70 / 36.76 / 0.12). A build that regressed sieve back to ~90 ms would still print "ok".
2. **Key mismatch for LOOP and FUNC.** Parsed metric keys are e.g. `for step`, `triple for`, `for set`, `array_alloc_1m`, `cross_func 1m`; the dict keys are `step`, `nested`, `set`, `array alloc`, `cross func`. Every LOOP/FUNC row therefore prints `(no baseline)` and the comparison silently no-ops — the `ALL OK` verdict of this session's run was produced while comparing nothing for two of the three suites.
3. **Tolerance ±15%** would mask regressions well above the "zero regression" bar the project actually enforces.

Proposed action: read `baseline.json` directly instead of embedding numbers, fix key mapping, reduce tolerance. Note: this finding partially invalidates the verification-tooling claim in `2026-08-16_jit_getvar_receiver_inc_elision.md` ("compares against baseline.json") — for LOOP/FUNC it does not, in fact, compare.

### 4.7 Ignored security test
`src/vm/core/tests.rs:236` — `test_ssrf_protection_link_local` (`#[ignore]`). The SSRF link-local block documented in `json_http.md` has no automated coverage. Proposed action: convert to a spawned-process test (avoids the FFI-panic-abort problem) or cover the guard predicate as a unit test.

### 4.8 Loop-suite bimodality (gate reliability)
Documented in the 2026-08-16 work doc and reconfirmed this session: `triple for`, `while up`, `while down` (and to a lesser degree `for break`) are per-process bimodal (~213 vs ~425 ms), most likely code-layout variance from randomized iteration order in the codegen prologue. With LOOP = 8 runs, suite averages are luck-dependent. Proposed action: see §2 recommendation (more runs / median gating), and longer-term investigate de-randomizing the prologue layout source.

### 4.9 Known open bug (properly filed, listed for completeness)
`bugs/json_recursion_limit/BUG_REPORT.md` — `json.parse` panics with R305 on nesting deeper than serde_json's default 128-frame recursion limit. Filed per the AGENTS.md bugs process; no fix attempted (out of audit scope).

---

## 5. Temporary / scratch artifacts

| Path | Nature | Proposed action |
|---|---|---|
| `node.js` (repo root) | Ad-hoc Node.js benchmark harness (6 runs, own averaging), hardcodes `B:\workspace\...` absolute paths; duplicates `run_xcx_only.py` with divergent methodology; misleading filename | delete (confirm first) |
| `test_nested_debug.xcx` (repo root) | 5-line JSON round-trip debug probe (Aug 10) | delete (confirm first) |
| `profile_json_ram.py` (repo root) | One-off RAM profiling script incl. pip-install of psutil; generates a temp `.xcx` at runtime | delete or move to a `tools/` with a purpose statement (confirm first) |
| `scratch/sieve_parts.xcx` | Sieve decomposition diagnostic; **explicitly referenced by the 2026-08-16 work doc** | keep (it is documentation-linked), or relocate with the doc updated |
| `xcx-benchmarks/Benchmarks/Main_Suite/c/err.txt` | Empty stray error-output file, untracked | delete (confirm first) |
| `xcx-benchmarks/fix_v_timer.py` | One-off migration script for the V-language benchmark timers; explains the modified `.v` files currently uncommitted in the benchmarks repo | keep until the `.v` changes are committed, then remove |
| `src/bin/` | Empty directory | remove or populate |

Also noted: the `xcx-benchmarks` repository carries uncommitted modifications (`csharp/*.csproj`, `v/*.v` files) and untracked files (`check_runtimes.*`, `fix_v_timer.py`, `results/`). No git operations were performed (git is locked); this is recorded because it affects merge hygiene.

---

## 6. Documentation impact

- This document supersedes nothing.
- **Correction to an existing work doc:** `2026-08-16_jit_getvar_receiver_inc_elision.md` states that `run_xcx_only.py` "compares against baseline.json". As detailed in §4.6, the runner's comparison is non-functional for the LOOP and FUNC suites (key mismatch) and uses stale MAIN numbers. That doc's own measured numbers remain valid; only the verification-tooling claim needs qualification.
- The dead-code inventory in §3 has no impact on `documentation/compiler/*` content; no doc describes the dormant trace compiler as active.

## 7. Ready-to-merge recommendation: **NO**

Blocking items, in order:

1. **`for step` regression** — 40.10 → ~54 ms median (+35%), fast mode absent in 24/24 processes, while all surrounding baseline numbers reproduce on this machine. Root cause unknown (not investigated in this read-only pass). This fails the "zero regression against baseline.json" bar on its own.
2. **Benchmark verification runner is not a gate** — `run_xcx_only.py` compares nothing for LOOP/FUNC and compares against stale MAIN numbers; any regression verdict drawn from it (including this session's `ALL OK`) is unreliable until §4.6 is fixed. Fixing this is cheap and should land before the next perf-sensitive change.
3. **`halt.alert` silence** (`module.rs:314`) is a user-visible divergence from the language documentation and should be resolved (restore print or change docs) before merge.

Non-blocking cleanup (dead code §3, DEBUG prints §4.1, silent map/set fallbacks §4.3, FIXME §4.4, test dumps §4.5, scratch files §5) can be scheduled separately; none of it affects correctness of the passing suite or the measured numbers, with the caveat that §4.3's silent halts degrade diagnosability of future failures.

Everything else is green: build clean (no new warnings), 199/199 runnable tests pass, main and func suites at parity or better, and the loop-suite deltas other than `for step`/`str concat` are demonstrably mode-mix artifacts with fast-mode runs beating baseline.
