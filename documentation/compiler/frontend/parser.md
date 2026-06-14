# Parser

The XCX Parser is written structurally as a recursive descent parser combined tightly with a system-level analyzer utilizing top-down operator precedence (Pratt parsing) upon verifying connected logical formations. Its verification deployment loops are placed under `src/frontend/parser/` to control syntax structures.

---

## Block Modules

```
src/frontend/parser/
├── mod.rs          — re-exports
├── parser.rs       — Parser struct, parse_program, base helper logic
├── pratt.rs        — parse_expression, current_precedence, peek_precedence
├── precedence.rs   — Precedence enum, Precedence::for_token
├── parse_expr.rs   — parse_prefix, parse_infix, and expression parser forms
├── parse_stmt.rs   — parse_statement_internal, and statement formatting
├── parse_type.rs   — parse_type, is_type_intro
├── parse_misc.rs   — parse_var_decl, parse_include_stmt, parse_serve_stmt,
│                     parse_database_decl
├── recovery.rs     — error, synchronize, expect, expect_semicolon
├── token_stream.rs — advance
└── expander.rs     — Expander (operational processing for include resolution and alias prefixing)
```

---

## Parser Structure

```rust
pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    pub interner: Interner,
    pub source: &'a str,
    pub current: Token,
    pub peek: Token,
    pub previous: Token,
    pub has_error: bool,
    pub depth: usize,
}
```

The Parser tracks state operations across shifted tokens (`current` and `peek`) to pipe modified system loops down from the `previous` token history. This aids error handling context and allows mapped fallback logic reporting back to the compiler. The `depth` acts as a guard against excessive recursion depth; any call dropping an instance into the parser increments an integration loop, and hitting the `depth` limit forces compilation out through a panic escape hatch bound specifically to 500 loop captures.

### Structural Construction

```rust
Parser::new(source: &str) -> Parser
Parser::new_with_interner(source, lexer, interner) -> Parser
```

`new` loads an optional format block over the pointer `Interner` structure. A precursor baseline formatting logic for expanders applies `new_with_interner` injecting sub-parsers back up via parents utilizing the shared `interner` ID map without allocating independent string arrays across child parser components.

Construction calls loop `next_token` twice upon initiation to properly allocate the `current` and `peek` buffers.

### Main Module Program Parser (`parse_program`)

```rust
pub fn parse_program(&mut self) -> Program
```

The pre-defined execution logic loops system inputs reading statements continuously through an empty sequence buffer. Cycle testing reads jump logic modification boundaries checking against `EOF`. The loop iteratively triggers `parse_statement()`, assigning success options straight back into `Program { stmts }`. Any logic failures inside statements drop out triggering specific formatting reports to flag the precursor tag using error recovery `synchronize()`.

### Interner Base Extraction (`into_interner`)

```rust
pub fn into_interner(self) -> Interner
```

Drops parsing input parameters and discharges the modified `interner` out of the lexer buffer structure. Extensively deployed within sub-compilation logic blocks via the expander passing string logic IDs natively back onto its parent parser wrapper without duplication.

---

## Token Stream Advance (`token_stream.rs`)

```rust
fn advance(&mut self)
```

Rotates the parsed logic pipeline testing modifiers sequentially:
```
previous ← current
current  ← peek
peek     ← lexer.next_token(&mut interner)
```

The system steps along predefined boundaries checking operations via functions like `expect`, updating modifier logic jumps by dropping operational errors back into checking sequences `expect_semicolon`. The compiler recursively invokes `self.advance()` internally routing parsed subset blocks into logic branches.

---

## Precedence Ranks (`precedence.rs`)

```rust
pub enum Precedence {
    Lowest,
    Lambda,       // ->
    Assignment,   // =
    LogicalOr,    // OR, ||
    LogicalAnd,   // AND, &&
    Equals,       // == !=
    LessGreater,  // > < >= <= HAS
    Sum,          // + - ++
    SetOp,        // UNION INTERSECTION DIFFERENCE SYMMETRIC_DIFFERENCE
    Product,      // * / %
    Power,        // ^
    Prefix,       // -x  !x
    Concatenation,// ::
    Call,         // f(x)  a.b  a[i]
    AsPrec,       // as
}
```

`Precedence` derives `PartialOrd` so that `<` comparisons in the Pratt loop work directly. Higher variants = higher binding power.

