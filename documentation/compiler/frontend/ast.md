# AST — Abstract Syntax Tree

XCX source code is parsed into a tree of `Stmt` and `Expr` nodes. Every node carries a `Span` for error reporting. The AST is defined under `src/frontend/ast/`.

---

## Module Layout

```
src/frontend/ast/
├── mod.rs        — re-exports
├── node.rs       — Program, AstNode<T>
├── stmt.rs       — Stmt, StmtKind, HaltLevel, ForIterType
├── expr.rs       — Expr, ExprKind, SetRange
├── argument.rs   — Argument (positional / named call args)
├── lit.rs        — Lit (standalone literal enum, minor use)
├── op.rs         — BinOp, UnaryOp (mirror of TokenKind subset)
├── ty.rs         — re-export of sema::types::{Type, SetType, DatabaseOpKind}
├── table.rs      — ColumnDef, ColumnAttribute
└── visitor.rs    — AstVisitor trait
```

---

## Root Node

### `Program`
```rust
pub struct Program {
    pub stmts: Vec<Stmt>,
}
```
The root of every parsed file. Produced by `Parser::parse_program()` and consumed by the `Expander` before semantic analysis.

### `AstNode<T>`
```rust
pub struct AstNode<T> {
    pub data: T,
    pub span: Span,
}
```
Generic span-carrying wrapper. Currently used in specific sub-structures; `Stmt` and `Expr` embed `Span` directly rather than wrapping.

---

## Statements

### `Stmt`
```rust
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}
```

### `StmtKind`

| Variant | Description |
|---|---|
| `VarDecl { is_const, ty, name, value }` | Variable or constant declaration. `value` is `None` when the variable is declared but not initialized. |
| `MultiVarDecl(Vec<Stmt>)` | Grouped list of sequential variable declarations. Created during desugaring of multiple variable declarations (e.g., `i: a, b, c;`). |
| `Print(expr)` | `>!` print statement. Writes to stdout. |
| `TerminalWrite(expr)` | `terminal.write(...)` output. Distinct from print for codegen purposes. |
| `Input(name, ty)` | `>?` read from stdin. Binds result to `name` with declared type `ty`. |
| `ExprStmt(expr)` | Expression used as a statement (e.g. a bare function call). |
| `If { condition, then_branch, else_ifs, else_branch }` | Full if/elseif/else chain. `else_ifs` is a `Vec<(Box<Expr>, Vec<Stmt>)>`. |
| `While { condition, body }` | While loop. |
| `For { var_name, start, end, step, body, iter_type }` | For loop. `iter_type` disambiguates range, array, set, or fiber iteration. |
| `Break` | Breaks out of the nearest enclosing loop. |
| `Continue` | Skips to the next iteration of the nearest enclosing loop. |
| `Assign { name, value }` | Assignment to an existing variable. |
| `Halt { level, message }` | Structured error/abort with `HaltLevel`. |
| `FunctionDef { name, params, return_type, body }` | Function definition. |
| `Return(Option<Expr>)` | Return from a function; `None` for bare `return;`. |
| `FunctionCallStmt { name, args }` | Function call as a statement (return value discarded). |
| `Include { path, alias }` | File inclusion. `alias` is `Some` when `include "..." as name;` syntax is used. |
| `JsonBind { json, path, target }` | Binds a JSON value extracted at `path` to `target`. |
| `JsonInject { json, mapping, table }` | Injects JSON data into a table using a mapping expression. |
| `FiberDef { name, params, return_type, body }` | Fiber (coroutine) definition. |
| `FiberDecl { inner_type, name, fiber_name, args }` | Instantiation of a fiber. `inner_type` is the yielded value type when known. |
| `Yield { value, target }` | Yield a value from a fiber. `target` is `Some` when `yield expr as name;` is used. |
| `YieldFrom(expr)` | Yield all values from another fiber or iterable. |
| `YieldVoid` | Bare `yield;` — signals the end of a fiber's current iteration. |
| `DatabaseDecl { name, fields }` | Database instance declaration with a list of named configuration fields. |
| `NetRequestStmt { method, url, headers, body, timeout, target }` | HTTP request statement. Binds the response to `target`. |
| `Serve { name, port, host, workers, routes }` | Starts an HTTP server. `routes` is required; `port` defaults to `8080`. |
| `Wait(expr)` | `@wait(ms)` — suspends execution for the given number of milliseconds. |

