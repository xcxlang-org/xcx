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
    pub stack_ptr_offset: u32,
    pub functions: Option<&'a [Arc<Chunk>]>,
    // Variable storage mapping register index -> Cranelift variables (bits and tag)
    pub slots: [Option<SlotVars>; 256],
    pub register_const: [Option<i64>; 256],
    pub known_types: [crate::vm::opcode::TypeTag; 256],
    ...
}
```

### Preloading and Spilling Mechanics
- **Preloading:** The context resolves which locals are used by calling `preload_locals`. It emits Cranelift load instructions to fetch register data from the pointer-offset of `locals_ptr` and binds them to Cranelift SSA variable definitions.
- **Spilling:** Because FFI methods can cause context switches or GC sweeps, registers must sometimes be written back (spilled) to the stack structure. `spill_all` stores all dirty Cranelift variables back into their offsets relative to `locals_ptr`.
- **Reloading:** At control boundaries or post-call entry blocks, registers are refreshed from stack slots (`reload_globals` / `clear_block_state`).

### Constant Tracking in Registers (`register_const`)

`register_const: [Option<i64>; 256]` records the known compile-time integer value of a register whenever it is populated by `LoadConst` with an `Int` constant, or by a `Move` that copies a register with a known constant. This is separate from `known_types`, which only tracks the type tag — `register_const` tracks the *value* itself when it is statically known.

Downstream emitters consult `register_const` to skip runtime checks entirely when an operand is a known constant — see "Fast Division and Modulo by a Constant Divisor" below. The map is invalidated per-block by `clear_block_state`, which is called after every conditional branch (`emit_jump_if`) unless the caller explicitly passes `keep_consts = true`; this ensures a constant known on one control-flow path is never incorrectly assumed to still hold after a branch merges from another path where it wasn't established.

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
- **`analyze_maybe_ptr_regs`:** Analyzes register states using a fast 256-bit bitmask (`[u64; 4]`) instead of `[bool; 256]`. Merging dataflow states across successor blocks is performed using bitwise bitmask operations, allowing the compiler to identify registers that possibly contain heap pointers and optimize refcount elision.

---

## Inlined Collection Size Optimization

To avoid the overhead of calling runtime FFI helpers, collection size queries (`.size()`, `.len()`, and `.count()`) on `Array`, `BoolArray`, and `Map` instances are compiled directly to native Cranelift instructions:
- The compiler emits a 64-bit load directly from offset 24 of the collection's base memory address (skipping the 8-byte `RwLock` header and reading the 16-byte offset capacity/length field).
- This bypasses runtime FFI helper calls (`xcx_jit_array_size` and `xcx_jit_map_size`).

---

## Inlined Bounds-Checked Array Access

`GetIndex`/`SetIndex` opcodes, and the equivalent `Get`/`Set` method calls on an `Array` receiver whose element type is statically known to be `Int`, compile to a native bounds check plus a direct memory access rather than an unconditional FFI call:
- The index is compared against the array's length (`index < len`) with a native Cranelift `icmp`.
- On the in-bounds path, the element is read or written directly from the array's backing buffer.
- On the out-of-bounds path, execution falls back to the corresponding runtime helper, which raises the standard bounds error.

### `BoolArray` Fast Path

`BoolArray` values (`array:b`) are packed into a single-byte-per-element `Vec<u8>` guarded by an `RwLock`, with a fixed memory layout on Windows x64: the data pointer sits at offset 16 and the length at offset 24 of the array's base address. `GetIndex`/`SetIndex` on a register statically typed `TypeTag::BoolArray` compile to:
1. A native load of the data pointer and length from the fixed offsets.
2. A bounds check (`index < len`) branching to a fast or slow block.
3. On the fast path, a direct byte load/store at `data_ptr + index`, masked to a single bit for reads.
4. On the slow path, a fallback call to `xcx_jit_array_get_bool` / the equivalent setter helper.

This eliminates FFI and `RwLock` overhead entirely for correctly-typed, in-bounds accesses. Correct typing of `BoolArray` registers depends on the type inference engine recognizing `array:b` constants as `TypeTag::BoolArray` rather than `TypeTag::Unknown` — see "Bytecode Type Inference" below; without that recognition this fast path is never selected and every access falls back to the dynamic dispatch helper.

---

## Bytecode Type Inference

Bytecode instructions do not natively carry static types. The compiler runs abstract type analysis on traces using the engine inside `src/jit/type_inference.rs`.

### Flow Propagation Rules
`analyze_chunk_types` propagates type tags (`TypeTag`) through registers by processing the bytecode layout forwards:
- Math operations (e.g. `Add`, `Sub`) default outputs to `TypeTag::Int` or `TypeTag::Float` if their sources are already inferred as such.
- Relational operators unify outputs to `TypeTag::Bool`.
- Collection operations set registers to `TypeTag::Array`, `TypeTag::BoolArray`, or `TypeTag::Map`. A `LoadConst` whose constant value is a `BoolArray` is recognized directly (`is_bool_array()`) and tagged `TypeTag::BoolArray`, rather than falling through to `TypeTag::Unknown`.
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

For floating-point fast paths specifically, both operands must be statically known as `TypeTag::Float` (a logical AND, not an OR) — a mismatched-type pair (`Float + Int`) always routes through the polymorphic path, which resolves the mixed types correctly via `Value::cast_float` at runtime instead of reinterpreting an integer's raw bits as float bits.

### Typed Control-Flow Guards

`JumpIfFalse`/`JumpIfTrue` compile through `emit_jump_if`, which checks `ctx.get_reg_type(src)` before deciding how to test the branch condition. When the source register is statically known to be `TypeTag::Bool`, the test reduces to a single `icmp_imm` against the raw bits; when the type is not statically known, the emitter falls back to comparing both the type tag and the bit pattern against the boxed `false` representation.

### GC Escape Analysis (`uses_heap`)
Type inference also tracks whether a trace actually uses the heap (`uses_heap` flag). If a function is pure math and lacks pointer allocation opcodes, allocator escapes are elided entirely. Global-variable reloading after a call (in `emit_call`) is likewise conditional on the callee's `uses_heap` flag — a call known not to touch the heap does not force a reload of globals on return.

---

## Fast Division and Modulo by a Constant Divisor

When the divisor operand of `Div`/`Mod` has a statically known value in `ctx.register_const` (see "Constant Tracking in Registers" above) other than `0` or `-1`, the guard blocks that would otherwise check for division-by-zero and `i64::MIN / -1` overflow are skipped entirely, and the native `sdiv`/`srem` instruction is emitted directly.

For `Mod` specifically, when the known constant divisor is a power of two, the operation is further reduced to a branchless bitwise sequence (sign-extend, mask, adjust) equivalent to `srem` but without the `srem` instruction itself — this covers divisors up to `2^32`.

---

## Simplified Recursion Depth Guard

Direct JIT-to-JIT recursive calls are protected by a depth counter stored on the `Executor`. Before performing a local recursive call, the compiler emits a native comparison of the current depth against a fixed limit of `800` (`IntCC::SignedGreaterThanOrEqual`); on overflow it spills registers and calls the `xcx_jit_check_recursion` FFI helper, which reports the failure and returns a halt status, rather than inlining the error-reporting sequence at every call site.

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