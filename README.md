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

> XCX 4.2 is an active project under development. If you run into something unexpected, [open an issue](https://github.com/xcxlang-org/xcx/issues).

---

## Why XCX exists

Most backend languages make you choose between two bad options: high-level languages that are productive but drag in frameworks, ORMs, and config files you didn't ask for, or low-level languages that give you control but make a simple HTTP endpoint feel like work.

XCX is an experiment in a third path: a statically typed language where HTTP, SQLite, JSON, crypto, and file I/O are part of the language itself, not libraries you bolt on. No `package.json`. No ORM. No middleware boilerplate. You write logic; the runtime handles the rest.

It started in December 2025 as a question: *can an AI generate a working language runtime from scratch?* It went through a Python prototype, a C rewrite, and finally a Rust implementation that became XCX 3.x. XCX 4.2 is the current release, built on a redesigned VM and JIT with substantially improved performance. One contributor so far.

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
| Platform support | Windows (primary) / Linux / macOS |

XCX is not trying to replace Go or Node. It occupies a different space: small backend services and tools where you want zero dependency setup and a language that knows what you're building. The trade-off is an early-stage ecosystem and a single contributor.

---

## Performance

With XCX 4.2, the benchmark suite was overhauled and moved to its own repo:
**[xcxlang-org/xcx-benchmarks](https://github.com/xcxlang-org/xcx-benchmarks)**.
This is the new Main Suite, run under a stricter, more transparent methodology
than previous releases.

> ⚠️ These benchmarks reflect the **current state of XCX 4.2**.
> The runtime, VM, and JIT are still under active development and will change.
>
> The goal of this section is **transparency**, not competition.

### Performance Benchmarks Results Table (JSON penalty method: PROPORTIONAL)

| Rank | Language | Category | fib(30) (ms) | lcg(100m) (ms) | sieve (ms) | json (ms) | Geom Mean (ms) | Rel. Slowdown |
|---|---|---|---|---|---|---|---|---|
| 1 | zig | AOT | 1.69 | 5.39 | 11.89 | 0.05* | 1.53 | 1.00x |
| 2 | rust | AOT | 1.81 | 21.44 | 12.40 | 0.08* | 2.52 | 1.65x |
| 3 | c | AOT | 1.16 | 85.87 | 8.36 | 0.10* | 3.03 | 1.98x |
| 4 | cpp | AOT | 1.17 | 84.92 | 17.50 | 0.13* | 3.87 | 2.52x |
| 5 | v | AOT | 1.50 | 190.50 | 12.00 | 0.16* | 4.85 | 3.17x |
| 6 | crystal | AOT | 2.67 | 106.99 | 22.34 | 0.20* | 5.97 | 3.90x |
| 7 | go | AOT | 3.09 | 236.66 | 12.65 | 0.22* | 6.75 | 4.41x |
| 8 | java | JIT | 2.74 | 220.36 | 24.67 | 0.26* | 7.91 | 5.16x |
| 9 | bun | JIT | 5.41 | 403.19 | 29.57 | 0.41 | 12.71 | 8.30x |
| 10 | nim | AOT | 18.00 | 194.50 | 20.50 | 0.44* | 13.37 | 8.73x |
| 11 | **xcx** | JIT | 12.48 | 108.07 | 102.26 | 0.25 | 13.60 | 8.88x |
| 12 | pypy | JIT | 18.82 | 119.94 | 99.86 | 0.33 | 16.48 | 10.75x |
| 13 | csharp | JIT | 5.67 | 210.67 | 16.62 | 5.64 | 18.29 | 11.94x |
| 14 | luajit | JIT | 6.90 | 381.38 | 89.46 | 0.66* | 19.86 | 12.96x |
| 15 | node | JIT | 7.04 | 1384.39 | 30.96 | 0.60 | 20.61 | 13.45x |
| 16 | php | INTERPRETED | 62.71 | 1782.28 | 389.90 | 0.83 | 77.55 | 50.62x |
| 17 | lua | INTERPRETED | 68.00 | 2337.00 | 468.00 | 4.50* | 135.27 | 88.30x |
| 18 | python | INTERPRETED | 98.45 | 14569.10 | 1016.23 | 0.44 | 159.14 | 103.88x |
| 19 | erlang | JIT | 5.50 | 4806.50 | 11593.50 | 7.22* | 216.86 | 141.56x |
| 20 | ruby | INTERPRETED | 57.62 | 21827.34 | 542.82 | 10.32 | 289.72 | 189.12x |
| 21 | perl | INTERPRETED | 349.93 | 4376.32 | 2391.05 | 16.50* | 495.77 | 323.62x |
| 22 | R | INTERPRETED | 570.00 | 11770.00 | 930.00 | 19.71* | 592.15 | 386.54x |

`*` = JSON value is a computed stdlib penalty, not a direct measurement (see methodology).

XCX 4.2 ranks 11th by geometric mean, ahead of PyPy, C#, LuaJIT, Node.js, and
every interpreted language tested. `lcg` and `fib` are competitive with JIT
peers; `sieve` remains the main target for optimization in upcoming releases.

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
  -> JIT          Cranelift tracing JIT, hot loops compiled to native machine code
```

**Value representation:** every value is a 16-byte `{ bits: u64, tag: u64 }` struct. The explicit integer tag means zero bitwise operations when reading the type in the interpreter. Scalars (int, float, bool, date) require zero heap allocation. Pointers to heap objects (strings, arrays, JSON, tables, fibers) are packed into `bits`. The JIT uses NaN-boxing internally (Cranelift registers hold a single NaN-boxed `u64`), with `pack_value`/`unpack_value` adapters at the boundary, which keeps CPU register usage lower and block signatures simpler in compiled traces.

**Fibers** are cooperative coroutines backed by saved `Vec<Value>` state. Not OS threads. Suspend/resume moves the locals vector without copying. Each HTTP handler runs as a fiber; the server spawns N OS worker threads, each with its own executor. Globals are shared via `Arc<RwLock<Vec<Value>>>`. Fiber scoping works correctly on all platforms.

**JIT**: backward jumps (loop edges) are counted per instruction pointer. After 50 visits to a given IP, trace recording starts. The threshold is configurable via `--threshold` / `--th`. The trace is specialized for the runtime types seen (integer guards, float guards), then compiled by Cranelift to native code. Functions have a separate threshold: compiled from the 5th call onward. Recursive calls compile to direct native `call` instructions. After 3 guard failures at a given IP, the trace is blacklisted to prevent re-compilation of unstable paths. General string operations are not JIT-compiled, with one exception: the self-concatenation pattern (`x = x + expr`) is recognized and compiled to a dedicated in-place-append fast path.

Full compiler internals: [`documentation/compiler/`](documentation/compiler/)

---

## Project status

XCX 4.2 is best treated as an experimental platform. It is not production-ready, and APIs may change. Expect rough edges.

**What works well:** HTTP servers, SQLite integration, JSON handling, file I/O, cooperative concurrency, interactive terminal programs, and numeric workloads that benefit from JIT-optimized loops.

**Known rough edges:** 

**Linux and macOS**: XCX 4.2 compiles and passes the full test suite on Linux and macOS. Primary development happens on Windows, so Unix-specific issues may take longer to address. If you run into anything platform-specific, please [open an issue](https://github.com/xcxlang-org/xcx/issues).

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

- **4.3**: additional fixes as discovered
- Fibonacci, Sieve, and JSON performance improvements across 4.x
- Better error messages and diagnostics
- PAX package manager stabilization
- Documentation improvements and more example projects
- VS Code extension improvements

### XCX 5.0: language evolution (early planning)

No timeline. Early-stage planning includes `match` statement and pattern matching. No breaking changes to existing 4.x syntax are planned.

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

**Configurable JIT threshold:** `xcx script.xcx --threshold=100` controls how many executions before a fiber segment is JIT-compiled. Default is 50.

---

## Building from source

Requires **Rust 1.75+**.

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