# Faza 11 — Nested Fiber Method Call Overwrite Fix

## What was changed

Modified the HIR compiler's `MethodCall` expression compilation logic to ensure the receiver expression of a method call is copied to a new temporary register before evaluating arguments and issuing the virtual machine method instruction.

## Why

In the HIR compiler, variable registers are returned directly to optimize register utilization. However, in a method call expression (such as `sub.next()`), if the receiver (`sub`) was represented by a local variable register slot (e.g. register 1), compilation was directly set to use `dst = base`, causing the output value of the method call to overwrite the local variable register. On subsequent activations (such as subsequent yields/resumptions of a nested fiber), using `.next()` on the corrupted register caused the executor to panic or halt with Type Errors due to trying to use integer values instead of an active Fiber structure.

Copying the receiver to a temporary register (`compiler.next_local`) preserves the original local variable's state across yield/resumptions, restoring full correctness for nested fibers.

## Files Modified

- [compile_expr.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/hir/compile_expr.rs)

## Benchmark Results

### Before Changes
- cargo test failed on `fiber_nested` and `ult_04_fiber_nested`.

### After Changes
- `cargo test --release`: 159 passed, 0 failed.
- `benchmarks_runner.py` results:
  - Loop(100m lcg): 129.72 ms (baseline: 116.27 ms)
  - Fib(30): 12.52 ms (baseline: 12.87 ms)
  - Sieve: 2.30 ms (baseline: 2.29 ms)
  - JSON: 21.08 ms (baseline: 21.46 ms)
