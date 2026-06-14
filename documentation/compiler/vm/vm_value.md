# VM — Value Representation

All runtime values in XCX are represented by a single 16-byte struct. The design is documented here alongside the tag system, reference counting, and arithmetic dispatch.

---

## Module Layout

```
src/vm/value/
├── mod.rs          — re-exports
├── value.rs        — Value struct, constructors, arithmetic, comparisons
├── nan_boxing.rs   — TAG_* constants, float packing/unpacking
├── tag.rs          — Tag enum (human-readable type identifiers)
├── ref_count.rs    — inc_ref / dec_ref implementations
└── heap_object.rs  — Arc-based heap accessor helpers
```

---

## `Value` Struct

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Value {
    pub bits: u64,
    pub tag:  u64,
}
```

16 bytes, `repr(C)`, `align(8)`. `Copy` — values are always passed by copy; reference counting is manual and explicit.

`bits` holds the raw payload:
- `i64` cast to `u64` for integers.
- `f64::to_bits()` output for floats.
- A raw `*const T` pointer (as `u64`) for all heap-allocated types.

`tag` is one of the `TAG_*` constants (see below). It is a `u64` for JIT FFI compatibility; only the low byte is meaningful.

---

## Tag Constants (`nan_boxing.rs`)

```
TAG_FLOAT   = 0   — f64
TAG_INT     = 1   — i64
TAG_BOOL    = 2   — bool (bits: 0 = false, 1 = true)
TAG_DATE    = 3   — i64 milliseconds since Unix epoch
TAG_STR     = 4   — *const StringObj (via Arc)
TAG_ARR     = 5   — *const RwLock<ArrayObj> (via Arc)
TAG_SET     = 6   — *const RwLock<SetObj> (via Arc)
TAG_MAP     = 7   — *const RwLock<MapObj> (via Arc)
TAG_TBL     = 8   — *const RwLock<TableObj> (via Arc)
TAG_FUNC    = 9   — u32 function index (in bits), or *const FunctionObj
TAG_ROW     = 10  — *const RowObj (via Arc)
TAG_JSON    = 11  — *const JsonObj (via Arc)
TAG_FIB     = 12  — *const RwLock<FiberObj> (via Arc)
TAG_DB      = 13  — *const DatabaseObj (via Arc)
TAG_CLOSURE = 14  — *const ClosureObj (via Arc)
TAG_ARENA   = 15  — pointer into a short-lived Arena; inner type identified by a second tag
```

`TAG_FIRST_PTR = TAG_STR (4)`. Any value with `tag >= 4` is heap-allocated and participates in reference counting, with the exception of `TAG_FUNC` when used as a function index (bits holds the index directly).

---

## Float Encoding

XCX previously used NaN-boxing (packing tags into IEEE 754 quiet-NaN bit patterns). The current scheme uses a separate `tag` field, but floats still go through `pack_float_bits` / `unpack_float_bits` to ensure that NaN bit patterns which collided with the old encoding scheme are remapped to a canonical quiet NaN:

```rust
pub const QNAN_BASE:      u64 = 0x7FF0_0000_0000_0000;
pub const QNAN_CANONICAL: u64 = 0x7FF8_0000_0000_0001;

