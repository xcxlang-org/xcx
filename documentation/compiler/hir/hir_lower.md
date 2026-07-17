# HIR — Lowering (AST → HIR)

Code in `src/hir/lower.rs`, `lower_expr.rs`, and `lower_stmt.rs`.

---

## Entry points: `lower_func` / `lower_program`

```rust
pub fn lower_program(
    program: &Program,
    func_indices: &HashMap<StringId, usize>,
    globals: &HashMap<StringId, usize>,
) -> HashMap<u32, HirFunc>
```

`lower_program` iterates over the top-level AST statements and calls `lower_func` for each `StmtKind::FunctionDef` / `StmtKind::FiberDef`, saving the result under the function index from `func_indices` (the same index register previously populated by `globals::register_globals_recursive` — see `compiler/compiler/compiler_core.md`). It returns a map of `HirFunc` indexed by function number, which is later consumed by `run_inliner_pass` and `compile_hir_to_chunk`.

`lower_func` creates a `HirFuncBuilder`, registers parameters as locals (`define_local`), and then lowers the body statement-by-statement via `lower_stmt`.

---

## `HirFuncBuilder`

```rust
pub struct HirFuncBuilder {
    pub name: StringId,
    pub params: Vec<HirParam>,
    pub return_type: Option<Type>,
    pub body: Vec<HirStmt>,
    pub locals: Vec<HirLocalDef>,
    pub is_fiber: bool,
    pub scopes: Vec<HashMap<StringId, HirLocal>>,
    pub next_local: u32,
}
```

The equivalent of `FunctionCompiler` from the classic AST→bytecode compiler, but operating at the lowering stage. `scopes` is a stack of `name → HirLocal` maps, and `next_local` allocates subsequent indices. `define_local` registers the declaration in `locals` (later consumed by codegen for register allocation) and binds the name in the current scope; `lookup_local` searches scopes starting from the innermost one.

Local name resolution to indices is therefore fully handled at the lowering stage — subsequent stages (inlining, codegen) operate exclusively on numerical `HirLocal` values, not on names.

---

## `lower_stmt` / `lower_expr`

`lower_stmt` maps each variant of AST `StmtKind` to its corresponding `HirStmtKind`, flattening constructs like `MultiVarDecl` (multiple variable declaration) into a sequence of single `VarDecl` statements. Before lowering subexpressions, it builds a flat snapshot of currently visible variables (`resolved_locals`) from all active scopes and passes it to `lower_expr` along with `func_indices` and `globals`. In this way, an expression referencing a local variable compiles directly to its `HirLocal`, whereas a reference to an unresolved name falls back to `HirExprKind::Global`.

`lower_expr` (`lower_expr.rs`) performs a similar mapping for expressions, including resolving the resulting type saved in `HirExpr::ty` — which is required by the codegen phase to generate correct opcodes without re-analyzing types (see [hir_codegen.md](hir_codegen.md)).