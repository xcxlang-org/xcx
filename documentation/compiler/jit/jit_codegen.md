# XCX JIT Codegen Context and Static Analysis Passes

Compilation efficiency in the JIT relies on code generation context trackers and pre-emission bytecode analyses to optimize register usage and skip unnecessary reference counting.

---

## Codegen Context (`CodegenCtx`)

The `CodegenCtx` struct (`src/jit/codegen_ctx.rs`) maintains SSA register state and handles register spelling/reloads.

### SSA Variable Register Model
Cranelift represents variables as SSA values. The virtual machine registers (`0` to `255`) are mapped to Cranelift SSA variables where each VM register corresponds to two Cranelift variables:
1. **Value Bits:** The actual 64-bit bits representing the payload (`types::I64`).
2. **Type Tag:** The 64-bit tag representing the runtime type identifier (`types::I64`).

```rust
pub struct CodegenCtx<'a> {
    pub b: &'a mut FunctionBuilder<'a>,
    pub out_ptr: Value,
    pub locals_ptr: Value,
    pub globals_ptr: Value,
    pub consts_ptr: Value,
    // Variable storage mapping register index -> Cranelift variables (bits and tag)
    pub slots: [Option<SlotVars>; 256],
    ...
}
```

### Preloading and Spilling Mechanics
- **Preloading:** The context resolves which locals are used by calling `preload_locals`. It emits Cranelift load instructions to fetch register data from the pointer-offset of `locals_ptr` and binds them to Cranelift SSA variable definitions.
- **Spilling:** Because FFI methods can cause context switches or GC sweeps, registers must sometimes be written back (spilled) to the stack structure. `spill_all` stores all dirty Cranelift variables back into their offsets relative to `locals_ptr`.
- **Reloading:** At control boundaries or post-call entry blocks, registers are refreshed from stack slots (`reload_globals` / `clear_block_state`).

---

## Static Analysis Passes

The compiler implements a static analysis layer inside `src/jit/analysis.rs` to gather trace info before generating any Cranelift IR.

### Local/Global Register Analyses
- **`analyze_chunk_locals`:** Performs a linear code sweep to find which VM registers (0-255) are read or mutated, avoiding compiling loads/stores for dead registers.
- **`analyze_chunk_globals`:** Tracks references to global slots to optimize preloading symbols.

### Pointer Elision Registers Analyses
The JIT uses quiet-NaN boxing. Heap-allocated types (strings, sets, arrays, maps, tables) require reference counting (`inc_ref`/`dec_ref`). Primitive integers, floats, and booleans do not. Writing/moving non-pointer values does not trigger costly reference count updates.
- **`analyze_non_ptr_regs`:** Identifies registers that are guaranteed to stay primitive (e.g. arithmetic registers, known index counters). The emitter queries this map (`ctx.is_known_non_ptr`) to avoid inserting FFI ref count additions/subtractions.
- **`analyze_global_int_regs`:** Checks which global variables are exclusively used as integer values.
- **`analyze_maybe_ptr_regs`:** Flags registers that could potentially hold pointers, guaranteeing safety by retaining refcount calls only where strictly necessary.

---

## Bytecode Type Inference

Bytecode instructions do not natively carry static types. The compiler runs abstract type analysis on traces using the engine inside `src/jit/type_inference.rs`.

### Flow Propagation Rules
`analyze_chunk_types` propagates type tags (`TypeTag`) through registers by processing the bytecode layout forwards:
- Math operations (e.g. `Add`, `Sub`) default outputs to `TypeTag::Int` or `TypeTag::Float` if their sources are already inferred as such.
- Relational operators unify outputs to `TypeTag::Bool`.
- Collection operations set registers to `TypeTag::Array`, `TypeTag::BoolArray`, or `TypeTag::Map`.
- Variables default to `TypeTag::Unknown` if types cannot be structurally unified.

### Type Specialization Branching
Type tag inference assigns target type tags to each bytecode index. Emitters use these type labels to target fast-paths where types are known or default to dynamic polymorph calls where tags are `Unknown`:

```rust
let t1 = ctx.get_reg_type(src1);
let t2 = ctx.get_reg_type(src2);
if t1 == TypeTag::Int && t2 == TypeTag::Int {
    emit_add_int(ctx, symbols, dst, src1, src2);
} else {
    emit_add_poly(ctx, symbols, dst, src1, src2); // Dynamic fallback
}
```

### GC Escape Analysis (`uses_heap`)
Type inference also tracks whether a trace actually uses the heap (`uses_heap` flag). If a function is pure math and lacks pointer allocation opcodes, allocator escapes are elided entirely.

---

## Quiet NaN-Boxing Model (`src/jit/nan_ops.rs`)

To dynamically represent primitive values (Int, Float, Bool) and pointers within a uniform 64-bit space, the JIT uses quiet-NaN boxing.

### Memory Representation
A value occupies 16 bytes:
- **`VALUE_BITS_OFFSET` (0):** 8-byte payload representing raw value bits.
- **`VALUE_TAG_OFFSET` (8):** 8-byte type tag identifying the representation.
- **`VALUE_SIZE` (16):** Total size allocated per VM register value.

### Bitwise Mapping Rules
- **Integers:** Bitwise ANDed with `0x0000_FFFF_FFFF_FFFF` (masking to 48 bits) and ORed with prefix `0x7FF1_0000_0000_0000`. On unpacking (`unpack_int`), the value is shifted left by 16 bits and arithmetic shifted right by 16 bits to preserve sign extension.
- **Booleans:** Bitwise ORed with prefix `0x7FF2_0000_0000_0000`. Unpacking (`unpack_bool`) performs a bitwise AND with `1`.
- **Floats:** Cast directly to 64-bit IEEE float bits (`bitcast`).
- **Pointers:** Unpackaged by masking away high payload bits (`0x0000_FFFF_FFFF_FFFF`), retrieving the raw 48-bit memory address pointer.

