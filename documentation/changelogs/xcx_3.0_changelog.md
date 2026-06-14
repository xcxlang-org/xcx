# XCX 3.0 — Changelog (2.2 → 3.0)
**April 2026 · Type: ANNOUNCEMENT**

---

## Database (NEW)

Entirely new subsystem. No equivalent existed in 2.2.

- **[NEW]** `database:` block — declares a SQLite connection with `engine`, `path`, `timeout`, `readonly` fields
- **[NEW]** Connections are lazy — opened on first use; SQLite creates the file automatically
- **[NEW]** Multiple simultaneous connections supported
- **[NEW]** Column attributes for `table:` declarations used by the DB module: `@pk`, `@unique`, `@optional`, `@default(v)`, `@fk(t.col)` — `@auto` already existed in 2.2
- **[NEW]** Write operations: `db.push()`, `db.save()` (upsert, requires `@pk`), `db.insert()`
- **[NEW]** Read operations: `db.fetch()`, `db.fetch().where()` (compiles to SQL `WHERE`), `db.query()`, `db.queryRaw()`, `.first()` on raw results
- **[NEW]** Delete operations: `db.remove().where()` (`.where()` required — compile error D401 otherwise), `db.truncate()`, `db.exec()`
- **[NEW]** Transactions: `db.begin()`, `db.commit()`, `db.rollback()` — auto rollback on `halt.error`
- **[NEW]** Result capture via `as`: returns object with `affected` and `insertId` fields
- **[NEW]** DDL: `db.sync()`, `db.drop()`, `db.has()`
- **[NEW]** `@optional` columns: SQL `NULL` maps to the XCX type's default value on read (`0`, `0.0`, `""`, `false`)
- **[NEW]** `save()` without `@pk` = compile error; `remove()` without `.where()` = compile error (D401)
- **[NEW]** Workaround documented for fiber-local variable scoping bug on Windows (planned fix in 4.0)

---

## Named Arguments (NEW)

- **[NEW]** `table.add()`, `table.insert()`, and `db.insert()` now accept arguments by column name
- **[NEW]** Positional and named arguments can be mixed — positional must come first
- **[NEW]** `@auto` columns cannot be passed explicitly (compile error)
- **[NEW]** Duplicate column names in one call = compile error
- **[NEW]** Namespace separation: left side of `=` is the column name, right side is a local expression — no conflict

---

## Type System

- **[NEW]** Type aliases added to simple types table: `i / int`, `f / float`, `s / str`, `b / bool`
- **[NEW]** `array:json` — array of JSON objects, documented as a distinct type
- **[NEW]** `database:` added as a complex type
- **[CHG]** Strict collection validation — type mismatches in array/set/table literals are now compile errors (e.g. `array:s: names {"Alice", 2}` → ERROR [S103])
- **[INT]** Internal "Other" type removed from the type checker

---

## Error System (NEW)

No structured error codes existed in 2.2.

- **[NEW]** Sxxx / Dxxx error code series, consistent across REPL and compiler:

| Code | Description           |
|------|-----------------------|
| S103 | Type mismatch         |
| S108 | Invalid index         |
| S109 | Property not found    |
| S110 | Method not found      |
| S111 | Invalid arguments     |
| S211 | Void fiber            |
| S212 | Invalid fiber run     |
| S302 | Table row mismatch    |

---

## Collections — Arrays

- **[NEW]** `.toStr()` — serializes array to a JSON-formatted string
- **[NEW]** `.toJson()` — converts array to a native `json` value

---

## Collections — Maps

- **[NEW]** `.toStr()` — serializes map to a JSON-formatted string
- **[NEW]** `.toJson()` — serializes map to a JSON object; keys converted to strings

---

## Collections — Tables

- **[NEW]** `.toJson()` — serializes all rows to a JSON array of objects; `@auto` columns included
- **[NEW]** Named arguments for `.add()` and `.insert()` (see Named Arguments section)
- **[NEW]** Column attributes `@pk`, `@unique`, `@optional`, `@default(v)`, `@fk(t.col)` — used when connecting a table to a database
- **[CHG]** Stricter row and column validation

---

## Collections — Sets / random.choice

- **[CHG]** `random.choice from` now works on **both sets and arrays** — in 2.2 it was restricted to sets only
- **[CHG]** Picking from an empty set or array now returns `false` (previously undocumented)

---

## Standard Library — crypto

- **[NEW]** `crypto.hash(data, "base64_encode")` — encodes to Base64
- **[NEW]** `crypto.hash(data, "base64_decode")` — decodes from Base64

---

## Standard Library — store

