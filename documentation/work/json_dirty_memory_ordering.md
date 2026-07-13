# Version-Based JSON Caching & Concurrency Safety

## What was changed

Replaced the `dirty` (`AtomicBool`) flag on `JsonObj` with an atomic versioning scheme using two counters:
1. `version` (`AtomicU64`): Track mutations to the JSON object.
2. `cached_version` (`AtomicU64`): Track the version represented by the current string cache in `cached_str`.

All reads / cache hit steps compare `version == cached_version` lock-free using `Ordering::Acquire`. 
On mutations, `version` is incremented via `fetch_add(1, Ordering::Release)`.
During serialization, the reader fetches the starting version `ver = version.load(Acquire)`, then serializes the root object. When updating the cache, it acquires the `cached_str` Mutex, and verifies `version.load(Acquire) == ver`. If unchanged, the cache is populated and `cached_version` is set to `ver` under `Ordering::Release`. If it changed, the serialized cache is safely discarded.

## Why

A standard boolean `dirty` flag (even with Acquire/Release semantics) is vulnerable to a silent cache invalidation race:
1. Reader starts serializing the current state `v1`.
2. Writer mutates the object to `v2` and sets `dirty = true`.
3. Reader finishes serializing `v1` to `s1`, takes `cached_str` lock, sets it to `s1`, and stores `dirty = false`.
4. The cache is now stale (`s1` represents `v1`, but `root` is `v2`), yet `dirty` is marked `false`, causing all future reads to return stale data.

The double version counter scheme eliminates this race entirely without adding any mutexes or locks to the JIT/VM fast-path checks.

## Modified Files

- [json_obj.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/object/json_obj.rs) (definition of `JsonObj` updated with `version` and `cached_version`)
- [heap_object.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/value/heap_object.rs) (synchronized serialization caching checks)
- [module.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/core/step/module.rs) (increments version)
- [json_ffi.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/runtime/ffi_helpers/json_ffi.rs) (FFI helpers updated with version checks)
- [mod.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/runtime/builtin/json/mod.rs) (updated JSON builtin methods)
- [json_concurrency.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/tests/json_concurrency.rs) (New integration test performing concurrent stress test of JIT/VM JSON operations)

## Verification Results

- Verified that **no locks or mutexes** were added to the JIT/VM fast check path. Checking `version == cached_version` is completely lock-free.
- Ran standard test suite `cargo test --release`:
  - **Status**: PASSED (159 passed).
- Ran the new integration test target `cargo test --release --test json_concurrency`:
  - **Status**: PASSED (1 passed).