pub fn pack_float_bits(f: f64) -> u64 {
    let b = f.to_bits();
    if (b & QNAN_BASE) == QNAN_BASE && b != f64::INFINITY.to_bits() && b != f64::NEG_INFINITY.to_bits() {
        QNAN_CANONICAL
    } else {
        b
    }
}
```

This affects constant pools serialized under the old format and the JIT FFI layer, which also passes floats as raw bits.

---

## Tag Enum (`tag.rs`)

```rust
pub enum Tag {
    Float, Int, Bool, Date, String, Array, Set, Map,
    Table, Function, Row, Json, Fiber, Database, Unknown
}
```

`Tag::name()` returns `"float"`, `"int"`, etc. Used in error messages and the `typeof` operator. `Value::tag()` maps the raw `u64` tag to the `Tag` enum; `Value::type_name()` returns the string directly.

---

## Constructors

| Method | Description |
|---|---|
| `Value::from_i64(v)` | `{ bits: v as u64, tag: TAG_INT }` |
| `Value::from_f64(f)` | `{ bits: pack_float_bits(f), tag: TAG_FLOAT }` |
| `Value::from_bool(b)` | `{ bits: b as u64, tag: TAG_BOOL }` |
| `Value::from_string(Arc<StringObj>)` | Stores the raw `Arc` pointer. |
| `Value::from_array(Arc<RwLock<ArrayObj>>)` | Same pattern. |
| `Value::from_set(Arc<RwLock<SetObj>>)` | Same pattern. |
| `Value::from_map(Arc<RwLock<MapObj>>)` | Same pattern. |
| `Value::from_table(Arc<RwLock<TableObj>>)` | Same pattern. |
| `Value::from_json(Arc<JsonObj>)` | Same pattern. |
| `Value::from_fiber(Arc<RwLock<FiberObj>>)` | Same pattern. |
| `Value::from_row(Arc<RowObj>)` | Same pattern. |
| `Value::from_database(Arc<DatabaseObj>)` | Same pattern. |
| `Value::from_function(id: u32)` | Stores the function index as `bits`. |
| `Value::from_date(ts: i64)` | Stores Unix millisecond timestamp. |
| `Value::pack_ptr<T>(ptr, tag)` | Low-level: packs any raw pointer with a given tag. |
| `Value::from_string_array(Arc<Vec<String>>)` | Convenience: converts a Rust string vector to an XCX array of strings. |

---

## Type Queries

All `#[inline]` predicates on `Value`:

| Method | True when |
|---|---|
| `is_int()` | `tag == TAG_INT` |
| `is_float()` | `tag == TAG_FLOAT` |
| `is_bool()` | `tag == TAG_BOOL` |
| `is_date()` | `tag == TAG_DATE` |
| `is_ptr()` | `tag >= TAG_FIRST_PTR` |
| `is_arena()` | `tag == TAG_ARENA` |
| `is_string()` | `tag == TAG_STR` or Arena with inner tag `TAG_STR` |
| `is_array()` | `tag == TAG_ARR` or Arena inner |
| `is_set()` | `tag == TAG_SET` or Arena inner |
| `is_map()` | `tag == TAG_MAP` or Arena inner |
| `is_table()` | `tag == TAG_TBL` or Arena inner |
| `is_json()` | `tag == TAG_JSON` or Arena inner |
| `is_fiber()` | `tag == TAG_FIB` or Arena inner |
| `is_row()` | `tag == TAG_ROW` or Arena inner |
| `is_db()` | `tag == TAG_DB` or Arena inner |
| `is_func()` | `tag == TAG_FUNC` or Arena inner |
| `is_closure()` | `tag == TAG_CLOSURE` or Arena inner |
| `is_numeric()` | `tag == TAG_INT` or `TAG_FLOAT` |
| `is_bool_false()` | `tag == TAG_BOOL && bits == 0` |

---

## Arithmetic

All arithmetic is implemented directly on `Value` with type dispatch:

### `add`
- Int + Int: wrapping add.
- Either is string: stringify both and concatenate.
- Either is float: float addition.
- Date + Int: adds `rhs * 86_400_000` ms (day units).
- Fallback: wrapping add on raw bits.

### `sub`
- Int - Int: wrapping subtract.
- Either float: float subtraction.
- Date - Int: subtracts `rhs * 86_400_000` ms.
- Date - Date: returns the difference in **days** as `i64`.

### `mul`
- Int × Int: wrapping multiply.
- Otherwise: float multiplication.

### `div` → `Result<Value, ()>`
- Either float: float division; errors on `0.0`.
- Integer: integer division; errors on `0`.
- Returns `Err(())` on division by zero; callers emit a halt.

### `rem` → `Result<Value, ()>`
- Same dispatch as `div`. Uses truncating remainder (consistent with truncating division toward zero).

### `pow`
- Either float: `f64::powf`.
- Integer with non-negative exponent ≤ `u32::MAX`: `i64::wrapping_pow`.
- Otherwise: `f64::powf`.

### `neg`
- Float: negate float.
- Otherwise: negate as i64.

---

## Comparison

`PartialEq`, `Eq`, `PartialOrd`, `Ord` are all implemented on `Value`.

Equality: checks `tag` first; if tags differ, values are not equal. For strings, arrays, sets, maps: dereferences the heap pointer and compares contents. All other types compare `bits` directly.

