# XCX Compiler Error and Diagnostics System

The XCX compiler incorporates a structured diagnostics pipeline designed to detect, track, and format diagnostic logs for syntax errors, semantic type mismatches, and execution halts.

---

## Data Structures (`src/error/`)

Core structures categorize error locations, diagnostics severity levels, and print properties.

### Spans (`span.rs`)
Position details are wrapped inside the `Span` structure:
```rust
pub struct Span {
    pub line: usize, // 1-based line number in source file
    pub col: usize,  // 0-based column offset in line
    pub len: usize,  // length of token to highlight
}
```

### Diagnostic Definitions (`diagnostic.rs`)
Diagnostics map individual report instances via `Diagnostic` carrying error severity types:
- **`Severity` (enum):** `Severity::Error`, `Severity::Warning`, or `Severity::Note`.
- **`Diagnostic` (struct):** Registers warning/error message strings (`message`), the target `Span`, a severity flag, and optional error code slices (`code: Option<&'static str>`).

### Diagnostic and Error Classifications

The compiler maps diagnostics using three distinct series of error codes:

1. **Placeholder Code Definitions (E-Series, `E101`–`E304`):**
   Defined in `src/error/codes.rs` for future compiler and VM VM-level integration, these codes are currently **unused placeholders**. The primary compiler parser currently raises raw text errors instead of instantiating these variants:
   - **Parser & Syntax Checks (100–199):** `E101` (Unexpected Token), `E102` (Missing Semicolon), `E103` (Malformed Expression)
   - **Semantic Errors (200–299):** `E201` (Variable already defined), `E202` (Variable not found), `E203` (Type mismatch), `E204` (Function not found)
   - **VM / Runtime Errors (300–399):** `E301` (Stack overflow), `E302` (Invalid OpCode), `E303` (Table Schema mismatch), `E304` (Index out of bounds)

