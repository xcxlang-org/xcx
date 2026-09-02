# Compiler — Statements & Control Flow

The statement compiler translates top-level AST `Stmt` nodes into bytecode. It handles assignments, variable declarations, I/O, database initializations, and control flow structures (if, while, for). The same statement-compilation entry points, and the same `loop_stack`/`LoopFrame` representation described below, are shared by the AST compiler (`compile_control.rs`) and the HIR compiler (`hir/compile_hir.rs` — see `compiler/hir/hir_codegen.md`).

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
- **Assignment**: Differentiates between local and global variables, emitting `Move` and `SetVar` respectively. Handles `+= 1` / `-= 1` inline optimizations (`IncLocal`, `IncVar`, `DecLocal`, `DecVar`). String-concatenating self-assignment (`var = var + expr`, `obj.field = obj.field + expr`, `arr.update(i, arr.get(i) + expr)`) is recognized as its own pattern and compiled to the `StrAppend*` family instead — see below.
- **Multiple Variable Declarations (`MultiVarDecl`)**: Recursively compiles each nested variable declaration statement in sequence, preserving their stack ordering.
- **Serve & Net**: Compiles `net.method` into `HttpRequest` and `serve:` into `HttpServe`, passing HTTP arguments as a pre-constructed `MapLiteral`.
- **JSON**: Resolves `json:bind` and `json:inject` into optimized `JsonBindLocal`/`JsonBindGlobal` or fast injection pipelined equivalents.
- After every call or `for`-loop compilation that advances `next_local`, `sync_max_locals()` is invoked to keep `max_locals_used` — the value that ultimately sizes the runtime stack frame — in sync with the high-water mark of register usage.

---

## Control Flow (`compile_control.rs`)

Generates forward-jump placeholders and backpatches them once the target block is fully compiled. Uses a `loop_stack` to track bounding indices for `break` and `continue`.

### `loop_stack` / `LoopFrame`

```rust
pub struct LoopFrame {
    pub start_pc: usize,
    pub breaks: Vec<usize>,
    pub continues: Vec<usize>,
    pub fiber_reg: Option<usize>,
}

pub loop_stack: Vec<LoopFrame>
```

Pushed on loop entry, popped on loop exit. `start_pc` is the loop's re-entry address (the `continue` jump target). `break` and `continue` instructions append their IP to `breaks`/`continues` on the current top frame for deferred backpatching once the loop's end address is known. `fiber_reg` holds the register of the fiber being iterated, if any — used to emit a safe hidden `Close` call when `break` fires inside a fiber loop. Both `compile_control.rs` (AST path) and `hir/compile_hir.rs` (HIR path) push and consume the same `LoopFrame` type, so loop compilation logic does not need to be duplicated per pipeline.

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
- **Array / Set (`in obj`)**: Sets are first converted to an indexable value array via a silent `MethodCall` to `kind: Values`. The object length is loaded via a silent `MethodCall` to `kind: Size`, and a hidden index register drives an `ArrayLoopNext` fused instruction for high-speed bounds-checked iteration.
- **Fiber (`in fiber_obj`)**: Executes the fiber until `IsDone` returns true, yielding values from `Next` directly to the `var_name` register. Emits a safe hidden `Close` method call if `break` is executed inside a fiber loop.

Emission of `LoopNext`, `IncLocalLoopNext`, `IncVarLoopNext`, `ArrayLoopNext`, and `TableIter` in the JIT is centralized in `src/jit/emit_control.rs` behind dedicated `emit_*_opcode` helper functions rather than building each instruction's arguments inline at every call site.

---

## String Append Optimization: `StrAppendVar` / `StrAppendLocal` / `StrAppendMember` / `StrAppendElement`

The self-concatenation pattern `var = var + expr` (global and local; the variable's static type must be `Type::String`), `obj.set(k, obj.get(k) + expr)` (JSON-typed receiver), and `arr.update(i, arr.get(i) + expr)` (string-array element) is recognized directly by the statement compiler and lowered to one of four dedicated opcodes instead of a generic read/allocate/write sequence. The member/element forms are matched on the **method-call** syntax (`set`/`get`/`update`), not on assignment syntax:

- `OpCode::StrAppendVar { var_idx, src }` — global variable target.
- `OpCode::StrAppendLocal { local_idx, src }` — local variable target.
- `OpCode::StrAppendMember { container, name_idx, src }` — JSON object field target.
- `OpCode::StrAppendElement { container, index, src }` — string array element target.

At the VM level, each opcode attempts an in-place buffer extension via `StringObj::try_extend_bytes` when the target string's `Arc` has a unique owner (`Arc::strong_count <= 1`), falling back to a full copy-on-write clone otherwise — see `compiler/vm/vm_opcode.md` and `compiler/vm/vm_value.md` for the runtime side.

Both the AST compiler (`compile_stmt.rs`) and the HIR compiler (`hir/compile_hir.rs`) recognize this pattern, including chained concatenations with left-associative recursion (e.g. `res = res + "a" + "b"`), which are flattened at compile time into a sequence of `StrAppendLocal`/`StrAppendVar` instructions rather than a nested binary-add tree — provided the target variable is not read again on the right-hand side of the expression.

---

## Variable and Database Declarations (`compile_decl.rs`)

Resolves lexical scoping by mapping a declared `StringId` to the current `FunctionCompiler::next_local`. Scope boundaries push and pop `scopes` blocks.
Variables declared at the root script level outside any function are treated as globals.

**Database Initialization Edge Case**:
The AST explicitly distinguishes `DatabaseDecl`. The compiler `compile_database_decl` looks for explicit map keys `"engine"` and `"path"`. All other map values are strictly treated as Tables. It emits `DatabaseInit` wrapping engine, path, and an arbitrary contiguous register span of tables. Validation that a field marked `is_const` is not reassigned is intentionally left to the semantic analyzer rather than duplicated here.

---

## Returns, Yields & Fibers (`compile_fiber.rs`)

- **Fibers**: A fiber is identical to a function, compiled as an ordinary `Chunk` where `is_fiber = true`.
- **Return**: A `return;` is guaranteed to compile as `ReturnVoid`. Returning expressions emit `Return(src)`. The compiler automatically injects `ReturnVoid` at EOF if absent.
- **Yield**: Fibers pause execution securely via `Yield { src }`. 
- **YieldFrom (Sugar Compilation)**: `YieldFrom(fiber)` is NOT a primitive opcode. It statically explodes directly into raw bytecode equivalent to a native loop:
  `while (!f.IsDone()) { yield f.Next(); }`
  This saves VM instruction width but slightly bloats chunk length for nested generator unpacking.