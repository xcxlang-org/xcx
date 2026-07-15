# XCX Bytecode Emitters to Cranelift IR

Emitters transform virtual machine bytecode instructions into Cranelift IR using runtime FFI helpers for complex operations and native instructions for fast-path primitive operations.

---

## Emitter Submodules Structure

The emission routines are split by operation groups under `src/jit/`:

```
                            [ JIT Compiler ]
                                    │
         ┌──────────────────────────┼─────────────────────────┐
         ▼                          ▼                         ▼
 ┌───────────────┐          ┌───────────────┐         ┌───────────────┐
 │  emit_arith   │          │   emit_call   │         │ emit_control  │
 ├───────────────┤          ├───────────────┤         ├───────────────┤
 │  Arithmetic   │          │  Method and   │         │ Loops, Guards │
 │  Operations   │          │   Function    │         │  and Yields   │
 │ (Int, Float,  │          │  Resolution   │         │ (Type Guards, │
 │  Polymorph)   │          └───────────────┘         │  Loop Iter)   │
 └───────────────┘                                    └───────────────┘
         │                          │                         │
         ├──────────────────────────┼─────────────────────────┤
         ▼                          ▼                         ▼
 ┌───────────────┐          ┌───────────────┐         ┌───────────────┐
 │emit_load_store│          │  emit_object  │         │   emit_misc   │
 ├───────────────┤          ├───────────────┤         ├───────────────┤
 │ Const Loading,│          │   Disk I/O,   │         │ Environment,  │
 │ Variable Move │          │   Database,   │         │ Halt/Fatal,   │
 │ & Refcounting │          │  Table Parsing│         │   Printing    │
 └───────────────┘          └───────────────┘         └───────────────┘
```

---

## 1. Arithmetic Emitters (`emit_arith.rs`)

Arithmetic operations compile through fast-path primitives if types are statically verified, or fallback to polymorphic FFI helper functions:
- **Fast-Path Integers:** Generates Cranelift machine instructions (`iadd`, `isub`, `imul`). Division and modulo (`sdiv`/`srem`) normally emit a guard block checking for a zero divisor and for the `i64::MIN / -1` overflow case; when the divisor is a statically known constant (tracked via `ctx.register_const`, see `compiler/jit/jit_codegen.md`) other than `0` or `-1`, the guard is skipped and the raw instruction is emitted directly. A known power-of-two divisor to `Mod` additionally bypasses `srem` entirely in favor of an equivalent branchless bitwise sequence.
- **Fast-Path Floats:** Emits Cranelift floating-point directives (`fadd`, `fsub`, `fmul`, `fdiv`), selected only when **both** operands are statically known to be `TypeTag::Float` — a mismatched pair (e.g. `Float + Int`) always falls through to the polymorphic path so that the mixed-type numeric conversion is performed correctly at runtime instead of reinterpreting an integer's bit pattern as a float.
- **Polymorphic Paths:** Calls FFI helpers (`xcx_jit_add`, `xcx_jit_sub`, `xcx_jit_mul`, `xcx_jit_div`, `xcx_jit_mod`) when variables have an `Unknown` type tag, or when operand types are known but mixed. It packs values as boxed quiet-NaN representations.

---

## 2. Call Emitters (`emit_call.rs`)

Calculates call offsets and invokes local JIT frames or VM wrappers.
- **`emit_call`:** Routes function calls. For local, matching-signature functions, it generates direct local recursive JIT-to-JIT calls guarded by the recursion-depth check described in `compiler/jit/jit_core.md`. For other functions, the compiler emits a fast-path direct `call_indirect` to the callee's JIT memory pointer if compiled — reloading global variables afterward only when the callee's static analysis marks it as heap-touching — otherwise it emits a slow-path FFI call to `xcx_jit_call_recursive`.
- **`emit_method_call` & `emit_method_call_custom`:** Resolves method dispatch targets by calling `xcx_jit_method_dispatch` or invoking FFI handlers. `Get`/`Set` method calls on an `Array` receiver with a statically known `Int` element type route through the same inlined bounds-checked fast path used for `GetIndex`/`SetIndex` — see `compiler/jit/jit_codegen.md`.

---