2. **Active Semantic Logic Codes (S-Series, `S101`–`S302`):**
   Emitted by the compile-time semantic checking phase (`src/sema/error/error_kind.rs`). See details in the [Semantic Type and Logic Verification](#semantic-type-and-logic-verification-srcsemaerror) section below.

3. **Active VM/Runtime Codes (R-Series, `R103`–`R501`):**
   Active diagnostic indicators printed to stderr or triggered as runtime exceptions when logical dynamic assertions fail during script execution. These range from bounds errors to IO exceptions:
   - **R103**: Input Type Mismatch (e.g. expected type, got float/string at CLI read).
   - **R303**: Access Index Out of Bounds (Table column indices, String slice offsets, or Array bounds).
   - **R304**: Missing Column references.
   - **R305**: Invalid JSON string parsed.
   - **R306**: Finished Fiber next access / Invalid JSON push.
   - **R307**: `.replace()` method empty 'from' search.
   - **R401**: Database connectivity / missing SQL table exceptions.
   - **R403**: SQL database query/prepare execution errors.
   - **R404**: Binding JSON path not found / DB row drop exception.
   - **R405**–**R410**: Relational SQL table constraints / insertion mismatches.
   - **R440**–**R443**: Terminal raw mode setup / cursor out of bounds / console inputs failure.
   - **R501**: Unsupported VM instruction/method call index type.


---

## Preprocessor and Lexer Validation Errors

### Include File Expansion (`src/frontend/parser/expander.rs`)
The preprocessor (`Expander`) resolves `include "..."` directives before parsing is finalized. If resolved paths fail, it throws formatted errors:
- **File Not Found:** If standard paths or include directory trials fail, returns `File not found: <filename> (tried: <paths>)`.
- **Circular Inclusion:** Detects nested loop boundaries using call tracking sets, raising `Circular dependency detected: <filename>`.

### C-Style Comment Validation (`src/frontend/lexer/lexer.rs`)
XCX strictly requires `---` for single-line comments and `---` ... `*---` blocks (which are initiated by placing `---` alone on a line and terminated by `*---` on a line of its own). If the lexer scans C-style comment notations (`//` or `/*`), it raises a hard compiler abort panic:
```text
[XCX Error] C-style comments (// or /* */) are NOT supported in XCX.
Use '---' for single-line comments.
```

---

## VM Execution and Arithmetic Halts (`src/vm/`)

### VM Call and Stack Overflows (`src/vm/core/executor.rs`)
During bytecode execution, the VM monitors stack limits:
- **Stack Boundaries:** If `stack_ptr` exceeds the execution stack pool limits, the VM outputs `ERROR halt: XCX VM Stack Overflow` and increments the `error_count` counter.
- **Recursion Limits:** The call hierarchy stack tracks nest levels. If function calls nested frame limits exceed 800 trace frames, execution aborts: `ERROR halt: Recursion limit exceeded (800 frames)`.
- **Invalid Method Invocation:** Unrecognized method tag kinds yield direct failures: `R501: Invalid method kind index: <kind>`.

### Safe FFI Error Reporting
To support safe and consistent exception reporting from JIT-compiled code and FFI modules (such as Array boundary checks `R303`), the runtime uses a thread-local pointer model:
- **`ACTIVE_VM: Cell<*const VM>`:** A thread-local cell linking runtime calls back to the executing VM context.
- **`ActiveVmGuard`:** An RAII guard managing `ACTIVE_VM` registration across VM execution sweeps (`run_frame`, `handle_call_no_jit`, and `dispatch_jit_call`).
- **`increment_error_count()`:** An FFI helper callable from JIT-emitted code or array bounds checks to safely increment runtime error counters, eliminating JIT compile-time execution divergence.

### Arithmetic Exception Handlers (`src/vm/core/step/arith.rs`)
Mathematical divisions evaluate bounds to preempt CPU crashes:
- **Division by Zero:** Triggers `ERROR halt: division by zero`, increments the VM error counter, and issues `OpResult::Halt`.
- **Modulo by Zero:** Triggers `ERROR halt: modulo by zero`, increments the VM error counter, and issues `OpResult::Halt`.

---

## Visual Diagnostic Reporter (`src/error/reporter.rs`)

The `Reporter` structure frames source buffers to generate colorized, framed terminal diagnostics output.

### CLI Formatting Configuration
Using ANSI colors (`\x1b[31;1m` for Red text, `\x1b[33;1m` for Yellow highlight, and `\x1b[36m` for Cyan boundaries), the reporter formats logs:
1. **Header Block:** Renders the severity level (e.g. `ERROR:` or `WARNING:`) followed by compilation messages.
2. **Context Framing:** Prints target source lines using a centered column line gutter:
   ```text
      3 |  i: val = "hello" + 42;
   ```
3. **Squiggle Highlights:** Creates underline indicators positioned directly offset from the start column:
   - For single-character error contexts, prints a pointing arrow symbol (`^`).
   - For multi-character spans, mirrors token lengths using wave squiggles (`~`).

---

## Semantic Type and Logic Verification (`src/sema/error/`)

The semantic check module identifies compile-time logic checks:

### Error Wrappers (`type_error.rs`)
Semantic type errors pair type errors payload (`TypeErrorKind`) with its corresponding ast source `Span` under the `TypeError` structure.

### Error Catalogs (`error_kind.rs`)
The `TypeErrorKind` enum maps precise semantic logic failures:

| Code | Error Category | Details |
|---|---|---|
| **`[S101]`** | Undefined Variable | Referencing an undeclared identifier. |
| **`[S102]`** | Redefined Variable | Declaring an identical variable in the same block. |
| **`[S103]`** | Type Mismatch | Expected and actual types mismatch. |
| **`[S104]`** | Invalid Binary Op | Attempting operators between non-matching operand types (e.g. adding INT to STRING). |
| **`[S105]`** | Const Reassignment | Attempting to mutate value bound to a `const` modifier. |
| **`[S106]`** | Break Outside Loop | Issuing a loop exit `break` statement outside loop blocks. |
| **`[S107]`** | Continue Outside Loop | Issuing `continue` directives outside loop contexts. |
| **`[S108]`** | Index Not Supported | Attempting index access on types that do not support indexing. |
| **`[S109]`** | Property Not Found | Accessing a property field not defined in structure type mappings. |
| **`[S110]`** | Method Not Found | Accessing methods not defined on the target data structure. |
| **`[S111]`** | Invalid Argument Count | Invoking functions with argument metrics that don't match the signature. |
| **`[S208]`** | Yield Outside Fiber | Utilizing `yield` syntax outside a fiber body representation. |
| **`[S209]`** | Fiber Type Mismatch | Issuing `yield expr;` return paths inside empty void fiber blocks. |
| **`[S210]`** | Return Mismatch In Fiber | Typed fibers must use `return expr;` instead of a blank `return;`. |
| **`[S211]`** | Iterate Over Void Fiber | Performing `for` loop iterations over empty void fibers. |
| **`[S212]`** | Run Typed Fiber | Invoking `.run()` methods on typed fibers (which require generator iterator loops instead). |
| **`[S301]`** | Predicate Name Collision | Conflicting naming parameters between local variables and query columns in `.where()` predicates. |
| **`[S302]`** | Table Row Count Mismatch | Inserting row values with item counts that don't match the table columns schema definition. |
