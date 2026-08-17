# Remediation plan — dead code, technical debt, performance regression

**Date:** 2026-08-17
**Type:** Plan. No code changed yet. Derived from `2026-08-16_codebase_audit_dead_code_debt_regression.md` (audit) and `2026-08-17_dead_code_reverification_pass.md` (re-verification: 46 claims confirmed, 2 disputed, 1 uncertain).
**Status:** EXECUTED 2026-08-17 (Phases 1, 3, 4, 5, 6; Phase 2 skipped by maintainer decision — the `for step` regression remains open and is carried in gates via `--allow "for step"`). All Phase 0 decisions applied (D1 = remove, D2 = remove, D3 = restore print). Final state: build warning-free, 200/0/0 tests, benchmark gate ALL OK vs baseline.json except the known-open `for step`. Per-phase records: `2026-08-17_phase1_benchmark_gate_repair.md`, `..._phase3a_dead_file_removal.md`, `..._phase3b_tracejit_fiberjit_removal.md`, `..._phase3c_scattered_functions_removal.md`, `..._phase4_tag_arena_removal.md`, `..._phase5_tech_debt_cleanup.md`, summary: `2026-08-17_remediation_summary.md`.
**Execution rule (every phase, per AGENTS.md):** `cargo build --release` with zero new warnings, `cargo test --release` 199/199, full benchmark suite vs `baseline.json` — after every batch. A work doc in `documentation/work` after every implemented change. Git stays locked; the maintainer commits per batch.

## Phase 0 — Decision gates (RESOLVED 2026-08-17)

| # | Decision | Resolution |
|---|---|---|
| D1 | Fiber-JIT cluster (`Hotspot`, `compile_fiber_segment`, `src/jit/compiler_fiber.rs` — 372 lines; dynamically unreachable because `Hotspot::resize` is never called) | **REMOVE.** Fibers stay interpreted, matching all shipped behavior. If fiber JIT is wanted later, it is reintroduced deliberately with a test asserting engagement and benchmark evidence. |
| D2 | `TAG_ARENA` value format (8 files, ~44 references, plus `from_arena_json` test dependency in `nan_ops.rs:252`) | **REMOVE COMPLETELY**, updating the nan_ops predicate tests in the same change (Phase 4). |
| D3 | `halt.alert` silence (`module.rs:314`) | **RESTORE the stderr print** — behavior returns to conformance with `errors_halt.md`; no doc change needed. |
| D4 | `for step` regression handling | Investigate and fix before any removal batch lands (Phase 2 — merge blocker, so all later batches are measured against a green gate). |

## Phase 1 — Repair the benchmark gate (before touching any code)

`xcx-benchmarks/run_xcx_only.py` currently compares nothing for LOOP/FUNC (key mismatch) and uses stale MAIN numbers; its `ALL OK` is not evidence.

1. Read `baseline.json` at runtime instead of embedding numbers.
2. Map parsed metric names to baseline keys explicitly (`for step`→STEP, `triple for`→NESTED, `array_alloc_1m`→ARRAY ALLOC, …).
3. Replace the ±15% tolerance with strict reporting: print raw delta for every entry and flag anything above baseline; use ±2% as the "parity" noise band for MAIN/FUNC.
4. Loop suite: 8 runs → 24 runs, gate on the median (documented bimodality makes 8-run averages a coin flip). Keep `--quick` for iteration.
5. Verify the repaired runner reproduces this week's known state (for step flagged as regression, everything else ok).

## Phase 2 — `for step` regression (blocker)

Current: median ~54 ms vs baseline 40.10 ms; no fast mode in 24/24 processes, while neighboring baselines reproduce within ~2%.

1. Build a decomposition script under `scratch/` (pattern: `sieve_parts.xcx`) splitting the benchmark into step-loop lowering, inner-body cost, and loop-exit checks; run with and without `--no-jit` to separate interpreter from JIT cost.
2. Inspect `@step` lowering (frontend → HIR → bytecode) and the JIT emission path for stepped loops in `compiler_method.rs` / `emit_control.rs`; compare against the plain-range loop path that beats baseline.
3. Fix the identified cause in the compiler; no benchmark or `.xcx` file is touched.
4. Gate: full suite via the repaired runner; `for step` back to ≤40.10 ms (or, if the 40.10 figure proves unattainable on this hardware, the maintainer re-baselines explicitly and the new baseline is documented).

## Phase 3 — Dead-code removal, confirmed set (three batches)

