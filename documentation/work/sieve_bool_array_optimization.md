# Sieve BoolArray JIT Optimization

## Description
This change addresses the JIT compiler performance regression in the `sieve` benchmark. Sieve was previously taking ~3100 ms under JIT compared to ~190 ms in the interpreter. The bottleneck was caused by two main issues:
1. Missing `is_bool_array()` type query in the JIT compiler's `LoadConst` transition step within `src/jit/type_inference.rs`. This led to the `sieve` variable (which is a boolean array) being inferred as `TypeTag::Unknown`, completely disabling the array fast-path and forcing JIT to drop back to slow dynamic method dispatch via `xcx_jit_method_dispatch`.
2. Omitted `switch_to_block(fast_blk)` call in `src/jit/compiler_method.rs` for `GetIndex` opcode which caused corrupt SSA generation for the Cranelift compilation.

By addressing the type inference gap and properly inlining memory load/store operations (using offset 16 for `pointer` and offset 24 for `length` in Windows x64 ABI), the memory layout dereferences the `RwLock<BoolArrayObj>` and access elements directly with a 1-byte stride.

## Modified Files
* `src/jit/type_inference.rs`
* `src/jit/compiler_method.rs`
* `src/jit/emit_call.rs`

## Benchmark Results
* **Sieve (10M)**: 101.355 ms (Reduced from ~195 ms interpreter baseline / 3138 ms FFI-JIT path).
* **Other benchmarks & suite tests**: Cargo test release suite matches previous behavior (159 passed).
