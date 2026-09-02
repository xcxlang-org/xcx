# XCX In-Memory Collections and Relational Databases

The XCX runtime supports basic structures (Arrays, Maps, Sets) and relational tables/databases synced directly with SQLite backends.

---

## Memory Collections (`src/runtime/builtin/`)

The JIT and VM interact with core types protected by thread-safe wrappers:

### Arrays (`array/`)
Represented by `ArrayObj` wrapping `RwLock<Vec<Value>>`.
- **Interoperability:** Standard JIT FFI functions (`xcx_jit_array_get`, `xcx_jit_array_set_bool`, `xcx_jit_array_update`) resolve references and perform bounds check validations.
- **Allocation:** `xcx_jit_array_init` allocates contiguous heap blocks on the VM heap.

### Maps (`map/`)
Represented by `MapObj` wrapping `RwLock<Vec<(Value, Value)>>`.
- **Symbol mappings:** Preserves insertion order with linear lookup. Supports element retrievals, mutations, and membership validations (`xcx_jit_map_init`, `xcx_jit_has`).

### Sets (`set/`)
Represented by `SetObj` wrapping `RwLock<BTreeSet<Value>>`.
- **Set Arithmetic:** Supports algebraic operations via optimized C-linkage helpers in `ffi_helpers/set_ffi.rs`:
  - **Union:** `xcx_jit_set_union` (combines two sets into a single distinct set).
  - **Intersection:** `xcx_jit_set_intersection` (returns a set with values present in both operands).
  - **Difference:** `xcx_jit_set_difference` / `xcx_jit_set_sym_difference`.

### JSON Translation and Relaxed Parsing (`json/`)
The JSON parser (`parse.rs`) implements a two-stage parsing flow to reconcile XCX-specific literals:
1. **Strict Parse Fallback:** Attempts normal decoding using `serde_json`.
2. **Relaxed Preprocessor (`relaxed_preprocess`):** If standard parsing fails, a lexical scanner identifies brace configurations representing arrays (e.g. `{1, 2}` instead of `[1, 2]`). It tracks bracket balances and colons: if a matching pair of curly braces `{}` lacks a mapping colon `: `, the preprocessor converts them to square brackets `[]` before submitting to the decoder.
3. **Structured Translation:** Recreates values as strongly-typed XCX constructs (e.g., nesting Maps or Arrays of type-tagged values).

Before either stage runs, `handle_json_parse` checks a `thread_local` cache (keyed by the raw source string; at most 16 entries or 512KB of keys, evicting the least-recently-inserted; only strings up to 16KB are cached) and returns the cached `JsonVal` on a hit — nested nodes are Arc-shared with the cache (copy-on-write via `make_mutable`), so the hit path skips both the strict and relaxed decodes entirely for repeated parses of an identical string.

**Simple-key fast paths:** Accessing a JSON object or array by a simple key — one containing none of `.`, `[`, `]`, `/` — bypasses the generic `json_pointer` path-resolution machinery in `get()`, `has()`, member access (`obj.field`), `keys()`/`len()`, and `bind()`/`json:bind` (`JsonBindLocal`). Instead, the field is looked up by scanning the object's backing storage directly (`data_ptr()`). The same direct-scan shortcut applies mid-path in general JSON path traversal (`get_path_value_xcx`): for object segments it avoids building a `/segment` pointer string and calling `pointer()`, and for array segments it parses the segment as a plain `usize` index. A structurally identical fast path exists for `get()` when the argument is an integer index into a JSON array — the index is used to reach the element directly rather than being converted to a string and re-parsed as a pointer segment.

**Cache versioning:** Because a JSON value can be read concurrently by the interpreter and JIT-compiled code, the cached serialized-string representation of a JSON object is invalidated using a pair of `AtomicU64` counters, `version` and `cached_version`, with `Acquire`/`Release` ordering, rather than a single `dirty: AtomicBool` flag. A mismatch between the two indicates the cached string is stale and must be regenerated; a match allows a lock-free cache hit. This closes a race window that existed with a boolean flag, where a reader could observe a serialized string mid-update.

### Fiber Schedulers (`fiber/`)
Fibers execute as co-routines on the interpreter stack frame (`ops.rs`):
- **Cooperative Yields:** Fiber schedules support `Status`, `IsDone`, and cooperatively yield values via execution frame contexts.
- **Execution model:** Fiber bodies run on the interpreter; JIT compilation is per-function (`jit_ptr` warmup) and fiber yields/resumes switch the executor's bytecode context. A former per-segment fiber JIT was unreachable and has been removed (see `documentation/work/2026-08-17_phase3b_tracejit_fiberjit_removal.md`).

---

## Relational Databases and Table Sync (`src/runtime/builtin/db/`)

The database module binds compiler structs directly to disk-backed SQLite database routines via `rusqlite`.

### Database Connection and DDL (`connection.rs`, `ddl.rs`)
- **Instantiation:** Creating or initializing a database spins up an active sqlite file wrapper (`DatabaseObj`), which maintains a shared thread-safe connection pool (`db_rc.conn`).
- **Table Detection (DDL):** Reading a database property (e.g. `db.users`) triggers dynamic table introspection inside `handle_database_ddl`:
  1. Queries the active database engine using `PRAGMA table_info([table_name])`.
  2. Parses results to extract column titles, data types, and primary key constraints.
  3. Maps SQL types (`INTEGER`, `REAL`, `TEXT`) back into the AST representation (`Type::Int`, `Type::Float`, `Type::String`, `Type::Bool`).
  4. Generates an in-memory `TableObj` containing a connection binding (`SqlBinding`).
- **Syncing Schemes:** `handle_database_sync` creates tables in the SQLite database dynamically if they do not yet exist, building the matching `CREATE TABLE` script using VMColumn tags.

---

## Table Operations & Query Translation (`src/runtime/builtin/table/`)

Virtual tables (`TableObj`) manage rows data either as in-memory arrays or SQL prepare handles.

### In-Memory vs. Database Queries (`select.rs`)
The table selection executor handles methods like `Where`, `Join`, `Show`, `Count`, and `Find`:
1. **Dynamic Filter Execution:** 
   - **Local Evaluation:** Under pure in-memory execution, the validator loops through the collection rows under a read lock. It executes code closures row-by-row on the active execution stack frame (`run_frame`), moving values satisfying the conditions into the output table. Each row is materialized as a `RowObj` looked up from (or inserted into) the `Executor`'s `row_cache` — keyed by the table's heap address — rather than allocated fresh on every call, since the same table is typically filtered many times across loop iterations; any structural mutation of the table (`insert`, `delete`, `update`, `clear`) invalidates its cache entry. See `compiler/vm/vm_executor.md`.
   - **Database-delegated Evaluation:** If the Table carries an database binding (`sql_binding`), the compiler utilizes a translator utility (`translate_filter_to_sql`) to parse matching expressions into a SQL `WHERE` clause. This statement is prepared as an SQLite prepared query on the thread's connection lock (`conn.prepare`), transferring filter optimization to SQLite.
2. **Relational Joins:** Matches left and right table records based on join keys (`JoinPred::Keys`) or lambda criteria (`JoinPred::Lambda`), returning a synthesized virtual table. Key-based joins (`JoinPred::Keys`) run through a hash-join algorithm (`join_tables` in `src/vm/utils/table.rs`): the right-hand table's key column is indexed once into a hash map keyed by a `HashableValue` wrapper (a `Value` newtype with a manual `Hash`/`Eq` impl that dispatches on the value's tag — string keys hash their contents, numeric/bool/date keys hash their raw bits), after which the left-hand table is scanned once, looking up matching right-hand rows directly instead of re-scanning the right table per left row. This makes key-based joins `O(N+M)` rather than `O(N×M)`. Lambda-predicate joins (`JoinPred::Lambda`), which have no fixed key to index on, still evaluate the predicate closure pairwise.
3. **Table CRUD Mutations:**
   - **Insertions:** `Table.insert` appends row vectors in-memory or issues `INSERT INTO table` queries to SQLite.
   - **Deletions & Updates:** Translates matching criteria into SQLite statements (`DELETE FROM table WHERE ...` or `UPDATE table SET ... WHERE ...`).
   - **Counting (`Count` / `Len` / `Size`):** When the table has both an active `sql_binding` and a pending `sql_where` filter, the row count is obtained via a `SELECT COUNT(*) FROM [table] WHERE ...` query executed directly against SQLite, rather than materializing and counting the filtered rows in memory. Without an active database-delegated filter, the count falls back to the in-memory row vector's length.
   - **JSON Serialization (`TableObj::to_json`):** Column names are converted to `Arc<String>` once, before the per-row loop, into a `Vec<Arc<String>>` shared by every row's JSON object — rather than calling `Arc::new(col.name.clone())` inside the loop for every row. The per-row JSON object's backing vector is also pre-sized with `Vec::with_capacity(columns.len())`.