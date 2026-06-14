# Compiler — Statements & Control Flow

The statement compiler translates top-level AST `Stmt` nodes into bytecode. It handles assignments, variable declarations, I/O, database initializations, and control flow structures (if, while, for).

---

## Module Layout

```
src/compiler/
├── compile_stmt.rs      — Main statement dispatch (print, assign, input, etc.)
├── compile_control.rs   — Control flow structures (if, while, for, break, continue)
├── compile_decl.rs      — Lexical variable and database declaration tracking
├── compile_fn.rs        — Function definition and compilation dispatch
├── compile_fiber.rs     — Fiber creation, execution, and yielding
├── compile_query.rs     — SQL AST query to JSON translation
└── compile_table.rs     — Table definition and DDL compilation
```

---

## Statement Dispatch (`compile_stmt.rs`)

```rust
pub fn compile_stmt(&mut self, stmt: &Stmt, ctx: &mut CompileContext)
```

Iterates over the statement kind and dispatches compilation:
- **I/O (`Print`, `Input`, `TerminalWrite`)**: Compiles the source expression, emits the I/O opcode, and pops the source register.
- **Halt (`Alert`, `Error`, `Fatal`)**: Emits the respective halt opcode and stops further block compilation if fatal.
- **Assignment**: Differentiates between local and global variables, emitting `Move` and `SetVar` respectively. Handles `+= 1` / `-= 1` inline optimizations (`IncLocal`, `IncVar`, `DecLocal`, `DecVar`).
- **Serve & Net**: Compiles `net.method` into `HttpRequest` and `serve:` into `HttpServe`, passing HTTP arguments as a pre-constructed `MapLiteral`.
- **JSON**: Resolves `json:bind` and `json:inject` into optimized `JsonBindLocal`/`JsonBindGlobal` or fast injection pipelined equivalents.

---

## Control Flow (`compile_control.rs`)

Generates forward-jump placeholders and backpatches them once the target block is fully compiled. Uses a `loop_stack` to track bounding indices for `break` and `continue`.

### `loop_stack`

```rust
// (start_ip, unresolved_breaks, unresolved_continues, fiber_yield_target)
pub loop_stack: Vec<(usize, Vec<usize>, Vec<usize>, Option<usize>)>
```
Pushed on loop entry, popped on loop exit. `break` and `continue` instructions append their IP to the current top entry's lists for deferred backpatching.

### `If` Statements

Creates a chain of `JumpIfFalse` testing each branch condition, skipping past the success block if false. A final unconditional `Jump` at the end of every successful branch block directs execution to `end;`. All skips are backpatched at the end.

### `While` Statements & Optimization

The compiler includes an algebraic simplifier that attempts to recognize standard indexing loops.

If a `while` loop condition matches the pattern `<counter> < <limit>`, `<counter> <= <limit>`, or their `>` equivalents, where `<counter>` is a simple local variable:
1. Calculates the terminating constraint boundary.
2. Emits an initial `JumpIfFalse` bounds check to skip execution if initially out-of-bounds.
3. Compiles the body.
4. Rewrites the final variable increment inside the body from a standard `IncLocal` or `Add` into a fused `LoopNext` or `LoopPrev` instruction that combines increment, condition test, and backward jump in a single VM cycle.

### `For` Statements

Separates compilation into three specialized pipelines depending on `ForIterType`:
- **Range (`a to b`)**: Compiles like an optimized `while` loop. Fuses the increment and bound check into `LoopNext` / `IncLocalLoopNext`.
- **Array / Set (`in obj`)**: Loads the object length via a silent `MethodCall` to `kind: Size`. Uses a hidden index register, emitting an `ArrayLoopNext` fused instruction for high-speed bounds-checked iteration.
- **Fiber (`in fiber_obj`)**: Executes the fiber until `IsDone` returns true, yielding values from `Next` directly to the `var_name` register. Emits a safe hidden `Close` method call if `break` is executed inside a fiber loop.

---

## Variable and Database Declarations (`compile_decl.rs`)

Resolves lexical scoping by mapping a declared `StringId` to the current `FunctionCompiler::next_local`. Scope boundaries push and pop `scopes` blocks.
Variables declared at the root script level outside any function are treated as globals.

**Database Initialization Edge Case**:
The AST explicitly distinguishes `DatabaseDecl`. The compiler `compile_database_decl` looks for explicit map keys `"engine"` and `"path"`. All other map values are strictly treated as Tables. It emits `DatabaseInit` wrapping engine, path, and an arbitrary contiguous register span of tables.

---

## Returns, Yields & Fibers (`compile_fiber.rs`)

- **Fibers**: A fiber is identical to a function, compiled as an ordinary `Chunk` where `is_fiber = true`.
- **Return**: A `return;` is guaranteed to compile as `ReturnVoid`. Returning expressions emit `Return(src)`. The compiler automatically injects `ReturnVoid` at EOF if absent.
- **Yield**: Fibers pause execution securely via `Yield { src }`. 
- **YieldFrom (Sugar Compilation)**: `YieldFrom(fiber)` is NOT a primitive opcode. It statically explodes directly into raw bytecode equivalent to a native loop:
  `while (!f.IsDone()) { yield f.Next(); }`
  This saves VM instruction width but slightly bloats chunk length for nested generator unpacking.