- **[NEW]** `store.list(p)` — lists files and folders at path
- **[NEW]** `store.isDir(p)` — checks if path is a directory
- **[NEW]** `store.size(p)` — returns file size in bytes
- **[NEW]** `store.mkdir(p)` — creates directory recursively
- **[NEW]** `store.glob(pat)` — returns files matching a glob pattern
- **[NEW]** `store.zip(src, target)` — archives to zip
- **[NEW]** `store.unzip(zip, dest)` — extracts zip

---

## Standard Library — random

- **[NEW]** `random.int(min, max)` / `random.int(min, max @step n)` — random integer in range
- **[NEW]** `random.float(min, max)` / `random.float(min, max @step n)` — random float in range
- **[CHG]** `random.choice from` now supports arrays in addition to sets (see Collections — Sets)

---

## Terminal & Input

- **[NEW]** `input` module: `input.key()`, `input.key() @wait`, `input.ready()`
- **[NEW]** Full set of key constants: `"UP"`, `"DOWN"`, `"LEFT"`, `"RIGHT"`, `"ENTER"`, `"ESC"`, `"BACKSPACE"`, `"TAB"`, `"F1"`–`"F12"`, `"KEY_CTRL_C"`, `"KEY_CTRL_Z"`, `"KEY_CTRL_S"`
- **[NEW]** `.terminal !raw` — raw mode (no echo, no buffering)
- **[NEW]** `.terminal !normal` — restores normal terminal mode
- **[NEW]** `.terminal !cursor on` / `.terminal !cursor off`
- **[NEW]** `.terminal !move x y` — moves cursor to position
- **[NEW]** `.terminal !write expr` — prints without trailing newline
- **[CHG]** 2.2 had only 3 terminal directives (`!clear`, `!exit`, `!run`); 3.0 adds 6 more

---

## I/O — User Input (`>?`)

- **[CHG]** In 2.2, entering a value of the wrong type caused the variable's type to change at runtime (dynamic typing). In 3.0 this behavior is removed — `>?` strictly attempts to parse into the declared type.

---

## JSON

- **[NEW]** `.push(val)` — appends a `json` element to a JSON array node; `halt.error` if called on an object
- **[NEW]** `.first()` — returns the first element of a JSON array; `halt.error` if empty
- **[NEW]** `.toStr()` — renamed from `.to_str()` (see Breaking Changes)
- **[NEW]** Serialization from collections: `.toJson()` available on maps and tables, producing JSON objects and arrays respectively

---

## HTTP — Response Object

2.2 response had four fields. 3.0 adds two:

- **[NEW]** `text` — raw response body as a string (always available)
- **[NEW]** `error` — error message string on request failure; empty string if OK

---

## HTTP — CORS

- **[CHG]** Behavior changed from 2.2 to 3.0:
  - **2.2**: Engine added `Access-Control-Allow-Origin: *` automatically; manually adding it caused a browser duplicate header error
  - **3.0**: Engine adds CORS headers only if they are **missing** from the response; explicitly declaring them in code is now **recommended**

---

## String Methods

- **[NEW]** `.split(separator)` — splits string by separator, returns `array:s`

---

## PAX Package Manager

### project.pax

- **[NEW]** `version` field — semantic version (e.g. `"1.0.0"`)
- **[NEW]** `author` field — synced with registry
- **[NEW]** `description` field
- **[NEW]** `main` field — custom entry point (default: `src/main.xcx`)
- **[NEW]** `tags` field — list of strings for registry discoverability
- **[NEW]** Version pinning in deps: `"mathlib@1.2.0"`

### Commands

- **[NEW]** `xcx pax clone <name>` — clones the entire package repository
- **[NEW]** `xcx pax login <token>` — saves API token for publishing
- **[NEW]** `xcx pax logout` — clears stored session
- **[NEW]** `xcx pax whoami` — verifies registry account
- **[NEW]** `xcx pax publish` — pushes project manifest to the registry

### Lockfile

- **[NEW]** `pax.lock` — deterministic builds; locks all dependencies to specific versions and sources; should be committed to version control

---

## Breaking Changes

| Area | 2.2 | 3.0 |
|------|-----|-----|
| JSON method | `.to_str()` | `.toStr()` (renamed) |
| `>?` input | Variable type changes at runtime if input mismatches | Strict parsing only, no type change |
| CORS | Do NOT add `Access-Control-Allow-Origin` manually | Recommended to declare headers explicitly |
| `random.choice from` | Sets only | Sets and arrays |

---

## Internal Changes

- **[INT]** Type checker refactor
- **[INT]** Removal of internal "Other" type
- **[INT]** Internal VM structures documented
- **[INT]** Readable errors in REPL
- **[INT]** Compilation timing output
- **[INT]** Improved diagnostics