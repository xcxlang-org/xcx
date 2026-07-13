# Array Element String Append Optimization (StrAppendElement)

## Description
Implementation of an efficient string concatenation optimization for array string elements. It optimizes the pattern:
```xcx
arr.update(i, arr.get(i) + expr);
```
By compiling it to a single in-place mutation instruction `StrAppendElement` instead of separate `Get`, `Add`, and `Update` method calls, which created multiple intermediate allocations and required full method dispatch inside the JIT and VM.

## Implementation Details
1. **OpCode**: Registered `OpCode::StrAppendElement`.
2. **VM Runtime**: Added `RuntimeOps::str_append_element` with COW logic to check `Arc::get_mut` on target string within write-locked `ArrayObj` elements buffer.
3. **Compiler**:
   - Added AST pattern detection in `src/compiler/compile_stmt.rs` (under `StmtKind::ExprStmt`).
   - Added HIR pattern detection in `src/hir/compile_hir.rs` (under `HirStmtKind::ExprStmt`).
4. **JIT & FFI**:
   - Added FFI helper `xcx_jit_str_append_element`.
   - Dynamic register analysis and Cranelift FFI call generation in `src/jit/emit_object.rs`.
5. **Verification**:
   - Added `test_array_element_string_append_cow` verifying safety of shared string elements within array.

## Benchmark Results
Benchmark `Benchmarks\str_elem_append\bench_elem_str.xcx` (100k iterations):

| Metric | Before Changes | After Changes |
| --- | --- | --- |
| **JIT Compile** | ~75.0 ms | **2.96 ms** |
| **No-JIT VM** | ~86.1 ms | **4.18 ms** |

Performance improvement: ~25x speedup for JIT, ~20x speedup for interpreter. Over 2.2x faster than Node.js (V8) at 6.59 ms.
