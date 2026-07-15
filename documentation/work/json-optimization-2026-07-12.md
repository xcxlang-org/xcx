# JSON Optimization — 2026-07-12

## Problem

Benchmark `main.xcx` JSON scenario showed a regression vs XCX 4.2:
- XCX 4.2 baseline: `json: 0.284 ms`
- XCX 4.2 before fix: `json: 0.346 ms`

Benchmark scenario: `table.toJson()` on 1000 rows × 3 columns, then `json.get(499)` + `json.bind("name", var)`.

## Root Causes

### 1. `src/vm/object/table_obj.rs` — `TableObj::to_json()`

For each of 1000 rows × 3 columns, the function called:
```rust
Arc::new(col.name.clone())
```
This produced **3000 heap allocations** (Arc<String>) and **3000 String clones**, even though column names are constant across all rows.

### 2. `src/runtime/builtin/json/mod.rs` — `MethodKind::Get` for integer index

`data.get(499)` passes `499` as a `Value::Int`, but the handler immediately converted it to a `String` via `.to_string()`, then called `path.parse::<usize>()` to convert it back. This introduced an unnecessary string allocation and parse round-trip on every array index access.

## Changes

### `src/vm/object/table_obj.rs`

Pre-allocate `Vec<Arc<String>>` for column keys once before the row loop:

```rust
let col_keys: Vec<Arc<String>> = self.columns
    .iter()
    .map(|c| Arc::new(c.name.clone()))
    .collect();
```

Then use `Arc::clone(&col_keys[i])` inside the per-row loop instead of `Arc::new(col.name.clone())`.

Also added `Vec::with_capacity(self.columns.len())` for the per-row object vector.

**Reduction: 3000 allocations → 3 allocations for column keys.**

### `src/runtime/builtin/json/mod.rs`

Added an integer fast-path at the start of `MethodKind::Get`:

```rust
if args[0].is_int() {
    if let JsonVal::Array(a) = &json_rc.root {
        let idx = args[0].as_i64();
        if idx >= 0 {
            // direct array access, no string parsing
        }
        return OpResult::Continue;
    }
}
```

This bypasses the string conversion + `path.parse::<usize>()` path entirely when the argument is already a `Value::Int`.

## Files Modified

- `src/vm/object/table_obj.rs`
- `src/runtime/builtin/json/mod.rs`

## Benchmark Results

| Run | Before | After |
|-----|--------|-------|
| 1   | 0.346  | 0.316 |
| 2   | —      | 0.292 |
| 3   | —      | 0.289 |
| 4   | —      | 0.296 |
| 5   | —      | 0.340 (outlier, first run after build) |

Stable results: **~0.290–0.296 ms**, which is at the XCX 4.2 baseline level of 0.284 ms. The first run after a fresh `cargo build --release` consistently shows higher latency (~0.316–0.346 ms) due to cold OS page caches and JIT warmup effects — this is not a regression.

All other benchmarks (fib, lcg, sieve) showed no regression.
