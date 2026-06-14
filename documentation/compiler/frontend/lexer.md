# Lexer

The Lexer transforms the raw source text of XCX into a flat stream of `Token` values. It is a single-pass, byte-oriented scanner defined in `src/frontend/lexer/`.

---

## Module Layout

```
src/frontend/lexer/
├── mod.rs        — re-exports (Lexer, Token, TokenKind)
├── lexer.rs      — Lexer struct and next_token dispatch
├── cursor.rs     — Cursor — byte-level source navigator
├── token.rs      — Token struct
├── token_kind.rs — TokenKind enum
├── keyword.rs    — keyword lookup table
├── number.rs     — lexical analysis of integer and float literals
├── string_lit.rs — lexical analysis of string literals and raw blocks
└── trivia.rs     — skipping whitespaces and comments
```

---

## Cursor

```rust
pub struct Cursor<'a> {
    pub source: &'a [u8],
    pub pos: usize,          // byte offset
    pub char_pos: usize,     // unicode character count
    pub line: usize,
    pub col: usize,
}
```

The cursor operates on raw bytes for speed. It handles UTF-8 only to track `char_pos` and recognize multi-byte set operation symbols (`∪`, `∩`, `⊕`). All ASCII tokens are processed byte-by-byte.

Key methods:

| Method | Description |
|---|---|
| `peek() -> u8` | Returns the next byte without consuming it. Returns `0` at EOF. |
| `peek_next() -> u8` | Returns the byte two positions ahead. |
| `peek_at(offset) -> u8` | Returns the byte at `pos + offset`. |
| `advance() -> u8` | Consumes and returns the current byte. Updates `line`, `col`, and `char_pos`. |
| `backtrack(n)` | Rewinds `pos` by `n` bytes. Does not fix `line`/`col`; safe only for small peeks on the same line. |
| `remain() -> &[u8]` | Slice from the current position to the end. |
| `slice_from(start) -> &[u8]` | Slice from `start` to the current position. |
| `is_at_end() -> bool` | True when `pos >= source.len()`. |

---

## Token

```rust
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
```

`Span` carries `line`, `col`, and `len` (in characters, not bytes). Created via `Token::new(kind, line, col, len)`.

---

## TokenKind

All variants of the `TokenKind` enum:

### Keywords - Types

| Variant | Source Text |
|---|---|
| `Const` | `const` |
| `TypeI` | `i`, `int` |
| `TypeF` | `f`, `float` |
| `TypeS` | `s`, `string`, `str` |
| `TypeB` | `b`, `bool`, `boolean` |
| `Array` | `array` |
| `Set` | `set` |
| `Map` | `map` |
| `Date` | `date` |
| `Table` | `table` |
| `Database` | `database` |
| `Json` | `json` |
| `TypeSetN` | `N` (case-sensitive) |
| `TypeSetQ` | `Q` (case-sensitive) |
| `TypeSetZ` | `Z` (case-sensitive) |
| `TypeSetS` | `S` (case-sensitive) |
| `TypeSetB` | `B` (case-sensitive) |
| `TypeSetC` | `C` (case-sensitive) |

### Keywords - Control Flow

| Variant | Source Text |
|---|---|
| `If` | `if` |
| `Then` | `then` |
| `ElseIf` | `elseif`, `elif`, `elf` |
| `Else` | `else`, `els` |
| `End` | `end` |
| `For` | `for` |
| `In` | `in` |
| `To` | `to`, `..` |
| `While` | `while` |
| `Do` | `do` |
| `Break` | `break` |
| `Continue` | `continue` |

### Keywords - Logic

| Variant | Source Text |
|---|---|
| `And` | `AND` (case-sensitive), `and`, `&&` |
| `Or` | `OR` (case-sensitive), `or`, `\|\|` |
| `Not` | `NOT` (case-sensitive), `not`, `!!` |
| `Has` | `HAS` (case-sensitive), `has` |
| `True` | `true` |
| `False` | `false` |

### Keywords - Builtins / Modules

| Variant | Source Text |
|---|---|
| `Net` | `net` |
| `Crypto` | `crypto` |
| `Env` | `env` |
| `Serve` | `serve` |
| `Fiber` | `fiber` |
| `Yield` | `yield` |
| `Terminal` | `terminal` |
| `Store` | `store` |
| `Random` | `random` |
| `Choice` | `choice` |
| `Halt` | `halt` |
| `Alert` | `alert` |
| `Error` | `error` |
| `Fatal` | `fatal` |
| `Func` | `func`, `function`, `fun` |
| `Return` | `return` |
| `Include` | `include` |
| `As` | `as` |
| `From` | `from` |
| `Empty` | `EMPTY` |
| `Columns` | `columns` |
| `Rows` | `rows` |
| `Schema` | `schema` |
| `Data` | `data` |

### Set Operations

