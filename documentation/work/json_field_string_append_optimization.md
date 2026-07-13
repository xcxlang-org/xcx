# JSON Field String Append Optimization (StrAppendMember)

## Description
Implementation of an efficient string concatenation optimization for JSON object fields (`JsonVal::String`). It optimizes the pattern:
```xcx
data.set("log", data.get("log") + "a");
```
By compiling it to a single in-place mutation instruction `StrAppendMember` instead of separate `Get`, `Add`, and `Set` method calls, which created multiple intermediate allocations.

## Implementation Details
1. **OpCode**: Registered `OpCode::StrAppendMember`.
2. **VM Runtime**: Added `RuntimeOps::str_append_member` with COW logic to check `Arc::get_mut` on target string within write-locked JSON object.
3. **Compiler**:
   - Added AST pattern detection in `src/compiler/compile_stmt.rs` (under `StmtKind::ExprStmt`).
   - Added HIR pattern detection in `src/hir/compile_hir.rs` (under `HirStmtKind::ExprStmt`).
4. **JIT & FFI**:
   - Added FFI helper `xcx_jit_str_append_member`.
   - Dynamic register analysis and Cranelift FFI call generation in `src/jit/emit_object.rs`.
5. **Verification**:
   - Added `test_json_field_string_append_cow` verifying safety of shared JSON objects.

## Benchmark Results
Benchmark `Benchmarks\str_field_append\bench_field_str.xcx` (100k iterations):

| Metric | Before Changes | After Changes |
| --- | --- | --- |
| **JIT Compile** | ~3737 ms | **3.20 ms** |
| **No-JIT VM** | ~1681 ms | **6.04 ms** |

Performance improvement: ~1100x speedup for JIT, ~270x speedup for interpreter.
