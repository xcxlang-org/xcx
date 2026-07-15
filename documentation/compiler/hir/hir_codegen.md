# HIR — Code Generation (HIR → bytecode)

Code in `src/hir/compile_hir.rs` (statements), `src/hir/compile_expr.rs` (expressions), and `src/hir/compile_expr_special.rs` (calls to special built-in functions, e.g., `i()`, `f()`, `s()`, `terminal.input`).

---

## Entry Point: `compile_hir_to_chunk`

```rust
pub fn compile_hir_to_chunk(
    func: &HirFunc,
    is_fiber: bool,
    ctx: &mut CompileContext,
    name: String,
    param_count: usize,
) -> Chunk
```

Creates a new `FunctionCompiler`, sets `next_local`/`max_locals_used` to the number of locals already declared in `func.locals` (HIR locals have predefined, flat indices and do not need to be allocated dynamically as they are during compilation from the AST), and builds the `local_map: name → index` mapping which is stored as the sole scope in `compiler.scopes`. It then compiles the body statement-by-statement (`compile_stmt`) and terminates the function with an implicit `OpCode::ReturnVoid` if the body does not already end with a return statement. The returned `Chunk` is structurally identical to the one produced by the classic AST→bytecode compiler (`compiler/compiler/compiler_core.md`) — subsequent stages (VM, JIT) do not differentiate between the paths used to generate a given function.

The `FunctionCompiler` (`src/compiler/compiler.rs`) is shared — both the legacy AST path and the new HIR path emit the same `OpCode` set using the same function compiler structure.

---

## Registers and `local_regs`

`compiler.local_regs` (a field in `FunctionCompiler`) is a set of register indices reserved permanently for HIR local variables — in contrast to temporary registers allocated dynamically via `push_reg`/`pop_reg` during the compilation of subexpressions. This distinction is important when compiling `InlineBlock` (see [hir_inline.md](hir_inline.md)) — `inline_stack` and `inline_result_locals` allow redirecting the value returned by the inlined block to the register of the local variable that was the target of the original call.

---

## Compilation of Complex Literals

The HIR compiler fully supports the compilation of complex literals:

- **`TableLiteral { columns, rows }`** — columns are mapped to `VMColumn` (name, type, flags like `is_auto`/`is_pk`/`is_unique`), the table skeleton is created (`Value::from_table`), and instructions are emitted to insert the rows.
- **`DatabaseLiteral(Vec<(StringId, HirExpr)>)`** — fields `engine` and `path` are recognized by name among the provided pairs and compiled to separate registers before emitting the instruction that creates the database handle.
- **`DateLiteral { date_string, format }`** — if a `format` is provided, the date notation (`YYYY`, `MM`, `DD`, ...) is translated to `chrono` format before emitting the constant.
- **`Tuple(Vec<HirExpr>)`** — tuple elements are compiled to adjacent registers starting from `compiler.next_local`, using an explicit `Move` when the result of a subexpression did not land in the target register immediately.

---

## `LoopFrame`

Loops (`While`, `For`) in both compilation paths (AST — `compile_control.rs`, and HIR — `compile_hir.rs`) utilize a shared, typed structure in `loop_stack`:

```rust
pub struct LoopFrame {
    pub start_pc: usize,
    pub breaks: Vec<usize>,
    pub continues: Vec<usize>,
    pub fiber_reg: Option<u8>,
}
```

`start_pc` is the address of the beginning of the loop (the target of a `continue` jump), `breaks`/`continues` are lists of locations to backpatch once the loop end address is finalized, and `fiber_reg` stores the fiber register if the loop iterates over its results (`TableIter`/`ArrayLoopNext` on a fiber).