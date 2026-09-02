# VM — Heap Objects

All heap-allocated values in XCX are wrapped in `Arc` (or `Arc<RwLock<T>>` for mutable types). The raw pointer is packed into a `Value::bits` field with the corresponding tag. This document covers every object type, its structure, ownership semantics, and notable implementation details.

---

## Module Layout

```
src/vm/object/
├── mod.rs          — re-exports all objects
├── string_obj.rs   — StringObj
├── array_obj.rs    — ArrayObj
├── set_obj.rs      — SetObj
├── map_obj.rs      — MapObj
├── function_obj.rs — FunctionObj
├── fiber_obj.rs    — FiberObj
├── json_val.rs     — JsonVal (recursive JSON value tree)
├── json_obj.rs     — JsonObj (root container)
├── table_obj.rs    — TableObj, VMColumn, SqlBinding, JoinPred
├── database_obj.rs — DatabaseObj
├── row_obj.rs      — RowObj
└── bool_array_obj.rs — BoolArrayObj
```

---

## StringObj

```rust
pub struct StringObj {
    pub data: Vec<u8>,
    pub hash: Option<u64>,
}
```

XCX strings are arbitrary byte sequences that are always treated as valid UTF-8 at the language level (the parser and all string construction paths guarantee this). The `data` field is a `Vec<u8>` rather than a `String` to avoid redundant encoding checks in hot paths.

`hash` is lazily computed and cached on first access. Used by hash-based lookups.

`StringObj` is wrapped in `Arc<StringObj>` (no `RwLock` — strings are immutable). `Value::TAG_STR` points to this `Arc`.

`StringObj` implements `Deref<Target = [u8]>`, `PartialEq`, `Eq`, `Hash`, and comparisons against `&[u8]` and fixed-size byte arrays for convenient use in the executor.

---

## ArrayObj

```rust
pub struct ArrayObj {
    pub elements: Vec<Value>,
}
```

Dynamic ordered collection. Wrapped in `Arc<RwLock<ArrayObj>>`. The `RwLock` allows shared reads across concurrent fiber execution.

`Drop` decrements the reference count of every element:
```rust
impl Drop for ArrayObj {
    fn drop(&mut self) {
        for e in &self.elements {
            unsafe { e.dec_ref(); }
        }
    }
}
```

`Deref<Target = Vec<Value>>` and `DerefMut` provide direct slice access.

---

## BoolArrayObj

```rust
pub struct BoolArrayObj {
    pub data: Vec<u8>,
}
```

A specialized array representation for boolean sequences, storing elements as bytes inside a `Vec<u8>`. Wrapped in `Arc<RwLock<BoolArrayObj>>`.

Its memory layout is leveraged by the JIT compiler's Fast Path:
- Data pointer offset: 16
- Length offset: 24

---

## SetObj

```rust
pub struct SetObj {
    pub elements: BTreeSet<Value>,
    pub cache:     Option<Vec<Value>>,
    pub cached_arr: Option<Value>,
}
```

Mathematical set backed by a `BTreeSet<Value>`. Ordering is defined by `Value::Ord` which sorts by type rank first, then value. This guarantees deterministic iteration order.

`cache` holds a `Vec<Value>` snapshot used by iteration opcodes that need indexed access into the set. `cached_arr` is a pre-built `Value::from_array(...)` of the sorted elements, used by methods that need to return the set as an array.

`Drop` decrements all element refcounts and releases `cached_arr`.

Wrapped in `Arc<RwLock<SetObj>>`.

---

## MapObj

```rust
pub struct MapObj {
    pub elements: Vec<(Value, Value)>,
}
```

An ordered key-value dictionary implemented as a `Vec` of pairs. Insertion order is preserved; lookup is linear. This matches XCX's map semantics where maps are expected to be small and iteration order is meaningful.

`Drop` decrements refcounts for every key and value.

Wrapped in `Arc<RwLock<MapObj>>`. `Deref<Target = Vec<(Value, Value)>>`.

---

## FunctionObj

Stores compiled functions. Normally functions are referenced by index into the `SharedContext::functions` vector. `FunctionObj` is used when a function must be carried as a first-class heap value (e.g. passed as an argument).

