<p align="center">
  <img src="https://github.com/xcxlang-org/xcx-branding/blob/main/banner/svg/banner.svg" width="560">
</p>

![Rust](https://img.shields.io/badge/built%20with-Rust-orange)
![License](https://img.shields.io/badge/license-Apache%202.0-blue)
![version](https://img.shields.io/github/v/release/xcxlang-org/xcx)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
![GitHub Stars](https://img.shields.io/github/stars/xcxlang-org/xcx?style=flat)
![GitHub Issues](https://img.shields.io/github/issues/xcxlang-org/xcx)
![Last Commit](https://img.shields.io/github/last-commit/xcxlang-org/xcx)
![Repo Size](https://img.shields.io/github/repo-size/xcxlang-org/xcx)

> **XCX 4.3 is the current release.** XCX 4.4 is under development, focused on `--no-jit` interpreter-mode performance, recursion time, and continued technical-debt cleanup. If you run into something unexpected, [open an issue](https://github.com/xcxlang-org/xcx/issues).

---

## Why XCX exists

Most backend languages make you choose between two bad options: high-level languages that are productive but drag in frameworks, ORMs, and config files you didn't ask for, or low-level languages that give you control but make a simple HTTP endpoint feel like work.

XCX is an experiment in a third path: a statically typed language where HTTP, SQLite, JSON, crypto, and file I/O are part of the language itself, not libraries you bolt on. No `package.json`. No ORM. No middleware boilerplate. You write logic; the runtime handles the rest.

It started in December 2025 as a question: *can an AI generate a working language runtime from scratch?* It went through a Python prototype, a C rewrite, and finally a Rust implementation that became XCX 3.x. XCX 4.3 is the current release, built on a redesigned VM and JIT with substantially improved performance. One contributor so far.

---

## What XCX looks like

```xcx
fiber handle_login(json: req -> json) {
    json: body; req.bind("body", body);

    s: username; s: password;
    body.bind("username", username);
    body.bind("password", password);

    s: hash  = crypto.hash(password, "argon2");
    s: token = crypto.token(32);

    json: resp <<< {"ok": true, "token": ""} >>>;
    resp.set("token", token);
    yield net.respond(200, resp);
    return <<< {} >>>;
};

fiber handle_404(json: req -> json) {
    yield net.respond(404, <<< {"error": "not found"} >>>);
    return <<< {} >>>;
};

serve: api {
    host   = "[IP_ADDRESS]",
    port   = 8000,
    routes = ["POST /login" :: handle_login, "*" :: handle_404]
};
```

**HTTP server with SQLite in ~20 lines:**

```xcx
database: app { engine = "sqlite", path = "app.db" };

table: users {
    columns = [ id :: i @auto @pk, name :: s @unique, age :: i ]
    rows = [EMPTY]
};

fiber handle_users(json: req -> json) {
    table: all = yield app.fetch(users);
    yield net.respond(200, all.toJson());
    return <<< {} >>>;
};

fiber handle_create(json: req -> json) {
    json: body; req.bind("body", body);
    s: name; body.bind("name", name);
    i: age;  body.bind("age", age);

    yield app.insert(users, name = name, age = age) as saved;

    json: resp <<< {"ok": true, "id": 0} >>>;
    resp.set("id", saved.insertId);
    yield net.respond(201, resp);
    return <<< {} >>>;
};

serve: api {
    host   = "[IP_ADDRESS]",
    port   = 8080,
    routes = ["GET /users" :: handle_users, "POST /users" :: handle_create]
};
```

---

## How XCX compares to alternatives

The honest picture: what you gain and what you give up:

| | Go | Node.js | Python | XCX |
|---|---|---|---|---|
| HTTP server | stdlib + router | Express / Fastify | Flask / FastAPI | built-in `serve:` |
| Database | GORM / sqlx | Prisma / Knex | SQLAlchemy | built-in `database:` |
| JSON | `encoding/json` + structs | native | `json` module | first-class type `json` |
| Crypto | `crypto` stdlib | `bcrypt` npm package | `bcrypt` pip | built-in `crypto.*` |
| Type safety | strong | optional (TS) | optional (mypy) | static, compile-time |
| Concurrency model | goroutines | event loop | async/await | cooperative fibers |
| Ecosystem | large | very large | very large | minimal (early stage) |
| Platform support | all | all | all | Windows (primary) / Linux / macOS |

XCX is not trying to replace Go or Node. It occupies a different space: small backend services and tools where you want zero dependency setup and a language that knows what you're building. The trade-off is an early-stage ecosystem and a single contributor.

---

## Performance

With XCX 4.3, the benchmark suite was overhauled and moved to its own repo:
**[xcxlang-org/xcx-benchmarks](https://github.com/xcxlang-org/xcx-benchmarks)**.
This is the new Main Suite, run under a stricter, more transparent methodology
than previous releases.

> ⚠️ These benchmarks reflect the **current state of XCX 4.3**.
> The runtime, VM, and JIT are still under active development and will change.
>
> The goal of this section is **transparency**, not competition.

### Performance Benchmarks Results Table (JSON penalty method: PROPORTIONAL)

| # | Language / runtime | FIB (30) | LCG (100M) | SIEVE | JSON |
| --- | --- | --- | --- | --- | --- |
| 01 | Zig [AOT] | 1.66ms | 5.28ms | 11.97ms | 0.04*ms |
| 02 | Rust [AOT] | 1.81ms | 21.36ms | 12.70ms | 0.06*ms |
| 03 | C [AOT] | 1.19ms | 85.34ms | 8.72ms | 0.07*ms |
| 04 | C++ [AOT] | 1.26ms | 86.33ms | 18.27ms | 0.09*ms |
| 05 | V [AOT] | 1.00ms | 189.00ms | 11.50ms | 0.10*ms |
| 06 | Crystal [AOT] | 2.69ms | 107.37ms | 22.59ms | 0.14*ms |
| 07 | Go [AOT] | 3.10ms | 242.37ms | 16.16ms | 0.17*ms |
| 08 | Java [JIT] | 2.64ms | 217.88ms | 24.38ms | 0.18*ms |
| 09 | Julia [JIT] | 3.48ms | 195.45ms | 22.91ms | 0.27*ms |
| 10 | XCX 4.3 [JIT] | 10.560ms | 104.852ms | 36.542ms | 0.134ms |
| 11 | Dart [AOT] | 4.92ms | 108.02ms | 19.78ms | 0.77ms |
| 12 | Nim [AOT] | 17.50ms | 193.00ms | 19.50ms | 0.31*ms |
| 13 | Bun [JIT] | 5.35ms | 400.84ms | 32.52ms | 0.40ms |
| 14 | PyPy [JIT] | 19.47ms | 119.28ms | 101.21ms | 0.32ms |
| 15 | LuaJIT [JIT] | 6.71ms | 378.47ms | 88.69ms | 0.46*ms |
| 16 | C# [JIT] | 5.66ms | 208.82ms | 15.94ms | 5.58ms |
| 17 | Node.js [JIT] | 6.70ms | 1363.96ms | 29.42ms | 0.63ms |
| 18 | Deno [JIT] | 7.20ms | 1456.83ms | 29.03ms | 0.57ms |
| 19 | PHP [INT] | 67.54ms | 1739.70ms | 387.66ms | 1.06ms |
| 20 | Lua [INT] | 69.00ms | 2346.00ms | 531.00ms | 3.33*ms |
| 21 | Python [INT] | 96.67ms | 16573.77ms | 1045.21ms | 0.49ms |
| 22 | Ruby [INT] | 59.27ms | 20938.24ms | 557.13ms | 1.39ms |
| 23 | Erlang [JIT] | 6.00ms | 4891.00ms | 12353.50ms | 5.39*ms |
| 24 | Gleam [JIT] | 7.01ms | 6086.81ms | 10581.50ms | 8.28*ms |
| 25 | Perl [INT] | 351.03ms | 4506.39ms | 2379.04ms | 11.75*ms |
| 26 | R [INT] | 560.00ms | 11220.00ms | 860.00ms | 13.26*ms |

`*` = JSON value is a computed stdlib penalty, not a direct measurement (see methodology).

XCX 4.3 ranks 10th by geometric mean, ahead of Dart, Nim, Bun, PyPy, LuaJIT, C#, Node.js, Deno, and every interpreted language tested. This run adds Julia, Dart, Deno, and Gleam to the field, and XCX's own numbers improved on all four tests versus the previous measurement (fib 12.303→10.560 ms, lcg 106.480→104.852 ms, sieve 97.814→36.542 ms, json 0.238→0.134 ms). `lcg` and `fib` are competitive with JIT peers; `sieve` remains the main target for optimization in upcoming releases.

**Methodology, short version:** every language runs the same algorithmic work
(no skipped operations, no dead-code elimination); AOT languages are compiled
at max optimization, JIT languages get a warmup phase before measurement,
interpreters are measured cold. JSON parsing only counts stdlib-native
parsers — languages without one get a computed penalty (median relative JSON
cost of other languages, scaled to that language's own speed, ×1.5) instead
of an unfair N/A. Final ranking uses geometric mean across all four tests.

Full methodology, benchmark sources, and category suites (loop variants,
function/allocation tests, HTTP client tests):
[xcxlang-org/xcx-benchmarks/METHODOLOGY.md](https://github.com/xcxlang-org/xcx-benchmarks/blob/main/METHODOLOGY.md)

---

## Architecture

XCX compiles source code through a multi-stage pipeline, all implemented in Rust:

```
Source (.xcx)
  -> Lexer        byte scanner on &[u8], no allocation, manual UTF-8 handling
  -> Pratt Parser top-down operator precedence, one-token lookahead
  -> Expander     resolves include directives, alias prefixing
  -> Sema         type checker, symbol table, collects all errors before codegen
  -> Compiler     two-pass, emits register-based bytecode + source spans
  -> VM           register VM, 16-byte { bits, tag } values, Arc ref counting
  -> JIT          Cranelift method JIT, hot functions compiled to native machine code
```

**Value representation:** every value is a 16-byte `{ bits: u64, tag: u64 }` struct. The explicit integer tag means zero bitwise operations when reading the type in the interpreter. Scalars (int, float, bool, date) require zero heap allocation. Pointers to heap objects (strings, arrays, JSON, tables, fibers) are packed into `bits`. The JIT uses NaN-boxing internally (Cranelift registers hold a single NaN-boxed `u64`), with `pack_value`/`unpack_value` adapters at the boundary, which keeps CPU register usage lower and block signatures simpler in compiled functions.

**Fibers** are cooperative coroutines backed by saved `Vec<Value>` state. Not OS threads. Suspend/resume moves the locals vector without copying. Each HTTP handler runs as a fiber; the server spawns N OS worker threads, each with its own executor. Globals are shared via `Arc<RwLock<Vec<Value>>>`. Fiber scoping works correctly on all platforms.

**JIT**: compilation is per function ("method JIT"). Every chunk counts its calls; when the count crosses the warmup threshold (`--threshold` / `--th`, default 50), the whole function is compiled by Cranelift to native code, specialized for the statically inferred register types. Functions invoked from already-compiled code are compiled from the 5th call onward. Calls between compiled functions become direct native `call` instructions (with a native 800-frame recursion guard); calls into uncompiled functions fall back through an FFI trampoline. Fibers execute on the interpreter. General string operations are not JIT-compiled, with one exception: the self-concatenation pattern (`x = x + expr`) is recognized and compiled to a dedicated in-place-append fast path.

Full compiler internals: [`documentation/compiler/`](documentation/compiler/)

---

## Project status

XCX 4.3 is best treated as an experimental platform. It is not production-ready, and APIs may change. Expect rough edges.

**What works well:** HTTP servers, SQLite integration, JSON handling, file I/O, cooperative concurrency, interactive terminal programs, and numeric workloads that benefit from JIT-optimized loops.

**Known rough edges:**

- **Linux and macOS**: XCX 4.3 compiles and passes the full test suite on Linux and macOS. Primary development happens on Windows, so Unix-specific issues may take longer to address. If you run into anything platform-specific, please [open an issue](https://github.com/xcxlang-org/xcx/issues).
- **HTTP server request body**: there is currently no size limit on incoming request bodies. A fix (rejecting oversized requests with `413` before the handler runs) is planned for XCX 4.4.

The ecosystem is minimal and evolving. APIs and internal behavior may change across minor versions.

Contributions are welcome; bug reports and pull requests are appreciated. There is no formal contribution process yet. For larger changes, please open an issue first.

---

## Development notes

XCX is currently a solo project, and that shapes how it gets built. Architecture,
design decisions, and debugging are worked out through discussion with
Claude, implementation is done with Google Antigravity.
Final decisions, review, and everything that ships are mine.

---

## Roadmap

### XCX 4.x: stabilization and known fixes

The 4.x line focuses on fixing known architectural issues and improving runtime correctness and performance:

- **4.3** (released): JIT stability and fallback, correctness fixes, verified dead-code remediation, and the sieve refcount-elision optimization.
- **4.4** (in development): performance of the `--no-jit` interpreter mode — it has regressed in every release since 4.0 and is the main focus of this cycle; further recursion-time improvements (`fib`); continued technical-debt elimination.
- **4.5**: ships only if substantive findings surface during 4.4; if 4.4 closes cleanly, work moves straight to 5.0a.

### XCX 5.0a: language evolution (early planning)

No timeline. Work begins when 4.4 closes (via 4.5a only if needed). Early-stage planning includes `match` statement and pattern matching. No breaking changes to existing 4.x syntax are planned.

---

## Getting started

**1. Download** the installer from [Releases](https://github.com/xcxlang-org/xcx/releases): `xcx-setup.exe` (Windows) or the Linux/macOS binary tarballs.

This adds `xcx` to your PATH. To uninstall on Windows: `xcx-uninstall.exe`.

**2. Hello world** (save as `hello.xcx`)

```xcx
>! "Hello, world!";
```

```bash
xcx hello.xcx
```

**3. Try the REPL:**

```bash
xcx
xcx> i: x = 2 ^ 10;
xcx> >! x;
1024
xcx> !exit
```

**4. Minimal HTTP server** (save as `server.xcx`)

```xcx
fiber handle(json: req -> json) {
    yield net.respond(200, <<< {"ok": true} >>>);
    return <<< {} >>>;
};

serve: api { host = "[IP_ADDRESS]", port = 8080, routes = ["*" :: handle] };
```

```bash
xcx server.xcx
# GET http://localhost:8080 -> {"ok":true}
```

---

## Core features

**Static typing:** `i`, `f`, `s`, `b`, `date`, `json`, `array:T`, `set:N/Z/Q/S/B/C`, `map`, `table`. Wrong types, missing fields, and unsafe queries are caught at compile time.

**Fibers:** cooperative coroutines with `yield`, `yield from`, and typed return values. Every HTTP handler is a fiber.

**Native SQL:** declare a `table:`, connect a `database:`, call `sync()`. No ORM, no migrations file, no config. SQLite out of the box.

**JSON as a first-class type:** raw literals `<<< {} >>>`, `.bind()`, `.set()`, `.inject()`, `.keys()`. JSON is how you talk to the outside world.

**Built-in HTTP:** client (`net.get/post/put/delete`) and server (`serve:`). Routes, handlers, CORS, and status codes, all in the language.

**Crypto and file I/O:** `crypto.hash`, `crypto.verify`, `crypto.token`, `store.read/write/append/glob/zip`.

**Terminal + interactive input:** raw mode, cursor control, non-blocking key input. Enough to build games, editors, and CLI tools.

**Collections:** `array.slice(start, end)` for subarray extraction; `.size()`, `.len()`, `.count()` as interchangeable aliases. Multiple variables of the same type can be declared in a single statement: `i: a, b = 42, c;`.

**PAX package manager:** `xcx pax install pkg`, `xcx pax upgrade xcx`. Own registry, beta stage; functional and usable, but API may still change.

**Configurable JIT threshold:** `xcx script.xcx --threshold=100` controls how many calls before a function is JIT-compiled. Default is 50.

---

## Building from source

Requires **Rust 1.85+** (the crate uses edition 2024).

```bash
git clone https://github.com/xcxlang-org/xcx
cd xcx
cargo build --release
```

Binary: `target/release/xcx`

---

## Editor support

VS Code extension: [xcxlang-org/xcx-vscode](https://github.com/xcxlang-org/xcx-vscode)

Syntax highlighting, snippets, `.xcx` and `.pax` support.

```bash
code --install-extension xcx-vscode-1.0.0.vsix
```

---

## Documentation

Full docs at **[xcxlang.com](https://xcxlang.com)**

Translated versions of the documentation are available at [github.com/xcxlang-org/xcx-docs](https://github.com/xcxlang-org/xcx-docs). Currently, translations cover XCX 3.1 only and have not been updated for 4.x. The English documentation in this repository is always the canonical and up-to-date reference.

### Language

| Topic | File |
|---|---|
| Types and variables | [`types.md`](documentation/language/types.md), [`variables.md`](documentation/language/variables.md) |
| Syntax basics | [`syntax.md`](documentation/language/syntax.md) |
| Operators | [`operators.md`](documentation/language/operators.md) |
| Control flow | [`control_flow.md`](documentation/language/control_flow.md) |
| Functions and fibers | [`functions_fibers.md`](documentation/language/functions_fibers.md) |
| Collections | [`collections.md`](documentation/language/collections.md) |
| String methods | [`string_methods.md`](documentation/language/string_methods.md) |
| JSON and HTTP | [`json_http.md`](documentation/language/json_http.md) |
| Database | [`database.md`](documentation/language/database.md) |
| Dates | [`dates.md`](documentation/language/dates.md) |
| I/O and terminal | [`io_terminal.md`](documentation/language/io_terminal.md) |
| Standard library | [`library_modules.md`](documentation/language/library_modules.md) |
| Error handling | [`errors_halt.md`](documentation/language/errors_halt.md) |

### Compiler internals

| Topic | File |
|---|---|
| Overview | [`README.md`](documentation/compiler/README.md) |
| **Frontend** | |
| AST | [`frontend/ast.md`](documentation/compiler/frontend/ast.md) |
| Lexer | [`frontend/lexer.md`](documentation/compiler/frontend/lexer.md) |
| Parser | [`frontend/parser.md`](documentation/compiler/frontend/parser.md) |
| **Semantic analysis** | |
| Core | [`sema/sema_core.md`](documentation/compiler/sema/sema_core.md) |
| Expressions | [`sema/sema_expr.md`](documentation/compiler/sema/sema_expr.md) |
| Statements | [`sema/sema_stmt.md`](documentation/compiler/sema/sema_stmt.md) |
| Types | [`sema/sema_types.md`](documentation/compiler/sema/sema_types.md) |
| **Compiler** | |
| Core | [`compiler/compiler_core.md`](documentation/compiler/compiler/compiler_core.md) |
| Expressions | [`compiler/compiler_expr.md`](documentation/compiler/compiler/compiler_expr.md) |
| Statements | [`compiler/compiler_stmt.md`](documentation/compiler/compiler/compiler_stmt.md) |
| Registers | [`compiler/compiler_registers.md`](documentation/compiler/compiler/compiler_registers.md) |
| **HIR** | |
| Overview | [`hir/README.md`](documentation/compiler/hir/README.md) |
| Structures | [`hir/hir_core.md`](documentation/compiler/hir/hir_core.md) |
| Lowering | [`hir/hir_lower.md`](documentation/compiler/hir/hir_lower.md) |
| Inlining | [`hir/hir_inline.md`](documentation/compiler/hir/hir_inline.md) |
| Codegen | [`hir/hir_codegen.md`](documentation/compiler/hir/hir_codegen.md) |
| **VM** | |
| Executor | [`vm/vm_executor.md`](documentation/compiler/vm/vm_executor.md) |
| Objects | [`vm/vm_objects.md`](documentation/compiler/vm/vm_objects.md) |
| Opcodes | [`vm/vm_opcode.md`](documentation/compiler/vm/vm_opcode.md) |
| Values | [`vm/vm_value.md`](documentation/compiler/vm/vm_value.md) |
| **JIT** | |
| Core | [`jit/jit_core.md`](documentation/compiler/jit/jit_core.md) |
| Codegen | [`jit/jit_codegen.md`](documentation/compiler/jit/jit_codegen.md) |
| Emitters | [`jit/jit_emitters.md`](documentation/compiler/jit/jit_emitters.md) |
| **Runtime** | |
| Core | [`runtime/runtime_core.md`](documentation/compiler/runtime/runtime_core.md) |
| Collections | [`runtime/runtime_collections.md`](documentation/compiler/runtime/runtime_collections.md) |
| Services | [`runtime/runtime_services.md`](documentation/compiler/runtime/runtime_services.md) |
| **Diagnostics** | |
| Compiler errors | [`diagnostics/compiler_errors.md`](documentation/compiler/diagnostics/compiler_errors.md) |
| **REPL** | |
| REPL | [`repl/repl.md`](documentation/compiler/repl/repl.md) |

### Tooling

| Topic | File |
|---|---|
| PAX manual | [`pax_manual.md`](documentation/tooling/pax_manual.md) |
| Doc tool | [`doc_manual.md`](documentation/tooling/doc_manual.md) |

---

## License

Apache 2.0 - see [LICENSE](LICENSE)
