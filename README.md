![XCX Banner](https://raw.githubusercontent.com/xcxlang-org/xcx-vscode/main/images/banner.png)

![Rust](https://img.shields.io/badge/built%20with-Rust-orange)
![License](https://img.shields.io/badge/license-Apache%202.0-blue)
![version](https://img.shields.io/github/v/release/xcxlang-org/xcx)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)
![GitHub Stars](https://img.shields.io/github/stars/xcxlang-org/xcx?style=flat)
![GitHub Issues](https://img.shields.io/github/issues/xcxlang-org/xcx)
![Last Commit](https://img.shields.io/github/last-commit/xcxlang-org/xcx)
![Repo Size](https://img.shields.io/github/repo-size/xcxlang-org/xcx)

> XCX 4.0 is an active project under development. If you run into something unexpected, [open an issue](https://github.com/xcxlang-org/xcx/issues).

---

## Why XCX exists

Most backend languages make you choose between two bad options: high-level languages that are productive but drag in frameworks, ORMs, and config files you didn't ask for, or low-level languages that give you control but make a simple HTTP endpoint feel like work.

XCX is an experiment in a third path: a statically typed language where HTTP, SQLite, JSON, crypto, and file I/O are part of the language itself, not libraries you bolt on. No `package.json`. No ORM. No middleware boilerplate. You write logic; the runtime handles the rest.

It started in December 2025 as a question: *can an AI generate a working language runtime from scratch?* It went through a Python prototype, a C rewrite, and finally a Rust implementation that became XCX 3.x. XCX 4.0 is a significant architectural step forward on that foundation, with a redesigned VM, resolved fiber scoping issues, and substantially improved performance. One contributor so far.

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
| Windows support | yes | yes | yes | Windows (primary) / Linux |

XCX is not trying to replace Go or Node. It occupies a different space: small backend services and tools where you want zero dependency setup and a language that knows what you're building. The trade-off is an early-stage ecosystem and a single contributor.

---

## Performance

Benchmarks run on Windows 11, Ryzen 7 5800X, 32GB RAM. XCX uses a register-based VM with a tracing JIT (Cranelift) that kicks in automatically on hot loops after ~50 iterations.

> ⚠️ These benchmarks reflect the **current state of XCX 4.0**.
> The runtime, VM, and JIT are still under active development and will change.
> Fibonacci, Sieve, and JSON are targeted for optimization in upcoming releases.
>
> The goal of this section is **transparency**, not competition.

Ranking is sorted by geometric mean across all four benchmarks.

| # | Language / Platform | Loop (100M) | Fib (30) | Sieve | JSON |
|---|---|---|---|---|---|
| 1 | C++ | 84.76ms | 1.03ms | 0.09ms | 2.17ms |
| 2 | C | 85.09ms | 1.01ms | 0.10ms | 16.45ms |
| 3 | Rust | 29.52ms | 1.79ms | 0.12ms | 53ms |
| 4 | Crystal | 90.9ms | 2.96ms | 0.29ms | 13.94ms |
| 5 | V | 89.45ms | 1.32ms | 0.16ms | 64ms |
| 6 | Java | 34.1ms | 2.2ms | 2.1ms | 8.53ms |
| 7 | Go | 84.42ms | 3.27ms | 0.10ms | 60.46ms |
| 8 | C# | 108ms | 6ms | 0.15ms | 100ms |
| 9 | Nim | 89ms | 18ms | 0.2ms | 58.9ms |
| 10 | Node.js | 358.89ms | 6.54ms | 2.28ms | 8.12ms |
| 11 | **XCX 4.0** | **119ms** | **14.28ms** | **2.55ms** | **22.74ms** |
| 12 | LuaJIT | 378ms | 9.1ms | 0.8ms | 119ms |
| 13 | Erlang | 157.08ms | 6.04ms | 74.65ms | 150.02ms |
| 14 | PHP | 3219.35ms | 80.33ms | 4.21ms | 10.83ms |
| 15 | Python | 11094.20ms | 100.65ms | 3.72ms | 38.15ms |
| 16 | Ruby | 27937.31ms | 65.56ms | 5.43ms | 33.34ms |
| 17 | Lua | 5766ms | 82.8ms | 7ms | 374ms |
| 18 | R | 23327ms | 580ms | 3ms | 357.48ms |
| 19 | Perl | 11036.03ms | 390.53ms | 17.18ms | 717.61ms |

XCX 4.0 ranks 11th by geometric mean, ahead of LuaJIT, Erlang, and all scripting languages. Loop performance lands ahead of Node.js and LuaJIT, and within the same order of magnitude as Go, Nim, and C#. This is a substantial improvement over 3.1 (520ms loop), driven by the redesigned VM and JIT improvements. Fibonacci, Sieve, and JSON are still being optimized and will improve in upcoming 4.x releases.

---

## Architecture

XCX compiles source code through a multi-stage pipeline, all implemented in Rust (~34.1k lines including test files):

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

**Fibers** are cooperative coroutines backed by saved `Vec<Value>` state. Not OS threads. Suspend/resume moves the locals vector without copying. Each HTTP handler runs as a fiber; the server spawns N OS worker threads, each with its own executor. Globals are shared via `Arc<RwLock<Vec<Value>>>`. Fiber scoping now works correctly on all platforms; the Windows workaround present in 3.x is no longer needed.

**JIT**: backward jumps (loop edges) are counted per instruction pointer. After 50 visits to a given IP, trace recording starts. The trace is specialized for the runtime types seen (integer guards, float guards), then compiled by Cranelift to native code. Functions have a separate threshold: compiled from the 5th call onward. Recursive calls compile to direct native `call` instructions. After 3 guard failures at a given IP, the trace is blacklisted to prevent re-compilation of unstable paths. String operations are not currently JIT-compiled.

Full compiler internals: [`documentation/compiler/`](documentation/compiler/)

---

## Project status

XCX 4.0 is best treated as an experimental platform. It is not production-ready, and APIs may change. Expect rough edges.

**What works well:** HTTP servers, SQLite integration, JSON handling, file I/O, cooperative concurrency, interactive terminal programs, and numeric workloads that benefit from JIT-optimized loops.

**Known rough edges:** String operation performance (no JIT coverage), and some known internal architectural issues being addressed in the 4.x line. Fibonacci, Sieve, and JSON performance are targeted for improvement.

**Linux**: XCX 4.0 compiles and passes the full test suite on Linux. Primary development happens on Windows, so Linux-specific issues may take longer to address. If you run into anything platform-specific, please [open an issue](https://github.com/xcxlang-org/xcx/issues).

The ecosystem is minimal and evolving. APIs and internal behavior may change across minor versions.

Contributions are welcome; bug reports and pull requests are appreciated. There is no formal contribution process yet. For larger changes, please open an issue first.

---

## Roadmap

### XCX 4.x: stabilization and known fixes

The 4.x line focuses on fixing known architectural issues and improving runtime correctness and performance:

- **4.1**: map iteration order correctness; call-site argument register mapping fix
- **4.2**: method dispatch refactor; compiler module consolidation
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

**1. Download** the installer from [Releases](https://github.com/xcxlang-org/xcx/releases): `xcx-setup.exe` (Windows) or the Linux binary.

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

**JSON as a first-class type:** raw literals `<<< {} >>>`, `.bind()`, `.set()`, `.inject()`. JSON is how you talk to the outside world.

**Built-in HTTP:** client (`net.get/post/put/delete`) and server (`serve:`). Routes, handlers, CORS, and status codes, all in the language.

**Crypto and file I/O:** `crypto.hash`, `crypto.verify`, `crypto.token`, `store.read/write/append/glob/zip`.

**Terminal + interactive input:** raw mode, cursor control, non-blocking key input. Enough to build games, editors, and CLI tools.

**PAX package manager:** `xcx pax install pkg`. Own registry, beta stage; functional and usable, but API may still change.

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

Translated versions of the documentation (Polish, French, Russian, Chinese, Japanese, and more) are available at [github.com/xcxlang-org/xcx-docs](https://github.com/xcxlang-org/xcx-docs). Note that translations were generated with AI assistance and may contain inaccuracies; the English docs in this repository are always the canonical, up-to-date reference. The compiler internals documentation translated into other languages, available at [xcx-docs](https://github.com/xcxlang-org/xcx-docs), currently lags behind the XCX 4.0 architecture in some languages. The English compiler documentation linked below is fully aligned with 4.0. We are working on bringing the translations up to date, if you'd like to help, please open an issue and let us know which language you want to translate.

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

### Package manager

| Topic | File |
|---|---|
| PAX manual | [`pax_manual.md`](documentation/pax/pax_manual.md) |

---

## License

Apache 2.0 - see [LICENSE](LICENSE)