Wrapped in `Arc<FunctionObj>`.

---

## FiberObj

```rust
pub struct FiberObj {
    pub func_id: usize,
    pub ip: usize,
    pub locals: Vec<Value>,
    pub status: FiberStatus,   // Suspended | Running | Done | Error
    pub is_done: bool,
    pub yielded_value: Option<Value>,
    pub trace_revision: u64,
}
```

Represents a suspended coroutine. A fiber references its function by index, has its own instruction pointer and locals array, and tracks completion via `FiberStatus` plus the `is_done` flag and the last `yielded_value`.

Wrapped in `Arc<RwLock<FiberObj>>`. The `RwLock` is necessary because the executor that drives the fiber mutates it while the parent executor may be reading the `done` flag concurrently.

---

## JsonVal

```rust
pub enum JsonVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Arc<StringObj>),
    Array(Arc<RwLock<Vec<JsonVal>>>),
    Object(Arc<RwLock<Vec<(Arc<String>, JsonVal)>>>),
}
```

An in-memory JSON tree. All nodes are `Clone`. Arrays and objects are wrapped in `Arc<RwLock<...>>` for shared mutable access.

`JsonVal` is distinct from `Value` — it is used only inside `JsonObj` and for conversions between XCX values and JSON.

### Key Methods

`from_serde(serde_json::Value) -> JsonVal` — converts a serde value tree into a `JsonVal` tree. Strings become `Arc<StringObj>`; object keys go through the thread-local `intern_key` cache (`json_val.rs`) so repeated key names share one `Arc<String>` allocation.

`to_serde() -> serde_json::Value` — reverse conversion.

`to_string_buf(&mut String)` — zero-allocation JSON serialization into a pre-allocated string buffer. Uses a hand-written integer formatter to avoid format string overhead on hot paths.

`pointer(path: &str) -> Option<JsonVal>` — RFC 6901 JSON pointer traversal. Splits path on `/`, traverses arrays by index and objects by key.

`deep_clone()` — recursively copies every node. The JIT JSON parse cache uses `is_flat()` to decide when to clone. (Note: `shallow_clone` was removed in Phase 3C).

---

## JsonObj

```rust
pub struct JsonObj {
    pub root: JsonVal,
}
```

The outer `Arc` wrapper around a `JsonVal` root. All JSON `Value`s (`TAG_JSON`) point to a `JsonObj`. The object is immutable after construction; mutations produce a new `JsonObj`.

---

## TableObj

```rust
pub struct TableObj {
    pub table_name: String,
    pub columns:    Vec<VMColumn>,
    pub rows:       Vec<Vec<Value>>,
    pub sql_binding: Option<SqlBinding>,
    pub sql_where:   Option<String>,
    pub pending_op:  Option<MethodKind>,
}
```

An in-memory relational table. Rows are stored as a flat `Vec<Value>` in row-major order — row `r`, column `c` lives at `rows[r * columns.len() + c]`; `TableObj::len()` derives the row count as `rows.len() / columns.len()`.

Wrapped in `Arc<RwLock<TableObj>>`.

### VMColumn

```rust
pub struct VMColumn {
    pub name:     String,
    pub ty:       crate::frontend::ast::Type,
    pub is_auto:  bool,
    pub is_pk:    bool,
    pub is_unique: bool,
}
```

Column metadata carried at runtime. `is_auto` and `is_pk` are used by the database binding layer to generate SQL DDL.

### SqlBinding

```rust
pub struct SqlBinding {
    pub db_conn:    Arc<Mutex<rusqlite::Connection>>,
    pub table_name: String,
}
```

When a table is bound to a SQLite database, `sql_binding` is `Some(SqlBinding)`. All mutating method calls on a bound table are forwarded to the database after being applied in memory. `sql_where` stores a pending WHERE clause for chained `.where(...)` calls before `.query()`.

### Drop

`TableObj::drop` decrements the refcount of every value in every row.

### Clone

`TableObj::clone` increments the refcount of every value in every row before returning the clone. This maintains the invariant that every copy of a `Value` in a table has its refcount reflected.

### Utility Methods

