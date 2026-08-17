# Phase 5: technical-debt cleanup + consolidated benchmark gate (Phases 3C/4/5)

**Date:** 2026-08-17
**Area:** diagnostics, tests, workspace hygiene
**Plan:** `2026-08-17_remediation_plan.md` Phase 5 (items 1–7).

## Changes

1. **`DEBUG:` prints replaced with span-aware diagnostics** (production paths that halt):
   - `src/vm/core/dispatch.rs:37` — now `Method call on non-object receiver (tag, bits, kind) + current_span_info(ip)`.
   - `src/vm/core/dispatch.rs:61` — now `Unknown receiver tag … in method dispatch + span`.
   - `src/runtime/builtin/json/mod.rs` — fallback arm restored to the span-info message (was a `DEBUG:` print with the correct message commented out beneath it).
2. **Silenced halts restored** (D3 = restore):
   - `HaltAlert` (`src/vm/core/step/module.rs`) prints `XCX Alert: {msg}` to stderr again — behavior now matches `errors_halt.md` ("prints a warning to stderr; continues execution"). No doc change needed.
   - Map and Set method-fallback arms (`map/ops.rs`, `set/ops.rs`) print `Method … not supported for {Map,Set} + span` instead of halting silently.
3. **FIXME resolved** (`src/vm/core/step/module.rs` DatabaseInit): `ip` is now threaded from `execute_step` through `module::handle` into `handle_database_init`, so database-init error reports carry the real source position instead of `0`. Three handlers' unused `_ip` parameters renamed to `ip` (`map`, `set`, `json`) since their span messages now use them.
4. **Test bytecode dumps gated**: the `[TEST BYTECODE]`/`[TEST GLOBALS]`/`[DEBUG]` println blocks in `src/vm/core/tests.rs` now run only with `XCX_TEST_DUMP=1`; default test output is clean.
5. **SSRF test un-ignored by conversion**: the in-process `test_ssrf_protection_link_local` (ignored since it panics across JIT FFI frames and hard-aborts the release test binary) is replaced by `tests/ssrf_link_local.rs`, which spawns the real binary on `tests/ssrf_link_local_probe.xcx` and asserts non-zero exit + `SSRF` on stderr. Verified empirically first: the guard panics `halt.fatal: SSRF - Link-local addresses are forbidden` pre-connect (no network access occurs). Automated coverage of the link-local block is restored.
6. **Scratch artifacts deleted**: `node.js` (duplicate ad-hoc bench harness), `profile_json_ram.py` (one-off profiler), `xcx-benchmarks/Benchmarks/Main_Suite/c/err.txt` (empty stray). Left in place: `scratch/sieve_parts.xcx` (referenced by the 2026-08-16 work doc) and `test_nested_debug.xcx` at the repo root — the latter is an `.xcx` file and per AGENTS.md requires explicit maintainer confirmation before deletion; flagged, not deleted.
7. **`left_rc` warning fixed** (`src/vm/utils/table.rs`): the variable was declared twice in a row (the second shadows the first — that was the entire long-standing warning); duplicate line removed. The build is now warning-free.

## Gate results

- `cargo build --release`: PASS, exit 0, **zero warnings** (first fully clean build; the pre-existing `left_rc` warning is gone).
- `cargo test --release`: PASS — **200 passed, 0 failed, 0 ignored** (37 lib + 1 json_concurrency + 1 new ssrf_link_local + 161 xcx_runner; net +1 vs the previous 199+1-ignored state, with the ignored test now genuinely covered).
- **Consolidated benchmark gate for Phases 3C+4+5** (`results/gate_after_phase5.txt`, run after the background game load ended; the machine was verified quiet via a fast-mode `triple for` sample of 215 ms first): **ALL OK** — every metric at parity or better vs `baseline.json`: fib +1.9%/min −0.3%, lcg +1.5%/min −0.0%, sieve −1.8%, json −2.2%; FUNC geo mean −8.0%; loop suite all improved or parity, `for continue` −23.8%, TOTAL −11.9% vs baseline. Compared against the Batch B gate (last valid measurement): MAIN/FUNC within the noise band, every loop metric equal or better. The invalidated Batch C measurement (uniform +10–30% under `cs2` load) is thereby superseded by a clean measurement of the cumulative 3C+4+5 state. The only above-baseline entry is `for step` (+35.1%) — the known-open regression, Phase 2 skipped by maintainer decision.

## Files modified

`src/vm/core/dispatch.rs`, `src/runtime/builtin/json/mod.rs`, `src/runtime/builtin/map/ops.rs`, `src/runtime/builtin/set/ops.rs`, `src/vm/core/step/module.rs` (HaltAlert + ip threading), `src/vm/core/step/mod.rs` (pass `*ip` to `module::handle`), `src/vm/core/tests.rs` (dump gating + SSRF test replacement), `src/vm/utils/table.rs`, new `tests/ssrf_link_local.rs`, new `tests/ssrf_link_local_probe.xcx`, deleted `node.js`, `profile_json_ram.py`, `xcx-benchmarks/Benchmarks/Main_Suite/c/err.txt`.

## Documentation impact

- `2026-08-17_remediation_plan.md` Phase 5 items 1–7 complete; the plan's per-batch benchmark gate for 3C/4/5 was consolidated into this single valid measurement (reason documented in the Batch C work doc).
- `errors_halt.md` unchanged — `halt.alert` behavior now matches it again.
- The audit's §4.1–§4.5, §4.7 findings are resolved; §4.8 (bimodality) remains true and is now visible via the runner's `min` column; §4.9 (json recursion bug) remains open in `bugs/`.
