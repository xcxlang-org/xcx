# XCX Technical Documentation Suite

Welcome to the official technical documentation for XCX 4.2 and the XCX Compiler.

## 📖 Language Reference
Comprehensive guides on the XCX language syntax and features.

- [Syntax Basics](language/syntax.md): Comments, identifiers, and blocks.
- [Variables & Constants](language/variables.md): Declarations, reassignment, and shadowing.
- [Data Types](language/types.md): Simple types, complex types, and casting.
- [Operators](language/operators.md): Arithmetic, string, logic, comparison, and sets.
- [Control Flow](language/control_flow.md): Conditionals and loops.
- [Functions & Fibers](language/functions_fibers.md): Subroutines, coroutines, and delegation.
- [Collections](language/collections.md): Arrays, Sets, Maps, and Tables (Relational).
- [JSON & HTTP](language/json_http.md): Data interchange, networking, and safety.
- [Date & Time](language/dates.md): Creation, fields, and formatting.
- [I/O & Terminal](language/io_terminal.md): Input, output, delays, and system commands.
- [String Methods](language/string_methods.md): Comprehensive list of string utilities.
- [Error Handling](language/errors_halt.md): Halt levels (alert, error, fatal).
- [Standard Library](language/library_modules.md): Built-in modules (`crypto`, `store`, `env`).

## ⚙️ Compiler Internals
Technical deep-dives into how the XCX compiler, JIT, and VM work.

- **[Compiler Architecture Index](compiler/README.md):** Main entry point for the XCX compiler technical documentation.
  - [Frontend Syntax](compiler/frontend/): Lexing, parsing, and AST specifications.
  - [Semantic Analysis](compiler/sema/): Type checking and scoping validators.
  - [Compiler Code Gen](compiler/compiler/): Register mapping and VM bytecode translation.
  - [HIR Pipeline](compiler/hir/): High-level IR lowering, inlining, and compiler passes.
  - [JIT Engine](compiler/jit/): Cranelift trace compilation and NaN-boxing.
  - [Virtual Machine](compiler/vm/): Bytecode execution interpreter loop and heap values.
  - [Runtime Services](compiler/runtime/): Linked collection APIs and FFI linkages.
  - [Diagnostics Gutter](compiler/diagnostics/): Spans, Severity types, and formatted errors.
  - [Interactive Shell](compiler/repl/): Persistent REPL workspace logic.

## 📦 Tooling
Guides for the XCX ecosystem tools.

- [PAX Manual](tooling/pax_manual.md): Package management and project structure.
- [DOC Tool Manual](tooling/doc_manual.md): Offline error code reference utility.

---
*Documentation Version: 4.2.0 *
