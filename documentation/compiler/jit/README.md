# JIT Compilation Engine (`src/jit/`)

This directory documents the integration of the JIT compilation engine with the Cranelift backend, quiet-NaN boxing, register pre-analyses, and native instruction emission rules.

## Directory Index

- **[jit_core.md](jit_core.md):** Details the Cranelift module integration, active Trace compilation pipeline, JIT FFI parameter layout, and nested local/direct wrapper-inner function ABIs.
- **[jit_codegen.md](jit_codegen.md):** Describes the `CodegenCtx` SSA register mapping, register pre-loading/spilling, static pointer analysis (featuring a 256-bit bitmask), bytecode type inference, NaN-boxing layout, and the inlined collection size optimization.
- **[jit_emitters.md](jit_emitters.md):** Covers the translation of bytecode instructions (arithmetic, control flow, memory, and database operations) into native Cranelift IR or FFI calls, and the eager execution pre-pass path.
