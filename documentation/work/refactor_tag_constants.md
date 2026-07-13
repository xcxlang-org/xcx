# Refactor Tag Constants to tag.rs

## Problem Statement
The runtime tag constants (`TAG_*`) were previously defined in `nan_boxing.rs` alongside legacy NaN-boxing float coding helpers. This was confusing because the compiler no longer uses a traditional NaN-boxing representation for tagged values, yet the file name and the comments suggested they were retained only for float serialization.

## Solution
1. Moved all `TAG_*` constants from `nan_boxing.rs` directly into `tag.rs` to keep them alongside the `Tag` enum definition.
2. Cleaned up the comments in `nan_boxing.rs` completely to keep it focused only on the necessary float bit pack/unpack helper routines.
3. Added backward-compatible re-exports in `nan_boxing.rs` and update exports in `mod.rs` to prevent breaking other files (e.g. JIT helpers, runtime ops, FFI definitions) that import constants via `nan_boxing::TAG_*` paths.

## Modified Files
*   `src/vm/value/tag.rs`: Added direct constant definitions.
*   `src/vm/value/nan_boxing.rs`: Removed old constants/comments. Re-exported constants from `tag.rs`.
*   `src/vm/value/mod.rs`: Updated re-exports to expose constants from the `tag` module rather than `nan_boxing`.
*   `src/vm/value/value.rs`: Updated imports to source `TAG_*` from `tag` and float helpers from `nan_boxing`.
*   `src/vm/value/heap_object.rs`: Updated imports.
*   `src/vm/value/ref_count.rs`: Updated imports.

## Verification
Tests were run via cargo:
```bash
cargo test --release
```
Result: 159 tests passed. No performance regression.