### `HaltLevel`
```rust
pub enum HaltLevel { Alert, Error, Fatal }
```
Maps to the three halt severity keywords. `Fatal` terminates the process; `Error` and `Alert` can be caught or logged depending on runtime behavior.

### `ForIterType`
```rust
pub enum ForIterType { Range, Array, Set, Fiber }
```
Set during parsing to indicate how the for-loop variable is iterated. Codegen uses this to emit the correct runtime iteration call.

---

## Expressions

### `Expr`
```rust
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}
```

### `ExprKind`

| Variant | Description |
|---|---|
| `IntLiteral(i64)` | Integer constant. |
| `FloatLiteral(f64)` | Float constant. |
| `StringLiteral(StringId)` | Interned string constant. |
| `BoolLiteral(bool)` | `true` / `false`. |
| `Identifier(StringId)` | Named reference. After the `Expander` pass, identifiers from aliased includes are rewritten to dotted form (e.g. `mylib.foo`). |
| `RawBlock(StringId)` | Content of a `<<<...>>>` raw block. |
| `ArrayLiteral { elements }` | Array literal. |
| `Binary { left, op, right }` | Binary expression. `op` is a `TokenKind` (see operator list below). |
| `Unary { op, right }` | Unary expression. `op` is `Minus`, `Bang`, or `Not`. |
| `FunctionCall { name, args }` | Call to a named function. |
| `MethodCall { receiver, method, args, wait_after }` | Method call on a value. `wait_after` is set when the call is followed by `@wait`. |
| `SetLiteral { set_type, elements, range }` | Set literal. Either `elements` or `range` is populated, not both. |
| `ArrayOrSetLiteral { elements }` | Ambiguous `{...}` literal; resolved during semantic analysis. |
| `RandomChoice { set }` | `random.choice(set)` — picks a random element. |
| `RandomInt { min, max, step }` | `random.int(min, max)` with optional step. |
| `RandomFloat { min, max, step }` | `random.float(min, max)` with optional step. |
| `MapLiteral { key_type, value_type, elements }` | Map (dictionary) literal. Requires explicit schema. |
| `DateLiteral { date_string, format }` | Date literal. Optional `format` string for custom parsing. |
| `TableLiteral { columns, rows }` | Inline relational table. Columns carry type and attribute information. |
| `DatabaseLiteral(Vec<(StringId, Expr)>)` | Inline database configuration as a list of key-value pairs. |
| `Index { receiver, index }` | Indexing: `receiver[index]`. |
| `MemberAccess { receiver, member }` | Dot access: `receiver.member`. Rewritten to `Identifier` by the `Expander` when the receiver is an alias. |
| `TerminalCommand(StringId, Vec<Expr>)` | `.command arg1 arg2` terminal command expression. |
| `Lambda { params, return_type, body }` | Anonymous function. `params` is `Vec<(Type, StringId)>`. |
| `Tuple(Vec<Expr>)` | Parenthesized comma-separated list of two or more expressions. |
| `ModuleCall { module, method, args }` | Call to a built-in module (e.g. `json`, `net`, `store`). `module` is a `TokenKind`. |
| `As { expr, name }` | `expr as name` — alias binding within an expression context. |
| `Yield(Box<Expr>)` | Yield expression (yields a value and optionally receives one back). |
| `Closure { capture_start, capture_count }` | Capturing wrapper evaluated by `MakeClosure` targeting specific external variables. |
| `Tag(StringId)` | `#tag` literal. |

### `SetRange`
```rust
pub struct SetRange {
    pub start: Box<Expr>,
    pub end: Box<Expr>,
    pub step: Option<Box<Expr>>,
}
```
Used inside `ExprKind::SetLiteral` to represent `set:N{1,,10 @step 2}` syntax. When a `SetRange` is present, `elements` on the enclosing `SetLiteral` is empty.

---

## Arguments

### `Argument`
```rust
pub enum Argument {
    Positional(Expr),
    Named(StringId, Expr),
}
```
Used in function calls, method calls, fiber instantiation, and module calls. Named arguments use `name: value` syntax.

Methods:
- `expr(&self) -> &Expr` — returns a reference to the expression regardless of variant.
- `expr_mut(&mut self) -> &mut Expr` — mutable version.

---

## Types

Types are defined in `sema::types` and re-exported from `ast::ty`. They appear in variable declarations, function signatures, column definitions, and lambda parameters.

