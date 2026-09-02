# Sema — Expression & Method Resolution

The semantic checker evaluates dynamic expressions across primitive combinations and expansive built-in methodologies. XCX is fundamentally un-imported; thus, its semantic method checking dictates implicit boundaries entirely.

---

## Operator Checking (`check_expr_op.rs`)

Binary and Unary traversal rules follow a precise subset:

- **String Aggregation**: Using `++` specifically prioritizes implicit string cascading across the majority of variable types natively, converting internal structures rapidly to strings without `TypeMismatch` halting.
- **Mathematical Date Overloading**: Implementing `-` naturally against a `Type::Date` array forces explicit resolution evaluating exact numerical timestamp deltas natively as `Type::Int`.
- **Set Theory Logic**: Built-in logical bounds operators (Union, Intersection, SymDifference, Difference) are rigidly verified to function exclusively when both the left and right operands identically mirror an exact matched `Type::Set(X)` variant architecture.
- **The `has` Operator**: A pseudo-binary mechanism. It is fundamentally audited to check whether a payload implicitly fits inside nested types traversing native Collections (`Array`, `Set`, `String`).

---

## Literals & Structure Assertions (`check_expr_literal.rs`)

- **Table Literal Mapping**: Strictly executes schema enforcement against un-rolled positional rows. Given a `Table` literal bound by `@schema`, it iterates across every nested column payload, confirming length (`TableRowCountMismatch`) and perfect sequence data alignments, purposefully bypassing variables marked `@auto`.
- **Set Generation**: Validates explicit step generations in range literals. While a `Type::Set(S)` (String) can be generated natively without steps, ranges across `Set(C)` are validated specifically ensuring step parameters mathematically generate against integer offsets to maintain continuous logical validity.

---

## Standard Method Resolutions (`check_expr_methods.rs`)

The core file for all dynamic method validations without modules.

- Massive exhaustive `match` logic evaluates against native primitives natively to execute methods safely:
  - Strings resolving `.split()` push back `Type::Array(String)`; `.trim()` returns `Type::String`; `.startsWith()`/`.endsWith()` return `Type::Bool`.
  - Native numbers safely matching `.toStr()`, bypassing rigid string casts dynamically.
  - Collections bind methods like `.push()`/`.add()`/`.insert()` without dedicated argument-count enforcement (Sets and Maps have no `push` at all — their insert methods are `add` and `insert`/`set`/`update`).

## Module Call Execution (`check_module_call.rs`)

Handles direct module pathing evaluating namespaces such as `net.`, `crypto.`, `store.`, and `json.`. The semantic checker validates method signatures securely natively.

- `json.parse()` dictates string payload evaluations.
- `store.zip()` (like write/append/delete/isDir/mkdir/unzip) only generically checks its argument expressions and returns `Type::Bool` — no static String-path enforcement exists.

## Database & Query Bindings (`check_query.rs` & `check_table.rs`)

- Identifies native queries evaluating DB calls. Eleven I/O methods set `last_expr_was_db_io = true`: `fetch`, `insert`, `save`, `push`, `query`, `queryRaw`, `remove`, `truncate`, `exec`, `sync`, `drop`.
- Inside a fiber body, a database I/O expression must be wrapped in a `yield` — otherwise the error "Database I/O method '{}' must be yielded inside a fiber" blocks compilation. Yielding a DB-I/O expression outside a fiber is conversely allowed.
- Argument mapping cascades specifically track Named arguments cleanly. If positional assignments evaluate properly, trailing assignments securely cascade into dictionary mappings verifying `@columns` without overlap (`Duplicate named argument`).
