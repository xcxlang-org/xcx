# Compiler — Core & Environment

The bytecode compiler is responsible for translating the AST (Abstract Syntax Tree) into a flattened array of `OpCode` instructions wrapped in a `Chunk`. It runs after the parser, expander, and semantic checker. The core compiler structures and environment abstractions are defined in `src/compiler/`.

---

## Module Layout

```
src/compiler/
├── mod.rs               — re-exports
├── compiler.rs          — Compiler, FunctionCompiler, CompileContext
├── globals.rs           — global variable resolution
├── constant_pool.rs     — compile-time constant deduplication
├── defaults.rs          — default type value generators
├── patch.rs             — backpatching for jump instructions
├── upvalue.rs           — AST upvalue capturing structures
... (other modules covered in dedicated docs)
```

---

## `Compiler` Struct

```rust
pub struct Compiler {
    pub globals: HashMap<StringId, usize>,
    pub func_indices: HashMap<StringId, usize>,
    pub functions: Vec<Arc<Chunk>>,
    pub constants: Vec<Value>,
    pub string_constants: HashMap<Vec<u8>, usize>,
    pub numeric_constants: HashMap<(u64, u64), usize>,
}
```

The top-level `Compiler` orchestrates the entire translation pipeline. It owns the global environment (functions, variables, constants) that spans the entire program.

- `globals`: Maps global variable names (`StringId`) to an integer index `0..N`.
- `func_indices`: Maps function/fiber names to their index in the `functions` array.
- `functions`: Accumulates JIT-ready `Chunk`s representing compiled functions/fibers.
- `constants`: A deduplicated pool of compile-time constants (strings, floats, ints, maps) used by `OpCode::LoadConst`.

### Pre-Registration (`globals.rs`)

Before any bytecodes are emitted, `Compiler::compile` calls `globals::register_globals_recursive`. This performs a pre-pass over the AST to assign indices to:
1. Functions and fibers (`func_indices`, pushing skeleton `Chunk`s into `functions`).
2. Global variables (`globals`).
3. Database instances.

Built-in modules (`json`, `date`, `random`, `store`, `input`) are hardcoded as global indices `0..4` (inclusive, totally 5 elements). The compiler synthesizes bytecode in the main chunk to preload these globals via `LoadConst` and `SetVar`.

---

## `CompileContext`

```rust
pub struct CompileContext<'a> {
    pub constants: &'a mut Vec<Value>,
    pub string_constants: &'a mut HashMap<Vec<u8>, usize>,
    pub numeric_constants: &'a mut HashMap<(u64, u64), usize>,
    pub functions: &'a mut Vec<Arc<Chunk>>,
    pub func_indices: &'a HashMap<StringId, usize>,
    pub globals: &'a HashMap<StringId, usize>,
    pub interner: &'a mut Interner,
}
```

A short-lived mutable view passed to compilation routines. It decouples the global state from specific function compilations, allowing recursive or independent function compilation without borrowing issues.

---

## `FunctionCompiler` Struct

```rust
pub struct FunctionCompiler {
    pub bytecode: Vec<OpCode>,
    pub spans: Vec<Span>,
    pub scopes: Vec<HashMap<StringId, usize>>,
    pub next_local: usize,
    pub loop_stack: Vec<(usize, Vec<usize>, Vec<usize>, Option<usize>)>,
    pub parent_locals: Option<HashMap<StringId, usize>>,
    pub captures: Vec<StringId>,
    pub is_main: bool,
    pub is_table_lambda: bool,
    pub max_locals_used: usize,
}
```

Compiles a single function, fiber, or the top-level main program.

- `bytecode` / `spans`: The parallel arrays of instructions and debug spans that will form the final `Chunk`.
- `scopes`: A stack of lexical scopes binding variable names maps to local register indices. Pushed on `{`, popped on `}`.
- `next_local`: The next free local register index in the current frame.
- `max_locals_used`: The high-water mark of `next_local`, dictating the size of the runtime stack frame overhead.
- `loop_stack`: Tracks control flow for `break` and `continue`. Tuple: `(start_ip, unresolved_breaks, unresolved_continues, step_ip)`.

### Register Allocation

The compiler treats arguments and local variables equally as `u8` local registers.
- `push_reg()`: Allocates the next sequential register and bumps `max_locals_used`.
- `pop_reg()`: Reclaims the most recently pushed register.

XCX's compiler uses an unoptimized flat register allocator during this phase; dense optimization and parameter pinning are deferred to the `RegisterManager` pass.

### Compilation Dispatch

`Compiler::compile` iterates over the AST, compiling main statements into the main `FunctionCompiler`, and delegating `FunctionDef` and `FiberDef` to independent `compile_function_helper` calls which spawn fresh `FunctionCompiler` instances.

The resulting output is a tuple: `(MainChunk, ConstantsPool, FunctionsArray)`.

---

## Default Value Generation & Constant Pool Edge Cases

When variables are declared without an explicit value (e.g., `i: count;`), the compiler automatically calls `defaults::get_default_value` and pushes a `LoadConst` instruction. 

**Limitation / Nuance**: For reference types like Arrays, Sets, Maps, and Tables, the default value initialized in the constant pool is a single shared `Arc<RwLock<...>>` instance. The executor runtime must therefore ensure to deeply clone or uniquely instantiate these defaults upon variable assignment, otherwise all uninitialized collections would point to the same memory block. 

### Table Literal Skeletons (`compile_table.rs`)

Defining `table { ... }` literals triggers a highly unique compiler path:
1. The compiler analyzes the schema AST (`columns`).
2. It statically builds an empty `TableObj` containing the exact `VMColumn` types, primary keys, and auto-increments.
3. It inserts this empty, fully-formed table into the constant pool.
4. It emits `TableInit` holding the index to this "skeleton". The runtime simply clones the skeleton rather than parsing schema metadata, heavily boosting JIT speed.

---

## Upvalue Capture & Closures (`upvalue.rs` / `scope_tracker.rs`)

**Constraint**: The XCX compiler captures variables flatly. 
`scope_tracker.rs` uses a mechanism where if a local variable is not found in the current frame, it checks `parent_locals`. If it exists there, it is pushed into a `captures` vector and implicitly assigned a local slot mapped as `1 + captures_index`. 
This results in a tightly bounded one-level closure structure; deeply nested closures require manual parameter propagation or run the risk of losing scope tracking.

---

## Backpatching (`patch.rs`)

When compiling jumps (e.g., `if`, `while`, `or`, `and`), the jump target IP is not known until the body is compiled. The compiler emits a placeholder instruction, records its IP, and backpatches it later.

`emit_jump(OpCode::Jump { target: 0 }) -> usize` returns the IP.
`patch_jump(ip)` overwrites the `target` parameter of the instruction at that IP with the current end of `bytecode`.