Ordering: first compares by `variant_rank()` (type ordering: int < float < bool < string < array < set < map < date < table < function < row < json < fiber < database). Within the same type, compares the natural value (numeric, lexicographic for strings, timestamp for dates).

Named comparison helpers: `xcx_eq`, `xcx_ne`, `xcx_lt`, `xcx_le`, `xcx_gt`, `xcx_ge` — thin wrappers used from the runtime and JIT FFI.

---

## Accessors

| Method | Returns |
|---|---|
| `as_i64()` | `bits as i64` |
| `as_f64()` | `unpack_float_bits(bits)` |
| `as_bool()` | `bits != 0` |
| `as_bits()` | raw `bits` (used by JIT FFI) |
| `as_string()` | `Arc<StringObj>` (panics if wrong type) |
| `as_array()` | `Arc<RwLock<ArrayObj>>` |
| `as_set()` | `Arc<RwLock<SetObj>>` |
| `as_map()` | `Arc<RwLock<MapObj>>` |
| `as_table()` | `Arc<RwLock<TableObj>>` |
| `as_json()` | `Arc<JsonObj>` |
| `as_fiber()` | `Arc<RwLock<FiberObj>>` |
| `as_row()` | `Arc<RowObj>` |
| `as_database()` | `Arc<DatabaseObj>` |
| `as_date()` / `as_date_millis()` | `i64` millisecond timestamp |
| `as_function_idx()` | `u32` function index |
| `as_array_opt()` | `Option<Arc<RwLock<ArrayObj>>>` — safe version |
| `as_str_borrow<'a>()` | `Option<&'a str>` — zero-copy string borrow (unsafe) |
| `as_string_lossy()` | `String` — owned UTF-8 lossy string |
| `unpack_ptr<T>()` | raw `*const T` from bits |

---

## Cast Methods

For use in type-coercing expressions (e.g. `to_int()` method calls):

- `cast_int() -> i64`: Int passes through; Float truncates; Bool → 0/1; String parses with `parse::<i64>()` defaulting to 0; Date returns timestamp.
- `cast_float() -> f64`: same logic for float target.

---

## Reference Counting (`ref_count.rs`)

XCX uses manual reference counting on top of Rust's `Arc`. Because `Value` is `Copy`, `Arc` clone/drop are not called automatically — every assignment must be managed explicitly.

```rust
pub unsafe fn inc_ref(val: &Value)  // calls Arc::increment_strong_count
pub unsafe fn dec_ref(val: &Value)  // calls Arc::decrement_strong_count
```

Both are no-ops for non-pointer values (`tag < TAG_FIRST_PTR`) and for `TAG_ARENA` values (arena memory is bulk-freed, not ref-counted individually).

### Assignment Helpers

```rust
// Increment new value, decrement displaced value, overwrite
pub unsafe fn assign_to(&self, dest: &mut Value)

// Decrement displaced value only (caller already incremented self)
pub unsafe fn replace_at(&self, dest: &mut Value)
```

These are used throughout the executor's locals manipulation to maintain correct reference counts when overwriting registers.

---

## Membership (`has`)

`Value::has(item)` checks whether `item` is contained in the receiver:
- Array: linear scan of `elements`.
- Set: `BTreeSet::contains`.
- Map: linear scan of keys.
- String: substring containment check.
- Other types: always false.

---

## `to_sql_value`

Converts a `Value` to `rusqlite::types::Value` for SQL binding. Forwarded to `heap_object::to_sql_value`.

---

## Arena

```rust
pub struct Arena {
    chunks: RefCell<Vec<Vec<u8>>>,
}
```

A bump allocator used for short-lived heap objects in performance-critical paths (e.g. JIT-compiled code that produces temporary values without needing full `Arc` overhead). Chunks start at 4096 bytes and grow as needed.

```rust
pub fn with_arena<F, R>(arena: &Arena, f: F) -> R
pub fn alloc_in_arena<T>(value: T) -> Option<*mut T>
```

`with_arena` installs the arena as a thread-local, so that code inside the closure can call `alloc_in_arena` without passing the arena explicitly. Values allocated in an arena have `tag == TAG_ARENA`; `inc_ref` / `dec_ref` are no-ops for them. The arena and all its memory are freed together when the `Arena` is dropped.