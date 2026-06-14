# XCX Compiler Technical Documentation Suite

This directory contains the comprehensive technical architectural specifications and compiler documentation for the XCX language compiler system (v4.0).

## Table of Contents

### 1. [Frontend Syntax Module](frontend/)
- [Lexer Details](frontend/lexer.md): Token scanning, unicode set operations operators, raw strings, and C-style comments validation.
- [Parser Loop](frontend/parser.md): Recursive-descent grammar rules, preprocessing expansions, synchronizations.
- [AST Architecture](frontend/ast.md): Syntax structures for expressions and statements.

### 2. [Semantic Analyzer Module](sema/)
- [Checker Flow](sema/sema_core.md): Semantic analyzer passes, scoping hierarchies, and validation entrypoints.
- [Semantic Type Check](sema/sema_types.md): Rules for primitive and complex collection constraints.
- [Statement Checker](sema/sema_stmt.md): Declarations, blocks, assignments validation.
- [Expression Checker](sema/sema_expr.md): Check methods, lambdas, operators.

### 3. [Compiler Code Gen Module](compiler/)
- [Compiler Flow](compiler/compiler_core.md): Opcodes generation, constant and function registries.
- [Registers Controller](compiler/compiler_registers.md): Allocation mapping limits.
- [Statement Emitting](compiler/compiler_stmt.md): Loops, variables, conditionals emission.
- [Expression Emitting](compiler/compiler_expr.md): Mathematical calculations, values loading.

### 4. [JIT Compilation Engine](jit/)
- [Core JIT Engine](jit/jit_core.md): Trace compiler integration, wrapper/inner call conventions, symbol registration helpers.
- [JIT Code Gen Context](jit/jit_codegen.md): Cranelift variable setups, pointer-elision static analysis, JIT type tag inferences, and NaN-boxing bit rules.
- [JIT Emitters](jit/jit_emitters.md): Low-level machine representations.

### 5. [Virtual Machine Executor](vm/)
- [VM Executor Execution](vm/vm_executor.md): Interpreter stack layouts, bytecode loops, call-depth limits, and frame dispatch setup.
- [VM Code Specs](vm/vm_opcode.md): Bytecode sets overview, relational table checks.
- [VM Value Systems](vm/vm_value.md): NaN-boxed tags memory representation.
- [VM Objects Structures](vm/vm_objects.md): Maps, sets, arrays, databases, tables, rows, fibers implementations.

### 6. [Runtime Services and Stdlib](runtime/)
- [Core Linkage](runtime/runtime_core.md): Native wrapper functions, FFI setup, boxed conversion.
- [Runtime Subsystems](runtime/runtime_services.md): Crypto timers, network web routing.
- [Runtime Structs](runtime/runtime_collections.md): Custom collections methods, relational transactions.

### 7. [Diagnostics Console Engine](diagnostics/)
- [Errors Pipeline](diagnostics/compiler_errors.md): Spans, Severity levels, Visual colorized terminals `Reporter` output formatting, and comprehensive error code definitions.

### 8. [Interactive Shell Engine](repl/)
- [REPL Workspace](repl/repl.md): Variable persistence, multi-line delimiters checks, and shell helper commands.
