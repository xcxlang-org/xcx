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
- **Fast-Path Integers:** Generates Cranelift machine instructions (`iadd`, `isub`, `imul`). Division checks zero values to raise division-by-zero errors.
- **Fast-Path Floats:** Emits Cranelift floating-point directives (`fadd`, `fsub`, `fmul`, `fdiv`).
- **Polymorphic Paths:** Calls FFI helpers (`xcx_jit_add`, `xcx_jit_sub`, `xcx_jit_mul`, `xcx_jit_div`, `xcx_jit_mod`) when variables have an `Unknown` type tag. It packs values as boxed quiet-NaN representations.

---

## 2. Call Emitters (`emit_call.rs`)

Calculates call offsets and invokes local JIT frames or VM wrappers.
- **`emit_call`:** Routes function calls. For local, matching-signature functions, it generates direct local recursive JIT-to-JIT calls. For other functions, the compiler emits a fast-path direct `call_indirect` to the callee's JIT memory pointer if compiled. Otherwise, it emits a slow-path FFI call to `xcx_jit_call_recursive`.
- **`emit_method_call` & `emit_method_call_custom`:** Resolves method dispatch targets by calling `xcx_jit_method_dispatch` or invoking FFI handlers.

---

## 3. Control Flow Emitters (`emit_control.rs`)

Manages execution bounds, conditional checks, loop construction, and fiber state machine yielding.
- **Type Guards (`emit_guard_int`, `emit_guard_float`, `emit_guard_bool`):** Inserts assertions checking that dynamic registers store expected type tags. On tag mismatch, they trigger deoptimization pathways by calling `xcx_jit_report_guard_failure`.
- **Loop Structs:** Standard loop operations (`LoopNext`, `LoopPrev`, `IncLocalLoopNext`, `ArrayLoopNext`, `TableIter`) translate into Cranelift block branches. Loops evaluate constraints against limits, jumping backwards to block headers or forward to exit targets.
- **Yield and Return (`emit_yield`, `emit_return`):** Serializes current compiler registers to `locals_ptr` and returns control to the interpreter parent frame, passing status states.

---

## 4. Load & Store Emitters (`emit_load_store.rs`)

Stores and transfers registers while orchestrating garbage collection routines:
- **GC Refcounting:** Coordinates reference counters. Emitter injections call `emit_conditional_inc_ref` and `emit_conditional_dec_ref` to clean up old pointer resources during overrides.
- **Variable Mapping:** Implements const loading (`emit_load_const`) and variable assignments (`emit_get_var`, `emit_set_var`).
- **JSON Binding:** Connects variables to JSON pathways (`emit_json_bind_local`, `emit_json_bind_global`).

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