| Variant | Source Text |
|---|---|
| `Union` | `UNION` (case-sensitive), `∪` |
| `Intersection` | `INTERSECTION` (case-sensitive), `∩` |
| `Difference` | `DIFFERENCE` (case-sensitive), `\` |
| `SymDifference` | `SYMMETRIC_DIFFERENCE` (case-sensitive), `⊕` |

### Identifier and Literal Tokens

| Variant | Description |
|---|---|
| `Identifier(StringId)` | User-defined name. The string is interned; `StringId` acts as a pointer (index) into the `Interner`. |
| `IntLiteral(i64)` | Decimal integer literal. |
| `FloatLiteral(f64)` | Decimal floating-point type (`1.5`, not `1.` or `.5`). |
| `StringLiteral(StringId)` | Double-quoted string literal. Escape characters are resolved during analysis. |
| `RawBlock(StringId)` | Text block inside `<<<...>>>` (scavenger block). Content is preserved exactly as typed. |
| `Tag(StringId)` | Literal hashtag-type attribution token, e.g., `#identifier`. |

### Attribute Tokens (@ prefix)

| Variant | Source Text |
|---|---|
| `AtStep` | `@step` |
| `AtAuto` | `@auto` |
| `AtWait` | `@wait` |
| `AtPk` | `@pk` |
| `AtUnique` | `@unique` |
| `AtOptional` | `@optional` |
| `AtDefault` | `@default` |
| `AtFk` | `@fk` |

`@` is not a generic precursor for arbitrary user configurations. The analyzer maps only known compiler attribute definitions. Using an unrecognized attribute returns `Unknown('@')`.

### Operators

| Variant | Symbol | Note |
|---|---|---|
| `Plus` | `+` | |
| `PlusPlus` | `++` | String concatenation |
| `Minus` | `-` | |
| `Star` | `*` | |
| `Slash` | `/` | C-style comments like `//` and `/* */` intentionally trigger parser exceptions if used; they are strictly disallowed so that programmers catch mistakes instantly. |
| `Percent` | `%` | |
| `Caret` | `^` | Exponentiation math formula |
| `Equal` | `=` | Variable allocation/assignment |
| `EqualEqual` | `==` | |
| `BangEqual` | `!=` | |
| `Greater` | `>` | |
| `Less` | `<` | |
| `GreaterEqual` | `>=` | |
| `LessEqual` | `<=` | |
| `Arrow` | `->` | Lambda separator / function formatter |
| `Bridge` | `<->` | Map schema key↔value link |
| `GreaterBang` | `>!` | Terminal print stdout instance |
| `GreaterQuestion` | `>?` | Terminal input stdin request |

### Punctuation

| Variant | Symbol |
|---|---|
| `LeftParen` | `(` |
| `RightParen` | `)` |
| `LeftBrace` | `{` |
| `RightBrace` | `}` |
| `LeftBracket` | `[` |
| `RightBracket` | `]` |
| `Comma` | `,` |
| `DoubleComma` | `,,` |
| `Colon` | `:` |
| `DoubleColon` | `::` |
| `Semicolon` | `;` |
| `Dot` | `.` |
| `Bang` | `!` |

### Special System Actions

| Variant | Description |
|---|---|
| `EOF` | End Of File flag. Triggered when the input source stream is fully consumed and the machine buffer is empty. |
| `Unknown(char)` | Representation of unrecognized characters. The parser interrupts this, emitting an error communication using the Reporter. |

---

## Lexer

```rust
pub struct Lexer<'a> {
    cursor: Cursor<'a>,
}
```

`Lexer::new(source: &str)` loads the full target source code into its buffer. Token extraction is requested cyclically, driven by calls to `next_token(&mut Interner) -> Token`. The Lexer does not eagerly tokenize the entire file into an iterator; instead, it performs lazy, on-demand scanning per trigger. For memory efficiency, the Parser controls the Lexer, requesting single tokens as needed via `next_token`.

### Token Dispatch Cycle Logic (`next_token`)

1. `trivia::skip` — Consumes all whitespace characters and skips past comments.
2. Captures the current `line` and `col` to construct a `Span` for error reporting and symbol tracking.
3. Checks for the end of the file, returning the `EOF` token if reached.
4. Examines the current byte `c`:
   - Decimal Digits — Delegates to `number::lex` for numeric parsing, utilizing small token backtracking prior to transferring logic flow.
   - Alphabetical ASCII or `_` — Delegates to `Lexer::identifier`. This loops over alphanumeric characters, collecting the string and dispatching it to `keyword::lookup` to check against known keywords vs custom variables.
   - Bytes `>= 128` (Non-ASCII) — Transitions to logic interpreting UTF-8 math set operators (`∪`, `∩`, `⊕`), probing prefixes against static mapping arrays. Will return `Unknown` if invalid.
   - Double Quote `"` — Consumes the delimiter and passes structural handling to `string_lit::lex`.
   - Hashtag `#` — Controller searches bounds for alphanumeric text block loops matching suffix tag traits, throwing the grouped logic as a `Tag` using the internally captured `Interner` index string ID.
   - Operator `<` — Inspects the bounds for compound operators. If it catches `<<<`, it delegates extraction to `string_lit::lex_raw`, which slurps all text raw, bypassing typical escape decoding logic inside standard strings.
   - Modifying Attribute `@` — Bumps the parser search buffer to a small boundary of jumps matching standard compiler procedural attributes (e.g. `@step`, `@auto`). Follows strict bounded rules mapping optional behavior checks.
   - Unrecognized / Isolated Operators — Falls out of logic into an exhaustive static symbol map match block to yield single or double-character relational or mathematical tokens.

