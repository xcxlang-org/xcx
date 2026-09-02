# Sema — Statement & Execution Control Flow

The `src/sema/check/` module separates out complex execution checks based on statement structure. Statement assertions validate execution boundaries rather than direct types, throwing explicitly descriptive errors on illegal structures.

---

## Control Flow (`check_control_flow.rs`)

### Loop Safety
- Maintains a strict `loop_depth` integer integer tracker. 
- Entering a `while` or `for` loop dynamically pushes `loop_depth += 1`.
- The parser itself maps `break` and `continue`, but the semantic checker validates whether they are utilized legally (in `check_stmt.rs`). If the semantic `loop_depth` reads `0`, dedicated variants fire: `BreakOutsideLoop` ("[S106] Break statement outside of loop") and `ContinueOutsideLoop` ("[S107] Continue statement outside of loop").

### Specialized `for` Iteration Extrapolation
XCX automatically extrapolates internal type abstractions when iterating:
- Iterating a natively defined `Array(Type::T)` resolves the inner binding variables to type `T`.
- Iterating a `Table` maps internal scope access.
- Iterating a `<Fiber>` forces validation across the internal yielded states, blocking attempts to iterate `None/Void` fibers explicitly with `CannotIterateOverVoidFiber`.

---

## Fiber & Yield Safety (`check_fiber.rs`)

XCX isolates and strictly bounds `yield` functionality exclusively inside declared `@fiber` definitions.

- `check_fiber_def()` configures the internal tracking state `fiber_context` and overrides `is_fiber_context = true`. It validates function closures and verifies trailing `return` consistency.
- **Yield Security**: `check_yield_stmt_with_target` actively audits if `is_fiber_context` is true before resolving. Yield returns throw errors if escaping the explicit `<Fiber>` bound state.
- **D401 Safety Guard**: A raw database operation (`DatabaseOpKind::Remove`) must be filtered with an explicit `.where()` before it is yielded, assigned, passed as an argument, or returned from a typed fiber — the guard fires at all five sites (yield, assignment, expression statement, argument passing, typed-fiber `return`).

---

## Declaration & Variable Binding (`check_decl.rs` & `check_assign.rs`)

- Asserts bounds explicitly. If `Symbols.has(&name_str)` executes securely up the immediate active scope frame on assignments, it permits overwrite. If the assignment tracks `symbols.is_const(&name)`, it panics at `ConstReassignment`.
- `check_var_decl` allows dynamic implicit binding. If the syntax does not declare a strict type, the assignment value natively cascades backward and assigns the AST's mapped variable binding type dynamically based on right-side evaluation.

---

## Network Initialization (`check_net_stmt.rs`)

The `serve:` keyword is audited in `check_net_stmt.rs` and `check_decl.rs`:
- Port/host/workers expressions are generically type-checked; no dedicated Int-port / String-host assertion exists.
- Route parameters are traversed via `ExprKind::Tuple` **and** `ExprKind::ArrayLiteral` (plus `Binary` right sides); identifiers are cross-checked against the `functions` map and the symbol table (with `"*"`/`"_"` exempt). There is no lambda-state validation.