`Precedence::for_token(kind)` maps a `TokenKind` to its precedence. The same mapping is duplicated in `current_precedence()` and `peek_precedence()` on the parser for efficiency.

---

## Pratt Expression Parser (`pratt.rs`)

### `parse_expression(precedence: Precedence) -> Option<Expr>`

The core Pratt loop:

1. Call `parse_prefix()` to get the initial left-hand expression.
2. While the current token is not `Semicolon` or `EOF` and `precedence < current_precedence()`: call `parse_infix(left)` and update `left`.
3. Return `left`.

The `depth` guard applies here identically to statement parsing to prevent stack overflows on recursive logic drops.

### `current_precedence() / peek_precedence()`

Both inspect the respective token and return the matching `Precedence` variant, or `Lowest` for anything unrecognized. The distinction matters during infix parsing where the parser must compare the current lookahead's precedence against the caller's minimum constraint boundary.

---

## Expression Parsing (`parse_expr.rs`)

### `parse_prefix`

Dispatches on `self.current.kind`:

| Token | Result |
|---|---|
| `Identifier(_)` | If followed by `(`, parse as `FunctionCall`. Otherwise `Identifier`. |
| Type keywords (`TypeI`, `TypeF`, etc.), logic keywords used as identifiers | Intern the keyword text, then check for `(` same as identifier. |
| `IntLiteral(v)` | `ExprKind::IntLiteral(v)` |
| `FloatLiteral(v)` | `ExprKind::FloatLiteral(v)` |
| `StringLiteral(id)` | `ExprKind::StringLiteral(id)` |
| `RawBlock(id)` | `ExprKind::RawBlock(id)` |
| `True` / `False` | `ExprKind::BoolLiteral` |
| `Minus`, `Bang`, `Not` | `ExprKind::Unary` with `parse_expression(Precedence::Prefix)` as operand |
| `LeftParen` | Parse comma-separated expressions. Single expression → unwrap parentheses. Two or more → `Tuple`. |
| `Set` | If followed by `:`, parse typed set literal. Otherwise treat as identifier. |
| `Map` | Parse `MapLiteral`. |
| `Table` | Parse `TableLiteral`. |
| `Random` | Dispatch to `random.int`, `random.float`, or `random.choice` sub-parsers. |
| `Date` | Parse `DateLiteral`. |
| `Net`, `Json`, `Store`, `Crypto`, `Env` | Parse `ModuleCall`. |
| `Dot` | Parse `TerminalCommand`. |
| `Fiber` | Parse lambda-style fiber expression or fiber call. |
| `Tag(id)` | `ExprKind::Tag(id)` |

### `parse_infix(left: Expr) -> Option<Expr>`

Dispatches on `self.current.kind` to handle operators that appear after a left-hand expression:

| Token | Result |
|---|---|
| Arithmetic / logical operators | `ExprKind::Binary { left, op, right }`. Right side parsed with the operator's own precedence (or `+1` for right-associative operators like `^`). |
| `Dot` | Peek at next: if followed by identifier and then `(`, parse `MethodCall`. Otherwise `MemberAccess`. |
| `LeftBracket` | `ExprKind::Index { receiver: left, index }`. |
| `As` | `ExprKind::As { expr: left, name }`. |
| `Equal` | `ExprKind::Binary` with assignment semantics (handled by the semantic layer; syntactically identical to binary). |
| `Arrow` | Lambda: parse parameter list from `left`, then parse body expression. |

### Argument Parsing

```rust
fn parse_arguments(&mut self) -> Vec<Argument>
```

Reads comma-separated arguments until `)` or `EOF`. Each argument is either:
- `name: expr` → `Argument::Named(name, expr)`
- `expr` → `Argument::Positional(expr)`

Named arguments must use a bare identifier followed by `:`.

### Set Literal Content

```rust
fn parse_set_literal_content(&mut self, st: SetType, lit_span: Span) -> Option<Expr>
```

Parses the interior of `set:T{...}`. If the first expression is followed by `,,`, it is a range (`SetRange`); otherwise it is an element list. An optional `@step` follows the end of a range.

### Map Literal

```rust
fn parse_map_literal(&mut self) -> Option<Expr>
```

