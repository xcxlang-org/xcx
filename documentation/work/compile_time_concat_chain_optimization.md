# Compile-Time Concat Chain Optimization (Option A)

## Problem Statement
The general string concatenation loop block pattern `res = res + "a" + "b"` still yielded suboptimal JIT results (~3604 ms) or interpreter times without chain-level optimization because of nested addition expression evaluation order, necessitating intermediate memory copies and FFI helper overhead.

## Proposed Solution
Implement compile-time concatenation chain flattening. The compiler recursively rolls up addition operations (`+`) whose target variable is the leftmost operand. It extracts a list of right-hand arguments, verifies that the target variable is never referenced on the right to preserve evaluation order safety, and generates a series of in-place append operations:
*   In AST compiler (`compile_stmt.rs`): Emits sequential `OpCode::StrAppendLocal` or `OpCode::StrAppendVar`.
*   In HIR compiler (`src/hir/compile_hir.rs`): Lowers assignments to sequential appends.

## Modified Files
*   `src/compiler/compile_stmt.rs`: Added AST concat chain collection (`collect_concat_chain`), recursive identifier check (`expr_contains_identifier`), and emitted sequential append/var opcodes under `StmtKind::Assign`.
*   `src/hir/compile_hir.rs`: Added HIR concat chain collection (`collect_hir_concat_local`, `collect_hir_concat_global`), reference safety checks (`hir_expr_contains_local`, `hir_expr_contains_global`), and lowered assignments to sequential nested appends.

## Outdated Documentation / Performance Updates
The document [general_string_append_cow_optimization.md](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/documentation/work/general_string_append_cow_optimization.md) is now partially outdated regarding `bench_general_str.xcx` performance. While runtime COW check is still used for non-destructively flat concatenations, compile-time chain flattening has superseded it for nested additions.

## Benchmark Results
Target: `bench_general_str.xcx` (100,000 iterations):
*   **JIT (SajaJIT) Before (No Option A)**: 3,604.117 ms
*   **JIT (SajaJIT) After (Option A)**: 5.494 ms (650x speedup)
*   **Interpreter Before (No Option A)**: 332.407 ms
*   **Interpreter After (Option A)**: 7.104 ms (45x speedup)
