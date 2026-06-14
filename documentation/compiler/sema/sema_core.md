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
The `Checker` dynamically tracks environment topology as it enters and leaves AST branch nodes:
- `loop_depth`: Increments on entering `while` and `for` blocks. `break` and `continue` keywords check if `loop_depth > 0`.
- `fiber_context`: An `Option<Option<Type>>` designating if execution is currently inside a fiber body, and whether that body yields values (typed) or yields void.
- `in_yield_expr` / `fiber_has_yield`: Safe-guards to ensure `yield` and Database I/O calls (`fetch`, `push`) are securely routed.

---

## The Symbol Table (`symbol/`)

The `SymbolTable` (`symbol_table.rs`) represents the lexical environment. It handles variable shadowing and type tracking.

- **Stack Allocation**: Implemented as a nested `Vec<Scope>`. Entering an `if` block, `for` loop, or `function` pushes a `Scope`. Exiting pops it. Lookups iterate from the deepest scope backward mapping variable names to their in-memory semantic type (`Type`).
- **Constant Enforcements**: A mapped variable wrapper (`Symbol`) tracks `SymbolKind::Constant` or `SymbolKind::Variable`. The checker validates `check_assign` and strictly errors on `ConstReassignment`.
- **Global Injections**: The standard global type constructors (`i`, `f`, `s`, `b`), and the contextual `input` pipeline string, are injected directly into the root scope upon initialized semantic analysis.
- **Table Scope Shadowing**: A unique sub-branch `symbols.define("__row_tmp", ...)` is securely attached when analyzing SQL `.where()` lambda predicates. It dynamically shadows variables to allow direct injection of row columns as variables without requiring explicit property accesses.