Strict structure:
```
map { schema = [KeyType <-> ValueType]  data = [k :: v, ...] }
```
The `schema` block is mandatory. `data` may contain `EMPTY` for an empty map.

### Table Literal

```rust
fn parse_table_literal(&mut self) -> Option<Expr>
```

Structure:
```
table {
    columns: [ colName :: Type @attr ..., ... ]
    rows:    [ [val, ...], ... ]
}
```
Column attributes (`@pk`, `@auto`, `@unique`, `@optional`, `@default(expr)`, `@fk(table, col)`) are parsed inline after the column type.

---

## Statement Parsing (`parse_stmt.rs`)

### `parse_statement_internal`

Top-level dispatch on `self.current.kind`. The parser uses one token of lookahead (`self.peek.kind`) to disambiguate ambiguous prefixes.

Notable disambiguation rules:

- A token that introduces a type (`TypeI`, `TypeF`, `Set`, `Map`, etc.) followed by `=` → assignment. Followed by `(` or `[` or `.` → expression statement. Followed by `:` → variable declaration (or database declaration if the token is `Database`).
- `Identifier` followed by `=` → assignment. Followed by `(` → function call statement. Followed by `:` → variable declaration.
- `End` → returns `None`, signalling the end of a block to the calling parse function.

### Variable Declaration (`parse_var_decl`)

```
[const] Type : name [= expr] ;
```

Type is parsed with `parse_type`. The initializer is optional. When the initializer opens with `{` and a set type was declared, the parser attempts to parse a set literal; otherwise it parses an array literal or a plain expression.

### Assignment (`parse_assignment`)

```
name = expr ;
```

The left-hand side may be a dotted identifier. The right-hand side is any expression.

### If Statement (`parse_if_statement`)

```
if condition then
    body
[elseif condition then
    body]*
[else
    body]
end;
```

`elseif`/`elif`/`elf` all parse the same way. The `then` keyword and `end;` are required.

### While Statement (`parse_while_statement`)

```
while condition do
    body
end;
```

### For Statement (`parse_for_statement`)

```
for var = start to end [@step step] do
    body
end;
```

Also handles `for var in collection do ... end;` for array, set, and fiber iteration. `ForIterType` is set based on the collection type detected at parse time (or left as `Range` when iterating over a numeric range).

### Function Definition (`parse_func_def`)

```
func name([Type param, ... -> ReturnType]) {
    body
};
```

Parameters are `(Type name)` pairs. Return type is optional and placed after an `->` arrow within the parameter list. The block is bounded by `{}` and requires a trailing `;`.

### Fiber Statements (`parse_fiber_statement`)

Two forms:
1. **Definition**: `fiber name(Type param, ...) [: ReturnType] ... end;` — stores as `FiberDef`.
2. **Instantiation**: `fiber [: InnerType] varName = FiberName(args);` — stores as `FiberDecl`.

### Halt Statement (`parse_halt_stmt`)

```
halt alert "message";
halt error "message";
halt fatal "message";
```

### Print / Input

- `>! expr;` → `StmtKind::Print`
- `>? name : Type;` → `StmtKind::Input`

### Return / Yield

- `return [expr];`
- `yield [expr [as name]];` — bare `yield;` becomes `YieldVoid`.
- `yield from expr;` — `YieldFrom`.

### Include

```
include "path/to/file.xcx" [as alias];
```

Stored as `StmtKind::Include`. The `Expander` resolves this at pre-semantic time.

### Serve

```
serve : name {
    port    = expr,
    host    = expr,       -- optional
    workers = expr,       -- optional
    routes  = expr
};
```

`routes` is mandatory; `port` defaults to `8080` if absent.

### Net Request

```
net.method(url [, body]) as target;
```

Where `method` is one of `get`, `post`, `put`, `delete`, `patch`, `head`, `options`. Without `as target`, emitted as a fire-and-forget expression statement.

### Database Declaration

```
database : name {
    field = expr,
    ...
};
```

### Wait

```
@wait(ms);
@wait ms;
```

Both forms are valid.

### Terminal Command

```
.command arg1 arg2;
```

Parsed as `ExprKind::TerminalCommand` and wrapped in `StmtKind::ExprStmt`.

---

## Type Parser (`parse_type.rs`)

```rust
pub fn parse_type(&mut self) -> Option<Type>
```

