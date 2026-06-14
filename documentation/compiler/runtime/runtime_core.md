# XCX Runtime Core and FFI Subsystem

The XCX runtime acts as an intermediary layer between the Virtual Machine interpreter, JIT execution blocks, and the underlying platform operating system. 

---

## Service Registry (`src/runtime/builtin/registry.rs`)

The Virtual Machine and JIT compiler access native services through a centralized registry. The built-in services registry maps and re-exports modules from the `builtin` directory, dividing capabilities into clear namespaces:

- **`json`**: Core JSON manipulation and query injections.
- **`math`**: Randomization algorithms, pseudo-random tables, and exponent powers.
- **`string`**: Case converters, trimmers, string slicers, and parsers.
- **`io`**: Text output (print buffers), input scanners, and ANSI terminal controls.
- **`net`**: HTTP request drivers, socket listeners, and response writers.
- **`crypto`**: Password generators, hash mechanisms (bcrypt/sha256/tokens).
- **`store`**: Disk caching, file operations, key-value data structures.

These namespaces compile cleanly into static symbols that are shared across both the interpreter runtime loops and compiled JIT functions.

---

## FFI Linking and Conventions (`src/runtime/ffi_helpers/`)

To invoke Rust-implemented built-ins from machine code, JIT traces must use a standardized calling convention. The files under `ffi_helpers/` implement cross-linkage interfaces prefixed with `xcx_jit_*` using standard C ABI bindings.

### ABI Signature Pattern
FFI functions conform to raw call configurations:
1. **Name Mangling Elision:** Extracted with `#[unsafe(no_mangle)]` and `pub unsafe extern "C"`.
2. **Output Handling:** Functions returning a heap-allocated structures or dynamic type write their returned values to an output pointer reference (`out: *mut Value`).
3. **Primitive Returns:** Methods returning pure numbers or flags return standard primitives (`i64`, `f64`, or `u8`/`bool`) directly in registers:

```rust
// String lowercase converter - writes to the out pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_lower(out: *mut Value, bits: u64, _tag: u64);

// String index searcher - returns primitive char count directly
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_string_index_of(bits: u64, _tag: u64, f_bits: u64, f_tag: u64) -> i64;
```

---

## NaN-Boxed Parameter Conversion

Because the VM stores parameters inside quiet-NaN boxing slots, FFI functions must unpack arguments before processing and repack them before returning:

```
    [ JIT Trace / Cranelift IR ]
                 │
                 │ Passes (bits: u64, tag: u64)
                 ▼
  ┌──────────────────────────────┐
  │      FFI Helper Function     │
  │     (e.g., string_trim)      │
  ├──────────────────────────────┤
  │ 1. Reconstruct Value struct  │
  │    Value { bits, tag }       │
  │                              │
  │ 2. Unpack to Arc pointer     │
  │    as_string() / heap_object │
  │                              │
  │ 3. Execute native Rust trim  │
  │    to_string().trim()        │
  │                              │
  │ 4. Repack result             │
  │    Value::from_string()      │
  │                              │
  │ 5. Place in out slot         │
  │    *out = result             │
  └──────────────────────────────┘
```

1. **Reconstruction:** Helper accepts raw components (`bits: u64`, `tag: u64`) and aggregates them into a `Value` struct instance.
2. **Tag Extraction:** Queries the tag against heap boundaries or maps it directly to heap-allocated object wrappers:
   ```rust
   let s_arc: Arc<StringObj> = crate::vm::value::heap_object::as_string(&Value { bits, tag: _tag });
   ```
3. **Rust Operations:** Unwraps data payload into safe Rust types (e.g. converting utf8 byte vectors into a `String`), executes the standard operation.
4. **Repacking:** Allocates output objects on the allocator heap, packs them under appropriate type tags (`TAG_STRING`, `TAG_ARRAY`, etc.), and updates the target `*out` pointer.

---

## Panic and Exit Propagation
If FFI functions run into unrecoverable invalid states (like parsing an alphabetical string as an integer):
- They trigger a panic `panic!("halt.error:...")`.
- The compiler JIT or VM executor context catches the panic message to interrupt the JIT loop and translate the execution path back to interpreter error diagnostics.
