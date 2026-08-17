# Phase 3 Batch B: trace-JIT and fiber-JIT cluster removal

**Date:** 2026-08-17
**Area:** JIT/VM — dormant tracing subsystem and unreachable fiber-segment JIT
**Plan:** `2026-08-17_remediation_plan.md` Phase 3 Batch B + decision D1 (remove). Basis: `2026-08-17_dead_code_reverification_pass.md` §Disputed 1 (Hotspot non-functional; `compile_fiber_segment` dynamically unreachable because `Hotspot::resize` is never called, so `tick` can never fire).

## What was removed

**Trace recorder subsystem** — `src/vm/trace/` deleted entirely (5 files: `trace.rs`, `trace_op.rs`, `recorder.rs`, `hotspot.rs`, `recording_helper.rs` was already gone in Batch A; `mod.rs`): `Trace`, `TraceOp` (131-line enum), `Recorder` (constructed but `start()` never called — `is_recording` permanently false), `Hotspot` (non-functional: `resize`/`reset`/`blacklist` had zero callers, `tick`'s only call site could never fire because `counts` was never grown).

**Trace compiler** — `JIT::compile` (`src/jit/jit.rs`, ~430 lines: the entire trace-IR-to-native compiler). `JIT` keeps only `new()`; struct fields `ptr_type` and `symbols` were write-only after this (compile_method builds its own registry per compilation and re-derives ptr_type from the module) and were removed. The four `analyze_trace_*` functions in `src/jit/analysis.rs` were called only by the dead compile entry and are gone; the chunk-based analyzers (`analyze_chunk_locals`, `analyze_chunk_globals`, …) are live and stay.

**Fiber-segment JIT** — `src/jit/compiler_fiber.rs` (372 lines), the `hotspot.tick` + `compile_fiber_segment` block in `src/runtime/builtin/fiber/ops.rs`, the segment-cache lookup block (`chunk.jit_segments.read()` — the map had no writers left, so the lookup always missed), and the `jit_segments` field of `Chunk` (`src/vm/opcode/chunk.rs`). Fibers now resume directly into the interpreter; the removed path never executed in practice (the fallback was always taken), so runtime behavior is unchanged. The misleading comment ("fibers use compile_fiber_segment, not the trace recorder") is gone with the code.

**Plumbing** — `Executor` fields `hotspot`/`recorder`/`trace_cache` (`executor.rs`), `VM::traces` (`vm.rs`), the `#[cfg(feature = "jit")] trace` module and `Trace`/`TraceOp` re-exports (`vm/mod.rs`), REPL `!jit` rows "Loop Traces"/"JIT-Compiled" (always displayed 0; removed; "Hotspot Limit" renamed to "Warmup Limit" to match what it actually shows — `jit_threshold`, which still drives `check_jit_warmup`).

**Orphaned emitters** (lost their last callers with the trace compiler and fiber compiler; verified by post-removal census, 15 functions): `emit_guard_int/float/bool`, `emit_loop_exit`, `emit_loop_next_generic`, `emit_loop_next_int`, `emit_inc_local_loop_next`, `emit_inc_var_loop_next`, `emit_array_loop_next`, `emit_table_iter`, `emit_table_size`, `emit_yield`, `emit_yield_void`, `emit_method_yield`, `emit_return_fiber` (all `emit_control.rs` — file rewritten keeping the 14 live emitters), plus `emit_mod_int` (`emit_arith.rs`). The `_opcode`-suffixed loop emitters used by `compile_method` are live and kept.

## Gate results

- `cargo build --release`: PASS, exit 0, zero new warnings (sole warning remains the pre-existing `left_rc`). One transient `unused_mut` and one `unused import` surfaced during the batch and were fixed within it.
- `cargo test --release`: PASS — 199 passed, 0 failed, 1 ignored (same as baseline state).
- Benchmarks (`results/gate_after_batchB.txt`): vs the Batch A gate, every metric equal or better — fib 11.05→10.83, lcg 106.79→104.82, sieve 40.93→34.66 (now beats baseline.json's 36.76), `while down` 228.8→212.7 (confirming Batch A's 228.8 median was a tail draw), `for break` 424.8→411.3, `str concat` 2.752→2.674 (parity vs baseline). `for continue` gated at 614.9 (+12% vs Batch A's 548.9); a 12-run distribution shows it is strongly bimodal (fast ~423 ms, slow ~632 ms, ≈50/50 per process) — its baseline.json value 574.54 is itself a mixed-mode average, the fast mode beats baseline by 26%, and neither mode shifted across A→B. Not attributable to this batch; recorded as an additional bimodal benchmark (extends the list in the 2026-08-16 JIT work doc, which covered `triple for`/`while up`/`while down`).
- Runner improvement made during this gate: `run_xcx_only.py` now also prints each metric's `min` (best mode) so bimodal benchmarks are visible in gate output without manual distribution runs.

## Files modified

Deleted: `src/vm/trace/` (5 files), `src/jit/compiler_fiber.rs`. Edited: `src/jit/jit.rs` (rewritten to struct+new), `src/jit/analysis.rs`, `src/jit/emit_control.rs` (rewritten), `src/jit/emit_arith.rs`, `src/jit/emit_misc.rs` (import), `src/vm/core/executor.rs`, `src/vm/core/vm.rs`, `src/vm/opcode/chunk.rs`, `src/vm/mod.rs`, `src/jit/mod.rs`, `src/runtime/builtin/fiber/ops.rs`, `src/repl/repl.rs`, `xcx-benchmarks/run_xcx_only.py`.

## Documentation impact

Supersedes the re-verification report's §Disputed-1 recommendation state: the cluster was removed per D1 rather than repaired. The 2026-08-16 JIT work doc's "Verification tooling" note and bimodality list are extended by this doc (for-continue added). REPL `!jit` output changed (two always-zero rows removed, one label renamed) — no doc describes REPL output rows, `CLI_REFERENCE.md` unaffected (checked: it documents CLI flags, not REPL diagnostics).
