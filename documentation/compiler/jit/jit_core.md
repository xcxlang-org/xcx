# XCX JIT Engine: Core Architecture and Pipeline

The XCX JIT Compiler uses Cranelift as its backend to dynamically compile VM functions (bytecode chunks) into native x86_64 machine code. Compilation is per-function ("method JIT"): a chunk is compiled once its call count reaches the warmup threshold (`VM::jit_threshold`, default 50), triggered from `Executor::check_jit_warmup` and the FFI entry warmup path in `vm/core/jit_helpers.rs`. Fibers execute through the interpreter; a former fiber-segment JIT and a trace (hot-loop) compiler existed in earlier revisions but were unreachable and have been removed.

---

## Cranelift Module Integration

The JIT engine is represented by the `JIT` struct (`src/jit/jit.rs`). It acts as a wrapper around Cranelift's `JITModule` and compiler context to orchestrate code generation, thread-safe function definition, and FFI linking.

```rust
pub struct JIT {
    pub(crate) ctx: codegen::Context,
    pub(crate) module: JITModule,
    pub(crate) in_progress: std::collections::HashSet<usize>,
}
```

### Method Compilation Pipeline
Compilation of one function is initiated by `JIT::compile_method` (implemented in `src/jit/compiler_method.rs`):
1. **Clear Context:** Instantiates/clears JIT function generation state (`module.clear_context`).
2. **Signature Verification:** Builds and validates the target function's calling convention.
3. **Register Pre-Analysis:** Sweeps the chunk's bytecode to build pointer tracking maps, type tag inference grids, and liveness regions (`src/jit/analysis.rs`).
4. **Symbol Importation:** Maps FFI symbols into the Cranelift internal function representation (`SymbolRegistry::import_in_func`).
5. **Cranelift IR Builder Execution:** Loops over bytecode, generating Cranelift IR for each operation.
6. **Code Generation:** Emits compiled machine code into executable memory using Cranelift's JIT infrastructure.

---

## Function ABI & Calling Conventions

Modern compilers optimize execution speed by defining custom ABIs for local paths while exposing standard APIs for FFI interaction. The JIT uses two distinct compilation strategies to reduce stack parameter unpacking overhead.

### Parameters Map
A JIT-compiled function conforms to the following ABI:
- **`out_ptr`:** Pointer to a 16-byte stack slot where the 64-bit bits and 64-bit type tag representing the returned `Value` are written.
- **`locals_ptr`:** Pointer to the VM register/locals array.
- **`globals_ptr`:** Pointer to the VM globals array.
- **`consts_ptr`:** Pointer to the constants list.
- **`vm_ptr`:** Pointer to the active `VM` context.
- **`executor_ptr`:** Pointer to the current `Executor` controller.
- **`shutdown_ptr`:** Pointer to the VM shutdown state flag.

```rust
// Standard JIT FFI Signature in cranelift
sig.params.push(AbiParam::new(ptr_type)); // out_ptr
sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
sig.returns.push(AbiParam::new(types::I32)); // execution status (0: ok, 1: error)
```

### JIT-to-JIT Direct Call & Local Call Optimizations

To minimize interpreter re-entry and FFI overhead on function calls, the JIT implements two optimization routes:

#### 1. Local Recursive Calls (Wrapper vs. Inner Model)
For internal self-recursive or local segment calls, the compiler generates two nested Cranelift functions:
- **Inner Function (`Linkage::Local`):** Receives parameters directly via register-based CPU ABIs (pairs of `(bits_reg, tag_reg)`) instead of reading them from the `locals_ptr` stack frame.
- **Outer Wrapper (`Linkage::Export`):** Serves as the FFI bridge for the interpreter. It unpacks arguments from `locals_ptr`, executes the local inner function, records the resulting `Value` back into `out_ptr`, and returns the frame status.

**Recursion depth guard:** before performing a local recursive call, the compiler emits a native comparison of the call-depth counter (stored on the `Executor`) against a fixed limit of `800`, using `IntCC::SignedGreaterThanOrEqual`. On overflow, execution branches to a dedicated block that spills registers, invokes the `xcx_jit_check_recursion` FFI helper to report the failure, and returns a halt status — the failure path itself is not inlined at each call site, keeping the guard's generated code compact. On the non-overflow path the depth counter is incremented before the recursive call and restored to its prior value afterwards.

#### 2. Cross-Function Direct Call Dispatch
When calling another function (`func_idx != ctx.self_func_idx`), the compiler audits the callee's JIT status:
- **Fast-Path (Direct Call):** If the target function has already been JIT-compiled (callee's `jit_ptr` is non-null), the compiler emits a Cranelift `call_indirect` instruction directly to that memory location. The compiler increments `call_depth` check-bounds, moves the VM register stack pointer (`stack_ptr`) forward by `callee_chunk.max_locals` to frame local scopes, and restores them on return. Global variables are reloaded from memory after the call only when the callee's static analysis marks it as heap-touching (`callee_uses_heap`); a callee known to be pure math skips the reload.
- **Slow-Path Fallback:** If the callee is uncompiled (`jit_ptr` is null), processing falls back to the interpreter interface via the `xcx_jit_call_recursive` FFI helper.

```
                  ┌──────────────────────────────────────────────┐
                  │            Call Dispatch Decision            │
                  └──────────────────────┬───────────────────────┘
                                         │
                        Calley JIT compiled? (jit_ptr != null)
                                         │
                        ┌────────────────┴────────────────┐
                        ▼ YES                             ▼ NO
            ┌──────────────────────┐          ┌──────────────────────┐
            │ Fast-Path Direct     │          │ Slow-Path Fallback   │
            │ Cranelift            │          │ FFI helper calls     │
            │ call_indirect        │          │ xcx_jit_call_r       │
            └──────────────────────┘          └──────────────────────┘
```

---

## Symbols Registry

To let JIT-compiled code interact with complex runtime subsystems, a comprehensive static mapping is maintained inside `src/jit/symbols/mod.rs`. The `SymbolRegistry` declares and registers external Rust symbols prefixed with `xcx_jit_*` with Cranelift.

Through this registration, JIT-generated Cranelift IR calls external routines for:
- Reference counting management (`xcx_jit_inc_ref`, `xcx_jit_dec_ref`).
- Memory allocation and initialization of `Array`, `Map`, `Set`, and `Table` instances.
- Core FFI library lookups (string manipulation, JSON serialization, date parsing).
- I/O and network layers (`net_call`, `net_request`, `terminal_move`, `terminal_cursor`).
- Deep recursion checks (`xcx_jit_check_recursion`, `xcx_jit_dec_recursion`).
- Bounds-checked collection fallbacks (`xcx_jit_array_get_bool` and equivalents), invoked when a fast-path bounds check on `Array`/`BoolArray` element access fails — see `compiler/jit/jit_codegen.md`.

### Macro-driven Declarations (`src/jit/symbol_macros.rs`)
To avoid manually repeating binding declarations, the compiler uses the `declare_jit_symbols!` macro. This macro:
1. Generates the fields of the `SymbolRegistry` struct (declaring `FuncId` imports for Cranelift).
2. Generates the fields of the `ImportedSymbols` struct (defining `FuncRef` function call reference mappings).
3. Automates the linkage imports declaration block inside `SymbolRegistry::new`.
4. Automates the declaration of target function imports in each compiled function inside `SymbolRegistry::import_in_func`.