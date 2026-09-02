# Compiler — Expressions

The expression compiling sub-system is responsible for lowering `Expr` nodes into bytecode. It uses a series of specialized handlers under `src/compiler/compile_expr/` to evaluate operators, instantiate collections, and resolve function, method, and module calls.

---

## Module Layout

```
src/compiler/compile_expr/
├── mod.rs          — compile_expr entry point
├── access.rs       — GetIndex, GetMember, JsonBind, RowGet
├── binary.rs       — Arithmetic, Concatenation, and Set operations
├── unary.rs        — Not, Neg (Minus)
├── leaf.rs         — Literals, Identifiers, Lambdas, Closures
├── collection.rs   — Collections (Array, Set, Map, Table, DB, Range)
├── call.rs         — MethodCalls, FunctionCalls, ModuleCalls, Terminal
└── control.rs      — Expressions that act as control flow (Yield)
```

---

## Expression Compilation Pipeline

```rust
pub fn compile_expr(&mut self, expr: &Expr, ctx: &mut CompileContext) -> u8
```

Every method in this directory adheres to this signature. They take an AST `Expr`, emit the corresponding `OpCode` instructions, and return a `u8` representing the local register index where the evaluation result is stored.

Because the XCX compiler maps evaluation directly to local registers (instead of a stack machine), nested expressions continuously call `push_reg()` to allocate temporary destination slots, compile their values, emit their processing instruction, and then `pop_reg()` the temporaries.

---

## Calls & External Modules (`call.rs`)

### Function Calls

Function calls evaluate all arguments sequentially. Argument evaluation results are moved sequentially into contiguous register block `base..base + arg_count`. The compiler emits `Call { base, arg_count, func_idx }`. For fibers, it emits `FiberCreate`. 

Built-in type casting operates differently and is optimized away into `CastInt`, `CastFloat`, `CastString`, and `CastBool` without function call overhead.

### Lambda Capture Operations

If a lambda expression captures variables from its enclosing environment, `collect_captures` (`src/compiler/upvalue.rs`) computes the captured set. The captured values are passed as consecutive leading arguments: either moved into the argument registers at a call site (`compile_expr/call.rs`) or pre-bound as leading locals of a sub-compiler whose flat-local map already contains them (`compile_expr/control.rs`, `compile_query.rs`). The lambda itself compiles to a plain function value (`Value::from_function` + `LoadConst`); there is no separate closure object or closure opcode.

### Method Calls & Receiver Dispatch

When compiling `value.method(...)`, the compiler identifies the receiver type. XCX provides massive standard library features mapped to dynamic methods.
### Module Method Calls (Static)
Identifiers like `net`, `json`, `env`, `crypto`, `date`, `store`, `perf`, `input`, and `terminal` are detected and compile strictly into specialized opcodes (e.g. `HttpCall`, `JsonParse`, `StoreWrite`, `DateNow`, `CryptoHash`, `PerfMs`).

### Inline Lambda & Query Compilation (`compile_query.rs`)
**Massive Optimization**: If the compiler encounters `.where(expr)` with exactly one argument that is **not** a syntactic lambda (the row variable is implicit, e.g. `.where(age > 18)`), it does **not** compile it as a standard function call. (A `Lambda` argument falls back to the standard call path with its captures appended as leading arguments.)
Instead:
1. It analyzes the lambda AST to find captured variables.
2. It pushes a completely independent sub-chunk (marked as `is_table_lambda = true`) directly into the `functions` array.
3. It emits a sequence of `Move` instructions to manually load ONLY the captured variables onto the runtime stack.
4. It emits a highly specialized `MethodCall { kind: Where }` instruction bypassing standard invocation overhead, tightly binding the lambda directly to the runtime SQL/Table execution engine.

### Standard Method Calls
- **Mapped Methods (Dynamic Enum)**: Standard collection manipulation (`push`, `len`, `sort`, `keys`) uses `MethodKind`. The compiler looks up the enum variant through `mapping::map_method_kind` and compiles to a lightweight integer `MethodCall { kind }`.
- **Custom Methods (Dynamic String)**: Unknown/Custom methods are compiled as `MethodCallCustom`, passing the name as a string constant. Used specifically for dynamic JSON path indexing and row column resolution.

### Terminal Commands

`.terminal` command expressions (`.clear`, `.exit`, `.run(cmd)`, `.raw`, `.normal`/`.cooked`, `.cursor on/off`, `.move(x,y)`) are statically compiled down into specific `Terminal*` variants, skipping all method lookup logic.

---

## Collections (`collection.rs`)

- **Struct Initialization**: Arrays, Sets, Maps are compiled by pushing every element into a contiguous block of registers. Emits `ArrayInit`, `SetInit`, or `MapInit` with a `count` offset mapping.
- **Ranges**: Set literals bounded by constraints (`set:N{1,,10 @step 2}`) are lowered immediately into `SetRange` native loop instructions rather than pre-allocating an array.
- **Random Generators**: Resolves `random.int` and `random.float`. Handles optional argument bounds by assigning dummy values to the step arguments.

---

## Access & Member Iteration (`access.rs`)

- **Dot Notation (`obj.field`)**: Compiles to `GetMember { name_idx }` only for a fixed property list (`length`, `year`, `month`, `day`, `hour`, `minute`, `second`, `affected`, `insertId`, `status`, `ok`, `error`); known method names compile to a lightweight `MethodCall { kind }`, and anything else to `MethodCallCustom` with the name as a string constant.
- **Bracket Notation (`obj[pos]`)**: Compiles to `MethodCall { kind: Get }`. (The `GetIndex` opcode is emitted only by set-iteration loops.)
- **JSON Get+Push (`obj.get(path).push(val)`)**: The call compiler matches this exact pattern and folds it into a single `JsonFastGetPush` instruction (`call.rs`). There is no chain-compression of general deep-access paths.
- **Table Row Access**: the AST path does not emit `RowGet`; row fields resolve through `MethodCallCustom` string dispatch at runtime.

---

## Operators (`binary.rs` & `unary.rs`)

Standard translation of left and right expressions. Operator token kinds map directly to matching `OpCode` variants (`Add`, `Sub`, `Equal`, `Has`). Set operation tokens (`UNION`, `\`, `⊕`) compile natively to `SetUnion`, `SetDifference`, etc. Logical conditions like `&&` (`And`) and `||` (`Or`) compile natively into their boolean equivalents. `++` compiles to `IntConcat`; `::` compiles to `MapInit { count: 1 }` (a single-pair map constructor), not a method call.
