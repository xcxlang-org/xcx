# Phase 1: benchmark gate repair (run_xcx_only.py)

**Date:** 2026-08-17
**Area:** benchmark tooling (`xcx-benchmarks/run_xcx_only.py`)
**Plan:** `2026-08-17_remediation_plan.md`, Phase 1. Executed with maintainer authorization; Phase 2 (for-step investigation) skipped by maintainer decision.

## What changed

`run_xcx_only.py` was rewritten. The previous version carried an embedded, stale `BASELINE` dict whose keys did not match the parsed LOOP/FUNC metric names, so it silently compared nothing for two of three suites and compared MAIN against pre-getvar-elision numbers (audit §4.6). The new version:

1. Reads `../baseline.json` at runtime. The file holds three JSON objects (main, loop, func) separated by `//` comment lines, so it is decoded with `json.JSONDecoder.raw_decode` after stripping comment lines — the baseline file itself is untouched.
2. Carries an explicit parsed-key → baseline-key map per suite, verified against the actual labels the benchmarks print (`fib(30)`, `for step`, `array_alloc_1m`, …), captured by running each suite binary before writing the map.
3. Gating statistic per suite: MAIN/FUNC mean of 100 runs; LOOP median of 24 runs (was 8-run mean). The loop suite is per-process bimodal on this machine (documented in the 2026-08-16 JIT work doc), so an 8-run mean is a coin flip; the median absorbs the mode lottery. `--quick` keeps the old 3/10, 1/3, 3/10 iteration config.
4. Strict reporting: every entry prints raw ms, baseline, and delta; anything above baseline by more than +2% (parity band) is a REGRESSION; below −3% is improved. Exit code 1 on any regression. TOTAL (loop) and GEO MEAN (func) are printed as derived, non-gating rows.
5. `--allow KEY...` marks known-open regressions (reported as KNOWN REGRESSION, excluded from the exit code) so the runner can gate subsequent work while an accepted regression stays open.

## Verification

Quick sanity run confirmed parsing, mapping, `--allow`, and exit-code behavior. Full run (official config) captured as the BEFORE snapshot for Phases 3–5: `xcx-benchmarks/results/gate_before_phase3.txt`.

Snapshot results vs baseline.json: MAIN fib +1.3% / lcg +1.1% / json −0.4% parity, **sieve +17.0% above baseline**; FUNC all improved (geo mean −8.8%); LOOP medians: triple for −16.5%, while up −9.1%, while down −3.9%, array −3.1%, set −3.4%, break −2.3%, float arith −2.8%, **for continue +12.1%**, **str concat +4.8%**, **for step +38.2% (known-open, allowed)**.

Machine-drift observation that changes how later phases must be gated: with the SAME binary, sieve measured 37.2 ms on 2026-08-16 and 43.0 ms on 2026-08-17 (+16% day-to-day), and for continue moved 553 → 644 ms. Absolute baseline.json comparison on this machine therefore carries variance well beyond the ±2% parity band for some benchmarks. Consequently, the per-batch gate for Phases 3–5 is **before/after comparison within the same session** (no metric may regress beyond the parity band vs its BEFORE value), with baseline.json remaining the official absolute target. For step stays a known-open regression per the maintainer's Phase 2 skip; it runs under `--allow`.

## Files modified

- `xcx-benchmarks/run_xcx_only.py` — rewritten as described.
- `xcx-benchmarks/results/gate_before_phase3.txt` — new, BEFORE snapshot.

## Documentation impact

- Supersedes the verification-tooling claim in `2026-08-16_jit_getvar_receiver_inc_elision.md` §"Verification tooling": as of this change the runner genuinely compares against baseline.json. The 2026-08-16 audit §4.6 finding is now resolved.
- `2026-08-17_remediation_plan.md` Phase 0 is annotated: D1/D2/D3 resolved; Phase 2 skipped by maintainer decision on 2026-08-17, so the for-step regression remains open and is carried via `--allow "for step"` in all subsequent gates.
