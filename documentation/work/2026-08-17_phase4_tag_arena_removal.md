# Phase 4: TAG_ARENA value-format removal

**Date:** 2026-08-17
**Area:** VM value representation (`src/vm/value/`) and its consumers
**Plan:** `2026-08-17_remediation_plan.md` Phase 4, decision D2 (remove completely). Basis: re-verification §Disputed 2 — TAG_ARENA producers were dead in production (only a `#[cfg(test)]` call constructed an arena value), while ~44 consumer references lived in executed code.

## What was removed

The arena value format: a raw pointer with the inner type tag packed into `bits[48..55]`, used by no production code path. Removed, per file:

- `src/vm/value/tag.rs` — `TAG_ARENA` constant (value 15; other tag constants keep their explicit numbers, nothing renumbers).
- `src/vm/value/heap_object.rs` — `from_arena_string`, `from_arena_json`, `arena_inner_tag`, `arena_ptr`, the TAG_ARENA branches in `as_string`/`as_json`, and the TAG_ARENA arm in `to_string`.
- `src/vm/value/value.rs` — the `TAG_ARENA` alternative in every type predicate (`is_string` … `is_db`, now single tag comparisons), the `TAG_ARENA` arm in `tag()`, the arena early-returns in `inc_ref`/`dec_ref`, the arena branch in the string-append fast path, the arena branch in `as_str_borrow`, and the import.
- `src/vm/value/nan_boxing.rs`, `src/vm/value/mod.rs` — re-exports.
- `src/runtime/ffi_helpers/json_ffi.rs` — five arena-pointer selection branches (json bind/set/push/get-push and one more), now direct `unpack_ptr`.
- `src/vm/core/jit_helpers.rs` — `xcx_jit_json_parse` arena-string branch.
- `src/vm/utils/json.rs` — `value_to_json` arena branch.
- `src/vm/core/step/module.rs` — json-set arena branch.
- `src/jit/nan_ops.rs` — test `inc_ref_tag_set_matches_value_inc_ref` updated: the predicate is now `tag >= TAG_FIRST_PTR && tag != TAG_FUNC` (TAG_ARENA clause dropped, the `from_arena_json(null())` probe removed); the `emit_conditional_inc_ref` doc comment updated to match. `emit_is_ptr_tag` itself never referenced TAG_ARENA (it is a threshold comparison) and is unchanged.

Note: the JIT predicate (`emit_is_ptr_tag`) and the Rust-side `ref_count` never special-cased TAG_ARENA beyond what was removed, so no behavioral surface remains.

## Gate results

- `cargo build --release`: PASS, exit 0, zero new warnings (sole warning remains the pre-existing `left_rc`, addressed in Phase 5).
- `cargo test --release`: PASS — 199 passed, 0 failed, 1 ignored (unchanged).
- Performance gate: deferred (same reason as Batch C — background game load during this window; consolidated gate planned for Batches 3C+4+5 when the machine is idle). The predicates touched (`is_string` etc.) are on the hottest VM paths, so the consolidated gate pays extra attention to MAIN/FUNC means.

## Files modified

Listed above.

## Documentation impact

Completes the re-verification report's §C/§Disputed-2 cleanup: every TAG_ARENA reference is gone. `src/vm/core/arena.rs` was already deleted in Batch A. No language documentation mentions arena values; `documentation/language/*` unaffected.
