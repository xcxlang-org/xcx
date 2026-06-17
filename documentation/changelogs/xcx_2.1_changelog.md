# Changelog

## XCX 2.1.0

---

### Virtual Machine (VM) changes

The virtual machine architecture was reworked for thread safety. Internal collection value types (`Array`, `Set`, `Map`, `Table`, `Json`, `Fiber`, `Row`) were rewritten from `Rc<RefCell<_>>` to `Arc<RwLock<_>>` from the `parking_lot` crate. This change allows values to be shared across threads without compromising data consistency.

The `VM` struct moved from a mutable model (`&mut VM`) to one based on `Arc<VM>`. Globals and the HTTP server registry are now stored behind `Arc<RwLock<_>>`, enabling safe shared access.

The execution context (`VMContext`) was replaced by `SharedContext` — a new struct implementing `Clone` that takes ownership of constants and functions instead of borrowing references. This removes lifetime constraints when invoking the VM from multiple places.

The internal `Executor` is no longer a struct holding lifetime-bound references to the `VM`. The call stack, depth counter, and the `fiber_yielded` flag were moved directly into `Executor`.

The `run` method now takes `Arc<Self>` and a `FunctionChunk` instead of a bytecode slice, reflecting the new ownership model.

---

### Method dispatch — new `MethodKind` enum

A `MethodKind` enum was introduced covering all built-in methods for collections, strings, dates, arrays, maps, files, and fibers. The `MethodCall` opcode signature changed from `MethodCall(usize, usize)` to `MethodCall(MethodKind, usize)`, eliminating runtime method-name lookup and enabling static matching at compile time.

A new `MethodCallCustom(usize, usize)` opcode handles user-defined methods that aren't part of the built-in set.

The separate `FiberNext`, `FiberRun`, `FiberIsDone`, and `FiberClose` opcodes were removed — their semantics are now covered by `MethodCall(MethodKind::Next | Run | IsDone | Close, _)`.

---

### Compiler — constant deduplication and position tracking

`Compiler` and `CompileContext` now hold a `string_constants: HashMap<String, usize>`. Every time a `String` constant is added, it first checks whether an identical value already exists in the pool — if so, it returns the existing index. This limits constant pool size in programs that heavily reuse string literals.

`FunctionCompiler` gained a `spans: Vec<Span>` field and an `emit(op, span)` method that records source position information for every emitted opcode. These positions are passed to the VM via `FunctionChunk` (the `bytecode` and `spans` fields are wrapped in `Arc<Vec<_>>`).

A helper method `map_method_kind(&str) -> Option<MethodKind>` was added, mapping method names to enum values at compile time.

---

### Symbol table — parent-child hierarchy

`SymbolTable` gained an optional `parent: Option<&'a SymbolTable<'a>>` pointer and a `new_with_parent` constructor. The `has`, `lookup`, and `is_const` methods now walk the parent chain when a name isn't found in the current scope.

In semantic analysis (`checker.rs`), places that previously cloned the entire symbol table (`symbols.clone()`) were replaced with calls to `SymbolTable::new_with_parent(symbols)`. This applies to function and fiber contexts.

---

### Semantic analyzer — simplified pre-scanning

The `pre_scan_stmts` method no longer recurses into the bodies of conditionals (`if`/`else`), loops (`while`, `for`), or function/fiber definitions. Pre-scanning now only covers declarations at the current level, which eliminates false forward registrations of symbols from nested scopes.

Method signatures taking `&mut SymbolTable` were updated to `&mut SymbolTable<'_>`, in line with the new lifetime parameters.

---

### Scanner (lexer)

`Scanner` changed its internal source representation from `Vec<char>` to `&'a [u8]` (bytes), eliminating the character-collection allocation at initialization and speeding up traversal of the source file. A `char_pos` field was added to track character position independently of byte position.

The scanner now handles multi-byte Unicode operators directly at the byte level: `∪` (union), `∩` (intersection), and `⊕` (symmetric difference) are recognized without first converting to `char`.

---

### Parser

In the array literal parsing method, the branch handling curly braces `{...}` for `Type::Array` types was removed. Array literals now require square brackets `[...]`. The `LeftBrace` branch for untyped containers (`ArrayOrSetLiteral`) remains unchanged.

Parsing of column names in table definitions now uses the `parse_identifier_as_string_id(false)` method instead of matching directly against `TokenKind::Identifier`.

`Parser` and `Scanner` now have explicit lifetime parameters tied to the source (`Scanner<'a>`, `Parser<'a>`).

---

### Signal handling and process shutdown

Added `CTRL-C` signal handling via the `ctrlc` crate. Upon receiving the signal, an atomic `SHUTDOWN` flag (`AtomicBool`) is set, which the VM can check during execution. An informational message is printed on shutdown.

---

### Error messages — source position

Runtime error messages (including division by zero, unknown crypto algorithm, missing environment variable, invalid `@wait` argument) now include the source position in the format `[line: N, col: N]`, generated by the `current_span_info` method.

---

### Dependency changes (`Cargo.toml`)

Added:
- `ctrlc = "3.4"` — system signal handling
- `parking_lot = "0.12"` — efficient synchronization primitives (`RwLock`)

Removed direct `extern crate` declarations for `tiny_http`, `ureq`, `serde_json`, `bcrypt`, `argon2`, and `hex` from `vm.rs` (unnecessary since Rust 2018 edition).

---

### REPL

`repl.rs` now uses `Arc<VM>` instead of a mutable `VM`. The `vm.run` call was changed to `vm.clone().run(main_chunk, ctx)`, in line with the new `VM::run` interface.
