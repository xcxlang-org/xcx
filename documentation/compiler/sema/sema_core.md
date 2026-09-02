# Sema — Core & Symbol Resolution

The `src/sema` module performs the semantic analysis pass on the AST prior to compilation. It is responsible for type-checking, scope resolution, and enforcing language invariants that cannot be captured purely by the parser.

---

## Architectural Entry Point (`checker.rs`)

The central component is the `Checker` struct. It maintains the current traversal state and performs a two-pass analysis:

1. **Pre-scan (`resolution.rs`)**: 
   - A single pass over root-level statements that registers all function and fiber definitions into a global map (`self.functions`). 
   - This ensures that hoisting is supported out of the box — functions can call other functions defined lower in the source file, or recursively call themselves, without hitting an `UndefinedVariable` error.

2. **Main Check**:
   - `Checker::check()` iterates through every standard statement.
   - It validates specialized structural rules: Rule S401 enforces that if a `serve:` keyword is present, it *absolutely must* be the final statement evaluated sequentially. Any statement trailing a server declaration immediately triggers a `TypeErrorKind::Other` error to prevent dead code blocks behind the blocking network listener.

### `Checker` State Bounding
The `Checker` does not own the symbol table — `SymbolTable` is passed as a parameter to `check(&mut self, program, symbols)`. Its actual fields track traversal state: `interner`, `functions` (the pre-scan registry), `loop_depth`, `fiber_context`, `is_fiber_context`, `is_table_lambda`, `fiber_has_yield`, `in_yield_expr`, and `last_expr_was_db_io`.
- `loop_depth`: Increments on entering `while` and `for` blocks (and resets to 0 inside fiber bodies). `break` and `continue` check `loop_depth > 0` and raise `BreakOutsideLoop` / `ContinueOutsideLoop` (S106/S107).
- `fiber_context`: An `Option<Option<Type>>` designating if execution is currently inside a fiber body, and whether that body yields values (typed) or yields void.
- `in_yield_expr` / `fiber_has_yield` / `last_expr_was_db_io`: Safe-guards to ensure `yield` and Database I/O calls (`fetch`, `push`) are securely routed.

A second `serve:` statement is rejected separately ("Only one serve: statement is allowed in a program").

---

## The Symbol Table (`symbol/`)

The `SymbolTable` (`symbol_table.rs`) represents the lexical environment. It handles variable shadowing and type tracking.

- **Stack Allocation**: Implemented as a nested `Vec<Scope>`. Entering an `if` block, `for` loop, or `function` pushes a `Scope`. Exiting pops it. Lookups iterate from the deepest scope backward mapping variable names to their in-memory semantic type (`Type`).
- **Constant Enforcements**: A mapped variable wrapper (`Symbol`) tracks `SymbolKind::Constant` or `SymbolKind::Variable`. The checker validates `check_assign` and strictly errors on `ConstReassignment`.
- **Global Injections**: Only `input` is defined in the root scope (typed `Type::Unknown`). The type constructors `i`, `f`, `s`, `b` are registered in the `functions` map as `FunctionSignature`s, not in any scope.
- **Table Scope Shadowing**: A unique sub-branch `symbols.define("__row_tmp", ...)` is securely attached when analyzing SQL `.where()` lambda predicates. It dynamically shadows variables to allow direct injection of row columns as variables without requiring explicit property accesses.
