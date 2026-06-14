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
  - Strings resolving `.split()`, `.trim()`, `.startsWith()`, pushing back `Type::Array(String)` natively.
  - Native numbers safely matching `.toStr()`, bypassing rigid string casts dynamically.
  - Collections securely binding methods like `.push()`, validating argument payload counts and expected abstract mapping variants. (Array:1 positional; Sets:1 positional variant; Maps:2 positional bindings).

## Module Call Execution (`check_module_call.rs`)

Handles direct module pathing evaluating namespaces such as `net.`, `crypto.`, `store.`, and `json.`. The semantic checker validates method signatures securely natively.

- `json.parse()` dictates string payload evaluations.
- `store.zip()` requires exact string arguments statically mapping paths natively before execution to avoid runtime file corruptions.

## Database & Query Bindings (`check_query.rs` & `check_table.rs`)

- Identifies native queries evaluating DB calls. I/O executions (`insert`, `save`, `queryRaw`, `sync`) explicitly check whether `last_expr_was_db_io = true`.
- If the statement attempts a database execution securely bound inside a native `@fiber`, it restricts explicit asynchronous evaluation: *Database I/O methodology securely must explicitly be executed outside native non-yielded fiber architectures.* (Compilation block active).
- Argument mapping cascades specifically track Named arguments cleanly. If positional assignments evaluate properly, trailing assignments securely cascade into dictionary mappings verifying `@columns` without overlap (`Duplicate named argument`).