Each batch: build (zero new warnings), tests (199/199), full benchmark suite, work doc. Removal includes the `mod` declarations and re-exports listed in the re-verification report.

- **Batch A — standalone dead files (11 files, ~1,050 lines):** `recording_helper.rs`, `register_manager.rs`, `print.rs` (+ `io/mod.rs:1,4`), `arena.rs`, `patch.rs`, `stack_guard.rs` (+ `stack/mod.rs:2,5`), `loop_context.rs`, `set.rs`, `parse_query.rs` (+ `parser/mod.rs:17`), `closure_obj.rs` (+ `object/mod.rs:6`), `upvalue_cell.rs` (+ `frame/mod.rs:9`, `vm/mod.rs:19`), plus dead singletons `is_closure`, `is_arena`.
- **Batch B — trace-JIT plumbing (~1,250 lines):** `JIT::compile` body in `jit.rs` (keep `JIT::new` and the method-compiler impls), the four `analyze_trace_*` functions, `Recorder` (`executor.rs:16,53`), `Trace`/`TraceOp` + `trace.rs`/`trace_op.rs`/`recorder.rs`, `VM::traces` (`vm.rs:29,43`; drop the "Loop Traces / JIT-Compiled" rows in `repl.rs:122–145`), `trace_cache` (`executor.rs:17,54`, `fiber/ops.rs:104,109`), `on_guard_failure`/`guard_failures`. If D1 = remove: also `Hotspot` (struct, `executor.rs:15,31–32,52`, `fiber/ops.rs:86–95,103,108`), `compile_fiber_segment` call site, and `compiler_fiber.rs`.
- **Batch C — scattered functions (26, per the re-verification table):** the full list from `2026-08-17_dead_code_reverification_pass.md` §D, each with its live replacement already identified there.

## Phase 4 — TAG_ARENA removal (only if D2 = remove)

Single self-contained change: remove `TAG_ARENA`, `arena_inner_tag`, `arena_ptr`, `from_arena_string`, `from_arena_json`, all arena branches in `value.rs` predicates/`tag()`/dec_ref skips/append/`as_str`, the arena paths in `json_ffi.rs` (6 sites), `jit_helpers.rs:615–616`, `utils/json.rs:70–71`, `step/module.rs:136–137`, `heap_object.rs` accessors; update the `nan_ops.rs` predicate tests (lines ~209–260) to construct values without the arena format. Full gate afterwards.

## Phase 5 — Technical debt

1. Replace the three `DEBUG:` prints with R-coded messages including span info: `dispatch.rs:37,61`, `json/mod.rs:324` (correct message already exists commented out at :325).
2. Restore silenced diagnostics: map/set fallback arms (`map/ops.rs:199`, `set/ops.rs:117`), `HaltAlert` (`module.rs:314`, per D3).
3. Resolve the FIXME at `module.rs:38`: thread the real `ip` into database-init error reporting.
4. Gate the test bytecode dumps (`vm/core/tests.rs:625,741–759,904–915`) behind an env var (`XCX_TEST_DUMP=1`), default silent.
5. Un-ignore the SSRF test (`tests.rs:236`) by converting it to a spawned-process test, restoring automated coverage of the link-local block.
6. Scratch-file cleanup, each file confirmed individually before deletion: `node.js`, `test_nested_debug.xcx`, `profile_json_ram.py`, `xcx-benchmarks/Benchmarks/Main_Suite/c/err.txt`; keep `scratch/sieve_parts.xcx` (documentation-linked).
7. Optional, with consent: rename `left_rc` → `_left_rc` (`table.rs:69`) so the build is warning-free — sole remaining warning after Phases 3–4.

## Phase 6 — Documentation

- Work doc per implemented batch (mandatory, per AGENTS.md rule 4).
- Update `documentation/language/errors_halt.md` only if D3 chooses (b).
- Update `documentation/compiler/jit/*` if Batch B changes any described structure; explicitly mark the trace-compiler description as removed.
- The two corrected statements in the 2026-08-16 audit are already flagged in the re-verification doc; no further retro-editing.

## Sequencing rationale

Gate repair (1) before regression fix (2) so the fix is measured by a trustworthy tool. Regression fix before removals (3) so every removal batch is gated on a green baseline rather than a known-red one. TAG_ARENA (4) after the main removals to keep its cross-cutting diff isolated. Debt cleanup (5) last so it lands on a stable base. Phase 0 decisions D1/D2 gate Batches B/4; D3 gates item 5.2; nothing in Phases 1–2 requires a decision beyond "go".
