# XCX JIT Engine: Core Architecture and Pipeline

The XCX JIT Compiler uses Cranelift as its backend to dynamically compile traces of virtual machine bytecode into native x86_64 machine code. 

---

## Cranelift Module Integration

The JIT engine is represented by the `JIT` struct (`src/jit/jit.rs`). It acts as a wrapper around Cranelift's `JITModule` and compiler context to orchestrate code generation, thread-safe function definition, and FFI linking.

```rust
pub struct JIT {
    pub builder_context: FunctionBuilderContext,
    pub ctx: codegen::Context,
    pub module: JITModule,
    pub symbols: Symbols,
}
```

### Trace Compilation Pipeline
Trace compilation is initiated by invoking `JIT::compile`:
1. **Clear Context:** Instantiates/clears JIT function generation state (`module.clear_context`).
2. **Signature Verification:** Validates the target trace's calling conventions.
3. **Register Pre-Analysis:** Sweeps bytecode to build pointer tracking maps, type tag inference grids, and liveness regions.
4. **Symbol Importation:** Maps FFI symbols into the Cranelift internal function representation.
5. **Cranelift IR Builder Execution:** Loops over bytecode, generating Cranelift IR for each operation.
6. **Code Generation:** Emits compiled machine code into executable memory using Cranelift's JIT infrastructure.

---

## Function ABI & Calling Conventions

Modern compilers optimize execution speed by defining custom ABIs for local paths while exposing standard APIs for FFI interaction. The JIT uses two distinct compilation strategies to reduce stack parameter unpacking overhead.

### Parameters Map
A standard JIT-compiled trace conforms to the following ABI:
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

### JIT-to-JIT Local Calls Optimization (Wrapper vs. Inner)
To minimize stack reads/writes on recursive or internal method calls, standard functions with matching target signatures compile as two nested Cranelift functions:

```
                            [ FFI CALL / VM CALL ]
                                       │
                                       ▼
                       ┌──────────────────────────────┐
                       │    Outer Export Wrapper      │
                       │   (method_X, Linkage::Export)│
                       └──────────────┬───────────────┘
                                      │
                                      │ Unpacks registers (locals_ptr)
                                      │ to raw pairs of (bits, tag)
                                      ▼
                       ┌──────────────────────────────┐
                       │       Inner Function         │
                       │ (method_inner_X, Linkage::L.)│
                       └──────────────────────────────┘
                                      ▲
                                      │
                                      │ Direct JIT-to-JIT call
                                      │ (fast register passing)
                                      │
                       ┌──────────────────────────────┐
                       │     Recursive/Local Caller   │
                       └──────────────────────────────┘
```

1. **Inner Function (`Linkage::Local`):** Parses parameters directly using the platform ABI (e.g., pairs of `(bits_reg, tag_reg)`) instead of loading from the `locals_ptr` buffer.
2. **Outer Wrapper (`Linkage::Export`):** Serves as an interface for the virtual machine interpreter. It fetches arguments from `locals_ptr` at offsets, passes them via CPU JIT-to-JIT parameters to the inner function, writes the output `Value` to `out_ptr` and returns the loop execution status.

---

## Symbols Registry

To let JIT-compiled code interact with complex runtime subsystems, a comprehensive static mapping is maintained inside `src/jit/symbols/mod.rs`. The `SymbolRegistry` declares and registers external Rust symbols prefixed with `xcx_jit_*` with Cranelift.

Through this registration, JIT-generated Cranelift IR calls external routines for:
- Reference counting management (`xcx_jit_inc_ref`, `xcx_jit_dec_ref`).
- Memory allocation and initialization of `Array`, `Map`, `Set`, and `Table` instances.
- Core FFI library lookups (string manipulation, JSON serialization, date parsing).
- I/O and network layers (`net_call`, `net_request`, `terminal_move`, `terminal_cursor`).
- Deep recursion checks (`xcx_jit_check_recursion`, `xcx_jit_dec_recursion`).

### Macro-driven Declarations (`src/jit/symbol_macros.rs`)
To avoid manually repeating binding declarations, the compiler uses the `declare_jit_symbols!` macro. This macro:
1. Generates the fields of the `SymbolRegistry` struct (declaring `FuncId` imports for Cranelift).
2. Generates the fields of the `ImportedSymbols` struct (defining `FuncRef` function call reference mappings).
3. Automates the linkage imports declaration block inside `SymbolRegistry::new`.
4. Automates the declaration of target function imports in individual trace functions inside `SymbolRegistry::import_in_func`.