## 3. Control Flow Emitters (`emit_control.rs`)

Manages execution bounds, conditional checks, loop construction, and fiber state machine yielding.
- **Type Guards (`emit_guard_int`, `emit_guard_float`, `emit_guard_bool`):** Inserts assertions checking that dynamic registers store expected type tags. On tag mismatch, they trigger deoptimization pathways by calling `xcx_jit_report_guard_failure`. A guard is skipped entirely when `ctx.known_types` already records the expected tag for that register.
- **Conditional Branches (`emit_jump_if`):** Backs `JumpIfFalse`/`JumpIfTrue`. When the source register is statically known to be `TypeTag::Bool`, the branch condition reduces to a single bit comparison (`icmp_imm`) instead of comparing both the type tag and the bit pattern against the boxed `false` value. After branching, `clear_block_state` resets any tracked register constants (`ctx.register_const`) for the continuation block, since a value known constant on one incoming path cannot be assumed constant after merging with a path where it wasn't.
- **Loop Structs:** Standard loop operations (`LoopNext`, `LoopPrev`, `IncLocalLoopNext`, `ArrayLoopNext`, `TableIter`) translate into Cranelift block branches. Loops evaluate constraints against limits, jumping backwards to block headers or forward to exit targets.
- **Yield and Return (`emit_yield`, `emit_return`):** Serializes current compiler registers to `locals_ptr` and returns control to the interpreter parent frame, passing status states.

---

## 4. Load & Store Emitters (`emit_load_store.rs`)

Stores and transfers registers while orchestrating garbage collection routines:
- **GC Refcounting:** Coordinates reference counters. Emitter injections call `emit_conditional_inc_ref` and `emit_conditional_dec_ref` to clean up old pointer resources during overrides.
- **Variable Mapping:** Implements const loading (`emit_load_const`) and variable assignments (`emit_get_var`, `emit_set_var`). Loading an integer constant additionally records its value in `ctx.register_const`, so later arithmetic on that register can take the constant-divisor fast paths described under Arithmetic Emitters above.
- **JSON Binding:** Connects variables to JSON pathways (`emit_json_bind_local`, `emit_json_bind_global`).
- **Inlined Array Access:** `GetIndex`/`SetIndex` on registers statically typed `Array` (`Int` element) or `BoolArray` compile to a native bounds check followed by a direct buffer read/write, falling back to the corresponding FFI helper only when the index is out of bounds — see `compiler/jit/jit_codegen.md` for the exact memory layout used for `BoolArray`.

---

## 5. Object & Database Emitters (`emit_object.rs`)

Links variables to tables, disk arrays, and structured I/O endpoints:
- **Disk Storage (`emit_store_read`, `emit_store_write`, `emit_store_exists`):** Connects to file-backed database storage helpers.
- **Database Initializer:** Spills register states and hooks database drivers (`emit_database_init`).
- **Table Member Accessors:** Emits instructions to retrieve and update row attributes (`emit_row_get`, `emit_table_push_row`, `emit_get_member`, `emit_set_member`).

---

## 6. Misc Emitters (`emit_misc.rs`)

Handles environment lookups and fatal errors.
- **Halt Handling (`emit_halt_alert`, `emit_halt_error`, `emit_halt_fatal`):** Halts compiler execution, registers error context fields, and executes clean return patterns.
- **OS Environment:** Accesses variables and startup scripts (`emit_env_get`, `emit_env_args`).

---

## 7. Eager JIT Compilation Pre-Pass

Before IR generation begins for a method or fiber segment, the JIT scans its bytecode for `OpCode::Call` instructions to eagerly compile dependencies:
- **Callee Pre-compilation:** Statically pre-compiling callees ensures that the target `jit_ptr` is resolved and ready in the fast-path direct `call_indirect` check, preventing slow FFI roundtrips.
- **Compiler Cycle Prevention:** Uses a thread-safe compiler state context tracker (`in_progress: HashSet<usize>` on the JIT compiler struct) containing bytecode chunk indexes currently undergoing compilation. If a circular call reference chain is encountered during eager compilation, the compiler immediately yields a null JIT pointer, safely fallbacking to the interpreter linkage for runtime resolution.