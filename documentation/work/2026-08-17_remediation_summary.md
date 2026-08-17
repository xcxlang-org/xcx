# Remediation summary — dead code, technical debt, benchmark gate

**Date:** 2026-08-17
**Type:** Consolidated record of the remediation executed from `2026-08-17_remediation_plan.md` (Phases 1, 3, 4, 5, 6; Phase 2 skipped by maintainer decision). Per-phase details live in the phase work docs; this file is the index and the final measured state.

## What was executed

| Phase | Record | Scope |
|---|---|---|
| 1 | `2026-08-17_phase1_benchmark_gate_repair.md` | `run_xcx_only.py` rewritten: baseline.json read at runtime, explicit key mapping, LOOP median-of-24, ±2% parity band, `--allow` for known-open regressions, exit-code gating, `min` column for bimodal visibility |
| 3A | `2026-08-17_phase3a_dead_file_removal.md` | 13 dead files (~1,480 lines) + closure vestige chain (`ClosureObj`, `UpvalueCell`, `TAG_CLOSURE`, `MakeClosure` opcode, `is_closure`/`is_arena`) + `collect_backedges` |
| 3B | `2026-08-17_phase3b_tracejit_fiberjit_removal.md` | Entire `vm/trace/` subsystem, `JIT::compile` trace compiler, fiber-segment JIT (`compiler_fiber.rs`, `jit_segments`), Executor/VM trace plumbing, REPL trace rows, 16 orphaned emitters |
| 3C | `2026-08-17_phase3c_scattered_functions_removal.md` | 36 scattered dead functions (26 planned + 10 census finds incl. `xcx_eq..ge`, `use_*_nanboxed`, `emit_div_int`) |
| 4 | `2026-08-17_phase4_tag_arena_removal.md` | `TAG_ARENA` value format removed across 9 files; nan_ops predicate test updated |
| 5 | `2026-08-17_phase5_tech_debt_cleanup.md` | DEBUG prints → span-aware diagnostics, silenced map/set/`halt.alert` halts restored, DatabaseInit FIXME resolved (real `ip`), test dumps behind `XCX_TEST_DUMP`, SSRF coverage restored as spawned-process test, scratch files removed, `left_rc` duplicate-line warning fixed |
| 6 | this file + doc corrections | `documentation/compiler` updated: trace-pipeline sections rewritten to the method-JIT reality (jit_core, jit_codegen, jit README, vm_executor, vm_opcode, vm_value, vm_objects, runtime_core, runtime_collections, repl, compiler README, ast.md, compiler_expr.md). Follow-up (maintainer request): root `README.md` JIT section rewritten (pipeline row + JIT paragraph now describe per-function warmup compilation, not trace recording) and `CLI_REFERENCE.md` `--threshold` / REPL `!jit` descriptions corrected; `documentation/README.md` JIT index line updated. Historical changelogs under `documentation/changelogs/` were deliberately left untouched — they document the state at their release time. |

Additional late find folded into Phase 6 (verified dead in-code, same category as the rest): `Chunk::has_loops` field and `calculate_has_loops` — after the trace recorder's removal the field had zero readers; removed with all 10 construction sites updated.

## Final state (all verified 2026-08-17, end of execution)

- `cargo build --release`: PASS, **zero warnings** (the historic `left_rc` warning was a duplicated binding line, now fixed).
- `cargo test --release`: PASS — **200 passed, 0 failed, 0 ignored** (was 199 + 1 ignored at session start; the SSRF link-local test is now real coverage via `tests/ssrf_link_local.rs`).
- Benchmarks (final full gate, `xcx-benchmarks/results/gate_final.txt`): every metric at parity or better vs `baseline.json` — sieve −1.7%, FUNC geo mean −7.9%, loop suite improved across the board (`for continue` fast mode −25% at min), TOTAL −1.6%. Two entries above baseline by design/circumstance: `for step` +35.4% (known-open, Phase 2 skipped — see below) and `for continue` median +8.2% on that run, which is its documented ≈50/50 bimodality (min column shows −25.2%); a distribution re-check confirmed both modes unchanged.
- Net code reduction: ~2,900 lines of verified-dead code removed; no functional surface changed except the deliberate restorations (halt.alert output, map/set fallback diagnostics, span info on three dispatch error paths, DatabaseInit error positions).

## Open items (explicitly not done)

1. **`for step` regression (+35% vs baseline.json)** — Phase 2 skipped by maintainer decision. Carried in gates via `--allow "for step"`. Root cause unknown; start point for any future investigation: decompose the benchmark (pattern: `scratch/sieve_parts.xcx`), compare `@step` lowering vs plain-range loops, with and without `--no-jit`.
2. **`test_nested_debug.xcx`** at the repo root — deletion requires explicit maintainer confirmation (`.xcx` file; not deleted).
3. **`bugs/json_recursion_limit`** — pre-existing open bug (serde_json 128-depth limit), untouched, per the bugs-process rule.
4. **Machine-drift/bimodality of the benchmark host** — sieve drifted 37→43→36 ms across days on identical binaries, and four loop benchmarks are per-process bimodal. The runner now surfaces `min`, but a stricter future gate could run bimodal benchmarks with more samples or mode-aware statistics.

## Git

No git operations were performed (locked per AGENTS.md). All changes are uncommitted in the working tree; the xcx-benchmarks nested repo additionally carries its own pre-existing uncommitted state. Commit points, if desired: one commit per phase record is a natural split.