### Identifier and Keyword Recognition

```rust
fn identifier(&mut self, line, col, interner) -> Token
```

Reads byte segments in a loop while `is_ascii_alphanumeric() || c == '_' || c >= 128`. The gathered structure is passed along as `text`, matched via a dictionary lookup assistant on the underlying keyword pool via `keyword::lookup(text, interner)`.

The `keyword::lookup` system operates over two phase mappings:
1. **Case-sensitive checks:** Mathematical set operations (`UNION`, `INTERSECTION`), control logical operators (`AND`, `OR`, `NOT`, `HAS`), and explicit Set modifiers (`N`, `Q`, `Z`, `S`, `B`, `C`).
2. **Case-insensitive checks:** All other system modules and loop bindings default back to `to_lowercase()`, matching seamlessly against mapping checks.

Compilation flow fallback binds anything outside of mapped definitions as a programmatic user variable instance, strictly returning standard `Identifier(StringId)` tokens.

### Numeric Lexical Analysis (`number::lex`)

A buffering procedure processes sequence bytes character by character until it determines the token boundaries. It executes logic checks for decimal separators (`.`) and delegates to optional fallback routines for method accessors. If the period is detected to be part of an object method accessor (`foo.bar`), it splits the token instead of treating it as a decimal point. Fully verified numeric sequences without a fractional part (`1`, `33`) map dynamically into `i64` integers, whereas values containing a fractional component are parsed under `f64` boundaries.

### String Literal Lexing (`string_lit::lex`)

This mechanism captures text sequences bounded completely by double quotation marks `"`. It includes a compiler assistant sequence for validating and escaping characters:

| Escape Sequence | Value |
|---|---|
| `\n` | Newline formatting (line feed). |
| `\t` | Tab indentation spacing sequence (tab). |
| `\r` | Carriage return operator pulling text output logic to line start. |
| `\0`–`\7...` | Octal format processing sequence (up to 3 characters long). |
| `\xHH` | Hexadecimal formatting extraction (up to 2 characters long). |
| `\"` | Escaped double quote literal bypassing string closure markers. |
| `\\` | Raw backslash parsing literal directly pushed to input buffers. |

Any unrecognized escape sequence formatted as `\X` will safely fall back and output directly into the parsed structure as text.

### Raw Block Lexing (`string_lit::lex_raw`)

When the scanning controller hits the `<<<` delimiter, it triggers an unrestricted text ingestion sequence. Precursor logic analysis evaluates and reads all raw text sequences without any string evaluation or escape token replacements, continuously caching strings until it encounters the closure operator `>>>`. Bounded operations are stored into system memory safely utilizing `RawBlock(StringId)` pointers. This avoids conventional formatting issues present inside string parsing loops (i.e. ignoring string interpolation completely and returning pure text).

### Whitespace and Comments (Trivia: `trivia::skip`)

This skips over empty ASCII whitespace without touching the central compiler parsing rules. Furthermore, it manages commentary structures bypassing syntactical evaluations:

| Syntax Type | Example | Description |
|---|---|---|
| Single Line Comment | `--- ignores line` | Starts with the `---` flag bounding all trailing characters within a line and effectively ignoring them from the compilation loop execution. |
| Multi-line Block Comment | `--- ... *---` | Spans across new lines cleanly ignoring block layouts until encountering the terminal `*---` exclusion exit pointer. |

C-style block allocations (`//`, `/* */`) intentionally trigger a hard panic fallback via the active parser verification steps to explicitly instruct the programmer about syntax migration formats into native XCX standards.

---

## String Interning Mechanism

The lexical scanner accepts the input array and utilizes an internal `&mut Interner` context object, injected repeatedly over `next_token` procedures. Tokens that hold pure text sequences natively (e.g. `Identifier`, `StringLiteral`, `RawBlock`, `Tag`) push string tracking away from heavy memory manipulation patterns toward `StringId` values rather than dynamically allocated `String` representations. A `StringId` acts as a lightweight `u32` array index directly tied to the overarching tracking compiler namespace accessed purely efficiently via `Interner::lookup(id)`.

This procedural cache shares functionality seamlessly with the parser and semantic expanders, enabling fast syntax validation queries using pure integer comparisons, creating O(1) tracking arrays over the lifetime of the pipeline.

---

## Lexer Error Handling

The lexical parser consciously avoids complex `Result` nesting during its extraction pass. Unknown characters and failed verifications drop instantly out of standard loops, returning a flag under `Unknown(char)`, where standard modifier fallbacks execute pipeline triggers through the main `Reporter` class structure to yield explicit syntax error communications natively. Specifically, panics generated due to the misuse of C-style comment syntax include distinct error handling explanations embedded to streamline developer adoption.