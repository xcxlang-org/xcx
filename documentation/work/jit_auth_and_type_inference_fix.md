# JIT Authentication and Type Inference Fix

## What was changed

Modified JIT parameter type inference logic in `src/jit/type_inference.rs`. Removed `OpCode::Equal` and `OpCode::NotEqual` from the heuristic that forces a parameter to be inferred as `TypeTag::Int`.

## Why

When the type-inference engine encountered a polymorphic comparison `==` or `!=` on two registers, it naively inferred the type of the compared parameter register to be `TypeTag::Int` (integer). 
In the CMS auth flow, a string token argument `token` is compared with an empty string: `token == ""`. The register was thus classified as `Int`.
During the register allocation spill phase (`spill_all()` in `codegen_ctx.rs`), the register's actual runtime string tag (`TAG_STR` = 4) was overwritten with hardcoded `TAG_INT` (1) on the VM stack.
When building the FFI parameters for database queries (e.g. `[token]`), the FFI helper read the parameter from the stack, saw it tag-union'd as `TAG_INT` (1) rather than `TAG_STR` (4), and passed it as a query integer parameter (the string pointer raw value) to rusqlite. This caused database token lookups to fail and return `401 Unauthorized`.

By removing `Equal`/`NotEqual` from the forced integer type-inference, polymorphic equality comparison does not coerce parameters to `Int`, preserving the runtime string type/tag.

## Modified Files

- [type_inference.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/type_inference.rs) (removed `Equal`/`NotEqual` from `infer_param_types` integer heuristic)
- [path.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/utils/path.rs) (removed temporary print debug statements)

## Benchmark Results

### Before Changes
- Loop(100m lcg) median: 130.34ms (regression from baseline 116.27ms)

### After Changes
- Loop(100m lcg) median: 88.59ms (improves on baseline)
- Fib(30): 12.73ms
- Sieve: 2.41ms
- JSON: 21.55ms

No performance regressions observed across standard benches.
