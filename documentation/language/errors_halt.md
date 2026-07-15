# XCX 4.2 Error Handling and the Halt Instruction

The XCX language utilizes a structured `halt` system to manage runtime errors and semantic (compiler) errors detected during the compilation and analysis phases.

## Halt Levels

| Level          | Behavior | Use Case |
|----------------|----------|----------|
| `halt.alert`   | Prints a warning to stderr; continues execution. | Auxiliary logging, non-critical alerts. |
| `halt.error`   | Prints an error to stderr; aborts the current stack frame (frame recovery). | Recoverable business/logical errors. |
| `halt.fatal`   | Prints an error to stderr; immediately terminates the virtual machine with an error code. | Security violations, data corruption, critical inconsistencies. |

---

## XCX Error Codes Registry

### Semantic Errors (Semantic / Compiler) — Block Compilation
*   **S101** – Undefined variable
*   **S102** – Redefined variable (declared multiple times in the same scope)
*   **S103** – Type mismatch (on assignment or operation)
*   **S104** – Invalid binary operation for the given type
*   **S105** – Const reassignment (attempt to assign a new value to a constant)
*   **S106** – Break outside loop (the `break` keyword used outside a loop context)
*   **S107** – Continue outside loop (the `continue` keyword used outside a loop context)
*   **S108** – Member access on primitive (attempt to access a member/field of a primitive type)
*   **S109** – Method not found on type
*   **S110** – Field not found or wrong arg count (invalid field name or wrong number of parameters in a method call)
*   **S111** – Static analysis/method signature mismatch
*   **S208–S212** – Fiber constraint errors (e.g., `yield` keyword used outside a fiber, invalid fiber instantiation)

### Runtime Errors (Runtime) — Raise `halt.error` or `halt.fatal`
*   **R103** – Input type mismatch (type mismatch on stream input, e.g., expected int, got string)
*   **R303** – Index out of bounds (array or string bounds exceeded)
*   **R305** – Invalid JSON syntax (malformed JSON data passed to parser — triggers `halt.fatal`)
*   **R306** – Invalid push or finished fiber (attempt to call `.next()` on a fiber that has already finished)
*   **R307** – Empty replace pattern (attempt to call `.replace()` on a string with an empty search query)
*   **R401** – DB table or connection error (SQLite) (missing table or error opening database file)
*   **R403** – DB query compiler/execution error (SQL syntax error or query execution failure)
*   **R404** – JSON path not found or DB delete error (record deletion failure, or JSON path not found in structure)
*   **R405** – Sync argument error (expected Table object on `db.sync` call)
*   **R406** – Sync failed (error creating database SQL table)
*   **R407** – Method argument error (expected table name or Table object on DDL call)
*   **R408** – Drop table failed (failed to drop table from database)
*   **R409** – Insert argument error (the first parameter of insert must be a Table)
*   **R410** – Save requires primary key (attempt to save/upsert on a SQL table without a defined `@pk`)
*   **R440** – Terminal mode error (failed to enable raw terminal mode)
*   **R441** – Cursor out of bounds (x/y cursor coordinates exceed allowed range)
*   **R442** – input.key() outside raw mode (invoked `input.key()` without first activating raw terminal mode)
*   **R443** – Failed to read input (hardware/stream error when reading input)
*   **R501** – Invalid method kind index (invalid header for dynamic method call)

### Database Static Errors (Database Static)
*   **S301** – Parameter and column clash (parameter name conflicts with a column name in a table's `where` expression)
*   **S302** – DB Columns count mismatch (column count mismatch when inserting structures)
*   **D401** – Remove without where clause (attempt to perform `db.remove()` / `table.remove()` without a filtering clause)

---

## Diagnostic Usage Examples

```xcx
--- Typical bounds check
if (divisor == 0) then;
    halt.error >! "Division by zero!";
    return 0;
end;
```


## Semantic and Runtime Panics

Certain invalid operations result in an automatic **panic** (equivalent to `halt.fatal` or `halt.error` depending on context):
- **Division by zero** (arithmetic)
- **JSON Parse Failure**: Invalid string in `json.parse()` results in an immediate VM exit.
- **Path Traversal**: Using `..` in `store` methods or absolute paths outside the project root triggers `halt.fatal`.
- **Recursion Depth**: Exceeding 800 frames triggers `halt.error`.


## Offline Error Reference Tool (`xcx doc`)

XCX includes an offline error reference utility located in `lib/doc/` which compiles to `xcx doc`. This tool allows you to search for error explanations and usage examples directly from your terminal.

### Usage

```sh
# Lookup a specific error code's explanation and example
xcx doc S103
xcx doc R305

# List all registered compile-time and runtime error codes
xcx doc list
```

