# Semantic Analysis Subsystem (Sema) (`src/sema/`)

This directory documents the static analysis rules, scope controllers, and typing annotations used during symbol resolution in the AST.

## File Index

- **[sema_core.md](sema_core.md):** Semantic analyzer initialization, scoping mechanics, execution passes, and the verification lifecycle.
- **[sema_types.md](sema_types.md):** Support for primitive type verification bindings, dynamic collections, and structurally related database tables.
- **[sema_stmt.md](sema_stmt.md):** Analysis and validators for instructions, variable assignments, operational blocks, and loop control structures.
- **[sema_expr.md](sema_expr.md):** Control rules for expressions, including pointer analyzers for operator methods or local captures of lambda variables.
