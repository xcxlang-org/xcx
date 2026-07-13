# Table Method Dispatch and Register Allocation Fix

## What was changed
1. Extended HIR lowering and compiler in `src/hir/compile_expr.rs` to support code generation for previously unhandled expression types: `TableLiteral`, `DatabaseLiteral`, `DateLiteral`, and `Tuple`.
2. Fixed register alignment/allocation for optimized table query `where` in both HIR (`src/hir/compile_expr_special.rs`) and AST (`src/compiler/compile_expr/call.rs`) compilers. Receiver and argument registers are now laid out continuously starting at `base`, resolving run-time method dispatch errors.
3. Cleaned up unused compiler warnings:
   - Removed unused `tag` variable: `src/vm/utils/path.rs`.
   - Removed unreachable match block: `src/hir/compile_expr.rs`.

## Why
Previously, table literal and method expressions defaulted to unit values, causing runtime method selection issues (`receiver is not ptr/date`) when calling operations like `where`. Furthermore, the compiler was loading the predicate closure to index `next_local` while issuing `MethodCall` with `base` index without copying/aligning registers. This led to the VM extracting the closure argument from offset `base + 1` (where another local lived), causing execution halt.

## Files Modified
* `src/hir/compile_expr.rs`
* `src/hir/compile_expr_special.rs`
* `src/compiler/compile_expr/call.rs`
* `src/vm/utils/path.rs`

## Benchmark Results

### Table Operations (`benchmark_tables.xcx`)
* **`table.where` (1000 runs, 500 rows)**:
  - *Before*: `OpResult::Halt` (Process failed with 1 errors)
  - *After*: **77 ms** (Completed successfully)
* **`table.join` (100 runs, 500x500 rows)**:
  - *Before*: `OpResult::Halt`
  - *After*: **221 ms** (Completed successfully)

### Performance Suites
* **Loop(100m lcg)**:
  - *Before*: `130.34ms`
  - *After*: **86.69 ms**
* **inline_arith**:
  - *Before*: Halted due to regression
  - *After*: **0.43 ms**
