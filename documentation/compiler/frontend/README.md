# Syntax Subsystem (Frontend) (`src/frontend/`)

This collection contains documentation on lexical analysis, built-in logical parsing rules, and the structural abstraction of Abstract Syntax Tree types—the internal representation of the syntax tree.

## Module File Index

- **[lexer.md](lexer.md):** Lexical token generation, trivia whitespace skipping, multi-character unicode handling barring operators, and the C-style raw comment block bypass tests.
- **[parser.md](parser.md):** Recursive descent semantic analyzer coupled with structural top-down operator precedence (Pratt), early-stage macro inclusion directive expansion, and synchronization logic for Lookahead panic recovery.
- **[ast.md](ast.md):** Structural and declarative layout of the extended interface types (Ast Nodes) based on logically assigned instruction modules (`StmtKind`) and expression branches (`ExprKind`), propagated down the pipeline stream.
