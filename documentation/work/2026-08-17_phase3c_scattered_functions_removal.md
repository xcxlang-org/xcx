# Phase 3 Batch C: scattered dead-function removal

**Date:** 2026-08-17
**Area:** whole-workspace dead code, final scattered batch
**Plan:** `2026-08-17_remediation_plan.md` Phase 3 Batch C. All 26 planned items from `2026-08-17_dead_code_reverification_pass.md` §D plus 10 additional zero-reference functions found by the full untruncated census (the original audit's census was truncated at 60 entries).

## What was removed

Planned items (re-verification report §D):

| Location | Functions |
|---|---|
| `src/vm/frame/frame_stack.rs` | `current_frame`, `current_frame_mut` |
| `src/sema/symbol/symbol_table.rs` | `lookup_symbol`, `_copy_globals` |
| `src/error/reporter.rs` | `has_errors` |
| `src/frontend/parser/precedence.rs` | `Precedence::for_token` (whole impl block; duplicate of live `Parser::current_precedence`) |
| `src/frontend/parser/pratt.rs` | `peek_precedence` |
| `src/jit/codegen_ctx.rs` | `def_local_int`, `def_local_bool`, `def_local_float`, `def_global_nanboxed`, `load_const_nanboxed`, `call_ffi_value_nanboxed` |
| `src/jit/abi.rs` | `decode_intcc` |
| `src/vm/object/json_val.rs` | `shallow_clone` |
| `src/runtime/builtin/io/input.rs` | `flush_stdin_device` (plus its windows/unix FFI declarations) |
| `src/vm/utils/table.rs` | `inject_json_into_table` |
| `src/runtime/builtin/db/ddl.rs` | `handle_database_member_access` |

Additional census finds removed in this batch (not in the original audit):

| Location | Functions |
|---|---|
| `src/vm/value/value.rs` | `xcx_eq`, `xcx_ne`, `xcx_lt`, `xcx_le`, `xcx_gt`, `xcx_ge` (thin PartialEq/PartialOrd wrappers, zero callers), `as_array_opt`, `typeof_str` |
| `src/vm/object/string_obj.rs` | `try_extend_bytes` (COW append helper, zero callers) |
| `src/jit/nan_ops.rs` | `unpack_float` |
| `src/jit/codegen_ctx.rs` | `use_local_nanboxed`, `use_global_nanboxed` |
| `src/jit/emit_arith.rs` | `emit_div_int` — orphaned by Batch B (its last callers were the dead trace compiler and dead `emit_mod_int`; the live method compiler uses `emit_poly_div_mod_fast_path`) |

Also removed earlier within Batch B's scope but enumerated here for the record: `emit_env_get`, `emit_env_args`, `emit_div_poly`, `emit_mod_poly`, `emit_random_int`, `emit_random_float_call`, `emit_loop_next_generic`, `emit_method_yield` (all were Batch-C-planned items that lost their final callers with the trace compiler and were removed together with the other orphaned emitters).

## Gate results

- `cargo build --release`: PASS, exit 0, zero new warnings (one transient unused import in `precedence.rs` after `for_token` removal, fixed within the batch; sole remaining warning is the pre-existing `left_rc`).
- `cargo test --release`: PASS — 199 passed, 0 failed, 1 ignored (unchanged).
- **Performance gate: INVALID MEASUREMENT, deferred.** The full benchmark run (`results/gate_after_batchC.txt`) shows a uniform +10–30% inflation on every metric including trivial ones (`json` +25.8%, `float arith` +9.6%) that have no plausible coupling to this batch — the removed functions were never executed. A process census during the run found the cause: a running game (`cs2`, ~10,000 CPU-seconds accumulated, active) plus Steam and browser load. A 12-run `triple for` distribution during the same window measured 261–542 ms versus 213–215 ms in every earlier idle window, confirming heavy background CPU contention. The batch-correctness gates (build + tests) are valid; the performance comparison for Batches 3C (and subsequently 4 and 5, which follow immediately) will be re-run as a single consolidated gate when the machine is idle, and compared against the Batch B gate (`results/gate_after_batchB.txt`), which was measured under normal conditions.

## Files modified

`src/vm/frame/frame_stack.rs`, `src/sema/symbol/symbol_table.rs`, `src/error/reporter.rs`, `src/frontend/parser/precedence.rs`, `src/frontend/parser/pratt.rs`, `src/jit/codegen_ctx.rs`, `src/jit/abi.rs`, `src/jit/nan_ops.rs`, `src/vm/object/json_val.rs`, `src/runtime/builtin/io/input.rs`, `src/vm/utils/table.rs`, `src/runtime/builtin/db/ddl.rs`, `src/vm/value/value.rs`, `src/vm/object/string_obj.rs`, `src/jit/emit_arith.rs`.

## Documentation impact

Completes the removal of every item in the re-verification report's §D table. The re-verification report's inventory is now fully removed from the codebase except the TAG_ARENA consumer support (Phase 4, next) and the two decision-gated clusters already handled in Batches A/B.