`to_json()` — serializes the table as a JSON array of objects, one per row.

`to_formatted_grid()` — returns a fixed-width ASCII table string suitable for printing, with padded column headers and a separator line.

### JoinPred

```rust
pub enum JoinPred {
    Keys(String, String),     // join on named columns
    Lambda(usize),            // join via function index
    Closure(usize, Vec<Value>), // join via closure with captured values
}
```

Used by the `Join` method implementation to specify how two tables are joined.

---

## RowObj

```rust
pub struct RowObj {
    pub table:   Arc<RwLock<TableObj>>,
    pub row_idx: u32,
}
```

A lightweight reference to a single row inside a table. Does not copy the row data; holds an `Arc` to the table and an index. `RowGet` opcodes and JIT helpers dereference the table and index on each access.

Wrapped in `Arc<RowObj>`.

---

## DatabaseObj

Wraps a SQLite connection (`rusqlite::Connection`) behind a `Mutex`, alongside the `engine`/`path` strings it was opened with and a `tables: Arc<RwLock<HashMap<String, Value>>>` cache of introspected table handles. Database method calls (query, insert, begin, commit, rollback, etc.) lock the connection mutex for the duration of the operation.

Wrapped in `Arc<DatabaseObj>`.

---

## Stack Structures

### `ValueStack`

```rust
pub struct ValueStack {
    pub stack: Vec<Value>,  // pre-allocated to MAX_STACK (256K values)
    pub sp:    usize,
}
```

A pre-allocated value stack. `MAX_STACK = 256 * 1024` values (2MB at 8 bytes/value). Operations: `push`, `pop`, `peek(distance)`, `get(idx)`, `set(idx, val)`. All are `#[inline]` and panic on overflow/underflow.

`Drop` decrements refcounts for all values still on the stack when the stack is freed.

Note: in the current `Executor` design the executor uses its own inline `Vec<Value>` stack (`stack: Vec<Value>`) initialized to 64K entries; `ValueStack` is available as an alternative but the main execution path uses the executor's own field. (Note: `StackGuard` was removed in Phase 3A as recursion limits are enforced elsewhere).

---

## JSON Utilities (`vm/utils/json.rs`)

Conversion functions between `Value` and `JsonVal`:

`value_to_json(v: &Value) -> JsonVal` — converts any runtime value to its JSON representation. Strings are decoded as UTF-8. Arrays and sets become JSON arrays. Maps become JSON objects. Tables become JSON arrays of objects. Rows become JSON objects. Dates become `"%Y-%m-%d %H:%M:%S"` strings.

`json_val_to_value(v: &JsonVal) -> Value` — reverse conversion. `Null` becomes `false`. Primitive types map directly. Arrays and objects become `Value::from_json(JsonObj)`.

`build_response_json(result: Result<ureq::Response, ureq::Error>) -> JsonVal` — constructs the standard HTTP response JSON object `{ status, ok, body, headers }`. Body responses larger than 50MB are replaced with a 413 error object (`{ status, ok, error }`).

---

## Path Utilities (`vm/utils/path.rs`)

`normalize_json_path(path: &str) -> String` — converts dot-notation and bracket-notation paths (`a.b[0].c`) to JSON Pointer format (`/a/b/0/c`). If the path already starts with `/`, it is returned as-is.

`get_path_value_xcx(root: Value, path: &str) -> Option<Value>` — traverses arrays, maps, and JSON objects following a normalized path. Returns `None` if any segment fails to resolve. Increments refcount on the returned value; the caller owns the result.

`set_path_value_xcx(root: Value, path: &str, value: Value)` — mutable path traversal. Sets the target field in an array or map. Creates intermediate nodes as needed.

`validate_path_safety(path: &str)` — security check that prevents `..` traversal and absolute paths. Calls `process::exit(1)` on violation.

---

## Network Utilities (`vm/utils/network.rs`)

`is_safe_url(url_str: &str) -> Result<(), String>` — SSRF protection. Blocks `file://` URLs, `169.254.*` link-local addresses, and all RFC 1918 private IP ranges. Localhost and loopback (`127.0.0.1`, `::1`) are explicitly allowed (for development use).