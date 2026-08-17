# Phase 3 Batch A: dead-file removal (13 modules + closure vestiges)

**Date:** 2026-08-17
**Area:** whole-workspace dead code, first removal batch
**Plan:** `2026-08-17_remediation_plan.md` Phase 3, Batch A. All items were verified dead in `2026-08-17_dead_code_reverification_pass.md`; two additional files (`liveness.rs`, `visitor.rs`) and the `MakeClosure` opcode were verified dead during the pre-batch full census described below.

## What was removed

**Files deleted (13, ~1,480 lines):**

| File | Content | Dead because |
|---|---|---|
| `src/vm/trace/recording_helper.rs` | `record_op` bytecode→TraceOp recorder | zero callers; part of dormant trace JIT |
| `src/compiler/register_manager.rs` | `RegisterManager::compress_registers` | zero callers; superseded register allocation |
| `src/runtime/builtin/io/print.rs` | `print_val`, `run_cmd` | `>!` handled in `step/module.rs`; `!run` in `terminal_ffi.rs` |
| `src/vm/core/arena.rs` | `Arena`, `CURRENT_ARENA`, `with_arena`, `alloc_in_arena` | zero callers; arena value format addressed separately in Phase 4 |
| `src/compiler/patch.rs` | `patch_jump`, `patch_jump_to` | zero callers |
| `src/vm/stack/stack_guard.rs` | `StackGuard` | never constructed; recursion cap enforced elsewhere |
| `src/jit/loop_context.rs` | `LoopContext` | zero references |
| `src/vm/utils/set.rs` | `set_op` | set algebra via `SetUnion`/`SetIntersection` opcodes |
| `src/frontend/parser/parse_query.rs` | `parse_query_expr` (stub) | zero callers |
| `src/vm/object/closure_obj.rs` | `ClosureObj` | only consumers were dead `from_closure`/`as_closure` |
| `src/vm/frame/upvalue_cell.rs` | `UpvalueCell` (incl. Drop impl) | only consumer was dead `closure_obj.rs` |
| `src/compiler/liveness.rs` | `LivenessAnalysis` | only consumer was dead `register_manager.rs` (found in pre-batch census, missed by the original audit) |
| `src/frontend/ast/visitor.rs` | `AstVisitor` trait (181 lines) | zero implementors, zero users (found in pre-batch census, missed by the original audit) |

**Closure vestige chain removed alongside the files:** `from_closure`/`as_closure` (`heap_object.rs`), `TAG_CLOSURE` constant and its refcount arms (`ref_count.rs`), `is_closure`/`is_arena` predicates and the `TAG_CLOSURE` arm in `tag()` (`value.rs`), `TAG_CLOSURE` re-exports (`nan_boxing.rs`, `value/mod.rs`). Lambdas compile to plain function values (`from_function` + `LoadConst`), so the closure runtime representation was unreachable.

**Newly discovered and removed:** `OpCode::MakeClosure` — never emitted by any compiler path and never executed (it was the only opcode falling into the step-dispatch `_` arm); removing it made the `step/mod.rs` opcode match exhaustive, so the now-unreachable `_ => None` arm was deleted (the match now fails to compile if a future opcode is added without a handler — stricter than before). `collect_backedges` (`opcode.rs`) — only caller was dead `liveness.rs` (`calculate_has_loops` stays; it is widely used).

**Module tree / re-export cleanups:** `compiler/mod.rs` (patch, register_manager, liveness), `runtime/builtin/io/mod.rs` (mod + glob), `runtime/builtin/registry.rs` (second `io::print::*` glob found at compile time — glob re-exports are invisible to name-based census), `vm/core/mod.rs` (arena), `vm/stack/mod.rs` (mod + re-export), `jit/mod.rs` (loop_context), `vm/utils/mod.rs` (mod + glob), `frontend/parser/mod.rs` (parse_query), `vm/object/mod.rs` (mod + glob), `vm/frame/mod.rs` (mod + re-export), `vm/mod.rs` (`UpvalueCell` re-export), `vm/trace/mod.rs` (recording_helper), `frontend/ast/mod.rs` (visitor mod + `AstVisitor` re-export).

Note on `compiler/upvalue.rs`: it was briefly suspected dead but verified LIVE before any action — `collect_captures` is called from `compile_query.rs:13`, `compile_expr/control.rs:11`, `compile_expr/call.rs:348`. Not touched.

## Gate results

- `cargo build --release`: PASS, exit 0. Sole warning is the pre-existing `left_rc` (`table.rs:69`). Zero new warnings.
- `cargo test --release`: PASS — 37 + 1 + 161 = 199 passed, 0 failed, 1 ignored (the ignored SSRF test, unchanged). Same counts as before the batch.
- Benchmarks (`run_xcx_only.py --allow "for step"`, full config): saved to `xcx-benchmarks/results/gate_after_batchA.txt`. Before/after (same session, `gate_before_phase3.txt`): every MAIN/FUNC metric within ±1.3% of before; loop medians at or better than before (`triple for` 217.7→214.7, `for continue` 643.8→548.9 — that benchmark's documented bimodality, both modes below/around baseline). One metric, `while down`, showed 216.8→228.8 in the gated run; a 12-run distribution re-check (214.3–230.5, 7/12 runs at 214–215, median ≈216) shows the underlying distribution unchanged — the 24-run median had drawn a tail sample. No regression attributable to this batch. `for step` unchanged at ~54 ms (known-open, Phase 2 skipped).
- Against `baseline.json` (absolute, for reference): sieve +11.3% and str concat +4.2% remain above baseline — both are the documented same-binary machine-drift items from the Phase 1 snapshot (sieve measured 37.2 ms on 08-16 and 40.9–43.0 ms today with no code change affecting it), not effects of this batch.

## Files modified

Deletions per the table above; edits: `src/compiler/mod.rs`, `src/runtime/builtin/io/mod.rs`, `src/runtime/builtin/registry.rs`, `src/vm/core/mod.rs`, `src/vm/stack/mod.rs`, `src/jit/mod.rs`, `src/vm/utils/mod.rs`, `src/frontend/parser/mod.rs`, `src/vm/object/mod.rs`, `src/vm/frame/mod.rs`, `src/vm/mod.rs`, `src/vm/trace/mod.rs`, `src/frontend/ast/mod.rs`, `src/vm/value/{heap_object,ref_count,value,tag,nan_boxing,mod}.rs`, `src/vm/opcode/opcode.rs`, `src/jit/analysis.rs`, `src/jit/method_compiler.rs`, `src/vm/core/step/mod.rs`.

## Documentation impact

Updates the re-verification report's inventory: items #1–9 plus the closure chain are now removed from the codebase; `liveness.rs`, `visitor.rs`, and `MakeClosure` are additions to that inventory (both docs describe them as present — this doc supersedes on that point). No language-facing behavior changed; `documentation/language/*` unaffected.
