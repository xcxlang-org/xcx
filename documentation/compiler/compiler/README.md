# Compiler Code Generation Subsystem (`src/compiler/`)

This sequence of documentation specifies the pipeline phases of lowering verification processes, converting Abstract Syntax Tree (AST) programs into operational stack-based virtual machine opcodes.

## Directory Index

- **[compiler_core.md](compiler_core.md):** Internal structures covering algorithmic logic verification stages, tracking constant base registers towards native bytecode representation.
- **[compiler_registers.md](compiler_registers.md):** Allocation assignment rules for the distribution manager, tracking machine register controllers alongside security bounds validation for constants.
- **[compiler_stmt.md](compiler_stmt.md):** Block-level translators (emitters) managing registry conversions for machine instructions operating on conditionals, variables, variables blocks, and loop iterations.
- **[compiler_expr.md](compiler_expr.md):** System translators mapping individual equations modifying constants into active state pointers, incorporating function reference loads.
