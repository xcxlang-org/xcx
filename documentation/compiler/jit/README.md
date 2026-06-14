# JIT Compilation Engine (`src/jit/`)

The following collection is the documentation for the integration system with the Cranelift machine compilation backend, nan-boxing alignment of types, and compilation logic using structure emitters.

## Directory Index

- **[jit_core.md](jit_core.md):** Structure and definitions of the integration instance for compiler environment interfaces associated with the execution architecture call stack under the Wrapper-Inner model.
- **[jit_codegen.md](jit_codegen.md):** Scope testing of preloading into the state base for operation types (JIT Type Inference), static allocation rules in address reduction, and mapping of bit configurations for nan-boxing bitwise specifications.
- **[jit_emitters.md](jit_emitters.md):** Documentation verification for the deployed mapping of emission environment commands and native compiler modules into Cranelift base instructions for logical blocks.
