# HIR — Data Structures

Definitions in `src/hir/hir.rs`. The shape of HIR is close to AST, but with two key differences: expressions carry already resolved types (`ty: Type`), and local variables are represented as flat indices instead of names in lexical scopes.

---

## Local Variables

```rust
pub type HirLocal = u32;

pub const LAMBDA_LOCAL_OFFSET: u32 = 1_000_000;
```

`HirLocal` is a standard `u32` index. Lambdas have their own separate index space — their locals start at `LAMBDA_LOCAL_OFFSET`, so they do not collide with the parent function's locals during lowering or inlining.

```rust
pub struct HirLocalDef {
    pub name: StringId,
    pub ty: Type,
    pub is_const: bool,
}
```

Each `HirFunc` has a `locals: Vec<HirLocalDef>` vector — the declaration under index `i` describes the local with index `i`.

---

## Expressions — `HirExpr` / `HirExprKind`

```rust
pub struct HirExpr {
    pub kind: HirExprKind,
    pub span: Span,
    pub ty: Type,
}
```

Unlike AST `ExprKind`, `HirExprKind` no longer contains nodes that require resolution during the semantic phase (e.g., operator overloads or unresolved references) — this task is performed by the lowering phase, described in [hir_lower.md](hir_lower.md).


```rust
TableLiteral { columns: Vec<ColumnDef>, rows: Vec<Vec<HirExpr>> },
DatabaseLiteral(Vec<(StringId, HirExpr)>),
DateLiteral { date_string: StringId, format: Option<StringId> },
Tuple(Vec<HirExpr>),
```

The rest of the variants cover the full set of language expressions: literals (`IntLiteral`, `FloatLiteral`, `StringLiteral`, `BoolLiteral`), variable access (`Local`, `Global`), operators (`Binary`, `Unary`), calls (`FunctionCall`, `MethodCall`, `ModuleCall`), collections (`ArrayLiteral`, `SetLiteral`, `MapLiteral`), randomness (`RandomChoice`, `RandomInt`, `RandomFloat`), structural access (`Index`, `MemberAccess`), `Lambda`, `As`, `Yield`, `Tag`, and `TerminalCommand`.

---

## Statements — `HirStmt` / `HirStmtKind`

```rust
pub struct HirStmt {
    pub kind: HirStmtKind,
    pub span: Span,
}
```

`HirStmtKind` covers the entire set of AST statements after lowering — declarations (`VarDecl`), local and global assignments (`Assign`, `AssignGlobal`), control flow (`If`, `While`, `For`, `Break`, `Continue`, `Return`), JSON operations (`JsonBind`, `JsonBindGlobal`, `JsonInject`, `JsonInjectLocal`), fibers (`FiberDecl`, `Yield`, `YieldFrom`, `YieldVoid`), network operations (`NetRequestStmt`, `NetRequestStmtGlobal`, `Serve`), and `InlineBlock` — the resulting block used when substituting a function body during inlining (see [hir_inline.md](hir_inline.md)), with an optional `result_local` storing the local variable that receives the value returned by the inlined function.

---

## Function — `HirFunc`

```rust
pub struct HirFunc {
    pub name: StringId,
    pub params: Vec<HirParam>,
    pub return_type: Option<Type>,
    pub body: Vec<HirStmt>,
    pub locals: Vec<HirLocalDef>,
    pub is_fiber: bool,
}
```

A single instance of `HirFunc` represents one function or fiber after lowering. `is_fiber` is read, among other places, by the inlining policy (`should_inline` in `inline_policy.rs`) — fibers are never inlined.