Parses a single type annotation. If `array:` is present at the start, the inner type is wrapped in `Type::Array`. Supported types:

| Syntax | Result |
|---|---|
| `i` / `int` | `Type::Int` |
| `f` / `float` | `Type::Float` |
| `s` / `string` | `Type::String` |
| `b` / `bool` | `Type::Bool` |
| `date` | `Type::Date` |
| `json` | `Type::Json` |
| `set[:T]` | `Type::Set(SetType)`. Defaults to `SetType::N` if `:T` is absent. |
| `map[:K<->V]` | `Type::Map(K, V)`. Defaults to `(Int, Int)` if `:K<->V` is absent. |
| `table` | `Type::Table(TableType::empty())` |
| `database` | `Type::Database` |
| `fiber[:T]` | `Type::Fiber(Option<Type>)`. Inner type is optional. |
| `array:T` | `Type::Array(Box<Type>)` |

`is_type_intro(kind)` returns `true` for any `TokenKind` that can start a type expression. Used to disambiguate `map:K<->V` from `map` used as a plain identifier.

---

## Error Recovery (`recovery.rs`)

### `error(message: &str)`

Creates a `Reporter`, calls `reporter.error(line, col, len, message)`, and sets `has_error = true`. Does not abort parsing.

### `synchronize()`

Skips tokens until a likely statement boundary is found:
- A `Semicolon` is consumed and the function returns.
- Tokens `Func`, `Fiber`, `Const`, `If`, `While`, `For`, `Return`, `GreaterBang` stop the skip without consuming (the parser will attempt to re-parse from there).
- `EOF` terminates the loop.

### `expect(kind, message) -> bool`

Advances and returns `true` if `current.kind == kind`. Otherwise calls `error(message)` and returns `false` without advancing.

### `expect_semicolon() -> bool`

Specialized `expect` for `;`. The error message is context-aware:
- After `end` → "Expected ';' after 'end'. Every block must be terminated with 'end;'."
- After `then` → "Expected ';' after 'then'. Did you forget the semicolon?"
- After `do` → "Expected ';' after 'do'. Did you forget the semicolon?"
- Otherwise → "Expected ';' at the end of the statement."

---

## Identifier Parsing (`parse_identifier_as_string_id`)

```rust
pub fn parse_identifier_as_string_id(&mut self, allow_dots: bool) -> Option<StringId>
```

Handles the common case where identifiers, reserved keywords, and type tokens may all appear in name positions. The function maps the current token to its canonical text string, advances past it, and if `allow_dots` is true, continues consuming `.identifier` suffixes to build dotted names like `math.sin` or `store.get`. The result is interned and returned as a `StringId`.

---

## Expander (`expander.rs`)

The `Expander` is not strictly part of the parser but runs immediately after parsing and before semantic analysis.

```rust
pub struct Expander<'a> {
    interner: &'a mut Interner,
    visiting_files: HashSet<PathBuf>,
    included_files: HashSet<PathBuf>,
    aliases: HashMap<StringId, String>,
    include_paths: Vec<PathBuf>,
}
```

### `expand(program, current_dir) -> Result<Program, String>`

Walks the top-level statement list. For each `StmtKind::Include`:

1. Resolve the path relative to `current_dir`, then relative to each registered `include_path`.
2. Detect circular includes via `visiting_files`.
3. Skip if already included (no alias) via `included_files`.
4. Lex and parse the included file using the shared interner.
5. Recursively expand the sub-program.
6. If an alias is present, call `prefix_program` on the sub-AST, then merge.

### Alias Prefixing

When `include "lib.xcx" as lib;` is used, all top-level names in `lib.xcx` are renamed to `lib.name` in the AST. Call sites that reference `lib.foo(...)` or `lib.foo` are rewritten to `FunctionCall { name: "lib.foo" }` or `Identifier("lib.foo")` respectively.

The expansion also handles the edge case of mathematical constants (`PI`, `E`, `TAU`, `PHI`, `SQRT2`, `LN2`, `LN10`, `INF`, `INT_MAX`, `INT_MIN`) in aliased includes — these are prefixed the same as functions.

### Include Path Registration

```rust
pub fn add_include_path(&mut self, path: PathBuf)
```

Adds a directory to search when a relative include path cannot be resolved relative to the including file's directory. Typically set to the compiler's standard library directory.