| Type | Keyword | Notes |
|---|---|---|
| `Type::Int` | `i`, `int` | 64-bit signed integer. |
| `Type::Float` | `f`, `float` | 64-bit float. |
| `Type::String` | `s`, `string`, `str` | Interned string. |
| `Type::Bool` | `b`, `bool` | Boolean. |
| `Type::Date` | `date` | Date value. |
| `Type::Json` | `json` | JSON blob. |
| `Type::Array(Box<Type>)` | `array:T` | Homogeneous dynamic array. |
| `Type::Set(SetType)` | `set:N`, `set:Z`, etc. | Mathematical set with domain. |
| `Type::Map(Box<Type>, Box<Type>)` | `map:K<->V` | Key-value dictionary. |
| `Type::Table(TableType)` | `table` | Relational table. |
| `Type::Database` | `database` | Database connection handle. |
| `Type::Fiber(Option<Box<Type>>)` | `fiber:T` | Coroutine handle. |

`SetType` variants: `N` (natural), `Q` (rational), `Z` (integer), `S` (string), `B` (boolean), `C` (character).

---

## Table Column Definitions

### `ColumnDef`
```rust
pub struct ColumnDef {
    pub name: StringId,
    pub ty: Type,
    pub attributes: Vec<ColumnAttribute>,
}
```

### `ColumnAttribute`

| Variant | Keyword | Description |
|---|---|---|
| `Auto` | `@auto` | Auto-incrementing column. |
| `PrimaryKey` | `@pk` | Primary key. |
| `Unique` | `@unique` | Unique constraint. |
| `Optional` | `@optional` | Column is nullable. |
| `Default(Expr)` | `@default(expr)` | Default value expression. |
| `ForeignKey(StringId, StringId)` | `@fk(table, col)` | Foreign key reference. |

Helper methods on `ColumnDef`:
- `is_auto() -> bool` — true if `@auto` is present.
- `is_pk() -> bool` — true if `@pk` is present.

---

## Function Signatures

### `FnSig`
```rust
pub struct FnSig {
    pub params: Vec<(Type, StringId)>,
    pub return_type: Option<Type>,
    pub is_variadic: bool,
}
```
Represents the signature of a function or fiber. `is_variadic` is a placeholder for future variadic argument support and is currently always `false`.

---

## Visitor

### `AstVisitor`
```rust
pub trait AstVisitor {
    fn visit_program(&mut self, program: &Program);
    fn visit_stmt(&mut self, stmt: &Stmt);
    fn visit_expr(&mut self, expr: &Expr);
}
```
Default implementations walk the entire tree with no-op leaf visits. Implementors override only the node types they care about. Used internally by semantic analysis and optimization passes.

---

## Operators (Binary)

Binary operator variants carried by `ExprKind::Binary.op` as `TokenKind`:

| TokenKind | Symbol | Precedence | Description |
|---|---|---|---|
| `Plus` | `+` | Sum | Addition |
| `Minus` | `-` | Sum | Subtraction |
| `Star` | `*` | Product | Multiplication |
| `Slash` | `/` | Product | Division |
| `Percent` | `%` | Product | Modulo |
| `Caret` | `^` | Power | Exponentiation |
| `EqualEqual` | `==` | Equals | Equality |
| `BangEqual` | `!=` | Equals | Inequality |
| `Greater` | `>` | LessGreater | Greater-than |
| `Less` | `<` | LessGreater | Less-than |
| `GreaterEqual` | `>=` | LessGreater | Greater-or-equal |
| `LessEqual` | `<=` | LessGreater | Less-or-equal |
| `And` | `AND`, `&&` | LogicalAnd | Logical AND |
| `Or` | `OR`, `\|\|` | LogicalOr | Logical OR |
| `PlusPlus` | `++` | Sum | String concatenation |
| `DoubleColon` | `::` | Concatenation | Collection concatenation |
| `Union` | `UNION`, `∪` | SetOp | Set union |
| `Intersection` | `INTERSECTION`, `∩` | SetOp | Set intersection |
| `Difference` | `DIFFERENCE`, `\` | SetOp | Set difference |
| `SymDifference` | `SYMMETRIC_DIFFERENCE`, `⊕` | SetOp | Symmetric difference |
| `Has` | `HAS` | LessGreater | Set membership test |
| `Bridge` | `<->` | — | Map schema key-value separator |

Unary operators: `Minus` (negation), `Bang` / `Not` (logical NOT).