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
Represented by `MapObj` wrapping `RwLock<HashMap<Value, Value>>`.
- **Symbol mappings:** Supports element retrievals, mutations, and membership validations (`xcx_jit_map_init`, `xcx_jit_has`).

### Sets (`set/`)
Represented by `SetObj` wrapping `RwLock<HashSet<Value>>`.
- **Set Arithmetic:** Supports algebraic operations via optimized C-linkage helpers:
  - **Union:** `set_union` (combines two sets into a single distinct set).
  - **Intersection:** `set_intersection` (returns a set with values present in both operands).
  - **Difference:** `set_difference` / `set_sym_difference`.

### JSON Translation and Relaxed Parsing (`json/`)
The JSON parser (`parse.rs`) implements a two-stage parsing flow to reconcile XCX-specific literals:
1. **Strict Parse Fallback:** Attempts normal decoding using `serde_json`.
2. **Relaxed Preprocessor (`relaxed_preprocess`):** If standard parsing fails, a lexical scanner identifies brace configurations representing arrays (e.g. `{1, 2}` instead of `[1, 2]`). It tracks bracket balances and colons: if a matching pair of curly braces `{}` lacks a mapping colon `: `, the preprocessor converts them to square brackets `[]` before submitting to the decoder.
3. **Structured Translation:** Recreates values as strongly-typed XCX constructs (e.g., nesting Maps or Arrays of type-tagged values).

### Fiber Schedulers and JIT Segments (`fiber/`)
Fibers execute as co-routines on the interpreter stack frame (`ops.rs`):
- **Cooperative Yields:** Fiber schedules support `Status`, `IsDone`, and cooperatively yield values via execution frame contexts.
- **Segmented JIT compilation:** High-frequency fiber loops execute compiled machine code blocks or compile bytecode segments inline:
  1. Checks the chunk's virtual trace cache (`jit_segments`) matching the current `ip`. If a compiled function resides in the map, it transmsites and runs it directly.
  2. If the current segment hits the hotspot tick threshold (`hotspot.tick`), the runtime compiles that specific loop segment (`compile_fiber_segment`) and updates the instruction map.
  3. When JIT is not active or compiling, execution falls back seamlessly to the interpreter interpreter-loop.

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
   - **Local Evaluation:** Under pure in-memory execution, the validator loops through the collection rows under a read lock. It executes code closures row-by-row on the active execution stack frame (`run_frame`), moving values satisfying the conditions into the output table.
   - **Database-delegated Evaluation:** If the Table carries an database binding (`sql_binding`), the compiler utilizes a translator utility (`translate_filter_to_sql`) to parse matching expressions into a SQL `WHERE` clause. This statement is prepared as an SQLite prepared query on the thread's connection lock (`conn.prepare`), transferring filter optimization to SQLite.
2. **Relational Joins:** Matches left and right table records based on join keys (`JoinPred::Keys`) or lambda criteria (`JoinPred::Lambda`), returning a synthesized virtual table.
3. **Table CRUD Mutations:**
   - **Insertions:** `Table.insert` appends row vectors in-memory or issues `INSERT INTO table` queries to SQLite.
   - **Deletions & Updates:** Translates matching criteria into SQLite statements (`DELETE FROM table WHERE ...` or `UPDATE table SET ... WHERE ...`).
