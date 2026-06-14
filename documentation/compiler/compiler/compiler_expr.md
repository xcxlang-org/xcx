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

### Closure Operations

If an inline `fn` or lambda expression captures variables from its enclosing environment, the compiler instead produces a `MakeClosure` opcode, generating an inline closure execution context. Call operations on identifiers bound to closures will evaluate as standard variables and use the `CallClosure` instruction for dynamic dispatch on the captured environment.

### Method Calls & Receiver Dispatch

When compiling `value.method(...)`, the compiler identifies the receiver type. XCX provides massive standard library features mapped to dynamic methods.
### Module Method Calls (Static)
Identifiers like `net`, `json`, `env`, `crypto`, `date`, `store` are detected and compile strictly into specialized opcodes (e.g. `HttpCall`, `JsonParse`, `StoreWrite`, `DateNow`, `CryptoHash`).

### Inline Lambda & Query Compilation (`compile_query.rs`)
**Massive Optimization**: If the compiler encounters `.where(fn(x) ...)` or `.where(x -> ...)` with exactly one lambda argument, it does **not** compile it as a standard function call. 
Instead:
1. It analyzes the lambda AST to find captured variables.
2. It pushes a completely independent sub-chunk (marked as `is_table_lambda = true`) directly into the `functions` array.
3. It emits a sequence of `Move` instructions to manually load ONLY the captured variables onto the runtime stack.
4. It emits a highly specialized `MethodCall { kind: Where }` instruction bypassing standard invocation overhead, tightly binding the lambda directly to the runtime SQL/Table execution engine.

### Standard Method Calls
- **Mapped Methods (Dynamic Enum)**: Standard collection manipulation (`push`, `len`, `sort`, `keys`) uses `MethodKind`. The compiler looks up the enum variant through `mapping::map_method_kind` and compiles to a lightweight integer `MethodCall { kind }`.
- **Custom Methods (Dynamic String)**: Unknown/Custom methods are compiled as `MethodCallCustom`, passing the name as a string constant. Used specifically for dynamic JSON path indexing and row column resolution.

### Terminal Commands

`.command` expressions (`.clear`, `.exit`, `.runcmd()`, `.raw`, `.move(x,y)`) are statically compiled down into specific `Terminal*` variants, skipping all method lookup logic.

---

## Collections (`collection.rs`)

- **Struct Initialization**: Arrays, Sets, Maps are compiled by pushing every element into a contiguous block of registers. Emits `ArrayInit`, `SetInit`, or `MapInit` with a `count` offset mapping.
- **Ranges**: Set literals bounded by constraints (`set:N{1,,10 @step 2}`) are lowered immediately into `SetRange` native loop instructions rather than pre-allocating an array.
- **Random Generators**: Resolves `random.int` and `random.float`. Handles optional argument bounds by assigning dummy values to the step arguments.

---

## Access & Member Iteration (`access.rs`)

- **Dot Notation (`obj.field`)**: Compiles to `GetMember { name_idx }` using a pre-interned constant pool string.
- **Bracket Notation (`obj[pos]`)**: Compiles to `GetIndex`.
- **JSON Deep Property Access (`obj.nested[1].data`)**: Analyzes the AST tree. If it detects a chain of JSON accesses, it compresses it into a high-performance, pipelined `JsonFastGetPush` chain to avoid allocating intermediate strings natively.
- **Table Row Queries (`row.column`)**: Analyzes table metadata (if known at compile time) and optimizes member accesses to integer `RowGet { col_idx }` offsets instead of hash map string lookups.

---

## Operators (`binary.rs` & `unary.rs`)

Standard translation of left and right expressions. Operator token kinds map directly to matching `OpCode` variants (`Add`, `Sub`, `Equal`, `Has`). Set operation tokens (`UNION`, `\`, `⊕`) compile natively to `SetUnion`, `SetDifference`, etc. Logical conditions like `&&` (`And`) and `||` (`Or`) compile natively into their boolean equivalents. For AST nodes representing concatenation (`++`, `::`), `binary.rs` selects the specifically optimized `IntConcat` or standard `MethodCall { kind: Join }` dispatch.
