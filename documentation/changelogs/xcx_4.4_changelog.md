# XCX 4.4 - Changelog (4.3 -> 4.4)
**Type: RELEASE**

---

## VM: interpreter (`--no-jit`) performance

Targeted, VM-internal optimizations of the interpreter hot path. No bytecode-format changes, no architectural redesign.

- **[NEW]** Global variable access no longer takes a lock. Reads/writes go through a cached raw pointer to the fixed-size globals array (65536 slots, never resized), the same unsynchronized model the JIT path already used. Refcount discipline preserved; `SetName` keeps the locked path since it can register new names.
- **[NEW]** All six comparison opcodes gained a dedicated int-int fast path comparing raw 64-bit payloads directly, instead of ranking both operands first.
- **[REF]** Constant-pool names are now resolved once per constant index instead of on every execution.
- **[NEW]** Array index get/set skip the lock acquire when the executor register uniquely owns the array; shared/locked path unchanged.
- **[NEW]** String append appends bytes directly from the source allocation when safe, instead of always cloning through an intermediate buffer first.
- **[REF]** The interpreter dispatch loop polls the shutdown flag every 1024 instructions instead of every instruction.

---

## JIT: deterministic code generation

- **[FIX]** The same binary used to produce two speed classes at random (~50/50 split) across process runs, because global/local variable ordering came from a randomly-seeded hash set. Switched to a deterministic ordered set, and spill/reload emission now sorts by index instead of iterating a hash map. This removes the register-allocation lottery that caused the split.

---

## JSON: single-pass deserialization

- **[NEW]** JSON values now deserialize directly into the runtime's object representation instead of building an intermediate generic tree and re-walking it.
- **[FIX]** Key ordering/duplicate-handling semantics preserved exactly as before.
- **[REF]** Parsing a string argument now borrows the payload instead of copying it, and skips a wasted cache-lookup hash for payloads too large to be cached.

---

## JIT: loop-header alignment

- **[NEW]** The JIT compiler now aligns hot loop headers to 64-byte boundaries (via a throwaway probe build + padding), removing a code-layout dependent slowdown where a loop's speed varied depending on unrelated code emitted before it.
- **[CHG]** JIT compile time roughly doubles for chunks containing loops (extra probe build). Runtime semantics unchanged.

---

## HTTP: request body size limit (413)

- **[NEW]** Request bodies over **10 MB** are rejected with `413 Payload Too Large` before the handler runs.
- **[REF]** Requests with a declared `Content-Length` over the limit are rejected without reading the body; undeclared/chunked requests are bounded to limit+1 bytes read.

---

## Runtime: memory watchdog + json.parse retention fix

- **[NEW]** A background watchdog now polls process memory every 100ms and terminates the process with a fatal error if it crosses a configurable ceiling (default 8GB, overridable via env var, 0 disables it).
- **[FIX]** Fixed a `json.parse` reference-counting bug that leaked the entire parse tree on every call (both interpreter and JIT-FFI paths), plus leaks on `.get()`/`.first()`/custom-get results. 5 parses of a 6.4 MB payload (100k users), peak private commit, `--no-jit`: 329.3 → 86.2 MB.
- **[KNOWN ISSUE, deferred]** A related JIT-mode leak (loop-carried heap values not released across loop back-edges) is not yet fixed; the watchdog remains the interim protection.

---

## Compiler: `.len()` alias

- **[FIX]** `.len()` was documented as an alias of `.size()`/`.count()` on arrays but wasn't actually implemented; it now works end-to-end.
- **[FIX]** Same alias was also missing on json (object + array), set, and map member access; now implemented consistently.
- **[FIX]** `map.isEmpty()` was documented but not implemented; now it works.

---

## Fixes: shutdown flag, halt semantics, method-argument limit

- **[FIX]** A duplicated shutdown flag meant cooperative (Ctrl-C) shutdown could never actually trigger; consolidated into one flag.
- **[FIX]** Method calls with more than 16 arguments no longer crash the process; they are now rejected at compile time with a clear error.
- **[FIX]** Network request rejections (SSRF guard) no longer raise raw crashes; they now follow proper halt semantics (fatal/error) consistently between JIT and interpreter.
- **[FIX]** Invalid JSON in `json.parse()` no longer raises a raw crash; it now raises a proper documented fatal error, consistent across both execution modes.
- **[FIX]** Halt diagnostics (fatal/error/alert messages) were worded differently between JIT and interpreter modes; now identical.
- **[FIX]** Running the CLI on a non-existent file now correctly exits with a failure code instead of silently succeeding.

---

## HIR: shallow self-recursion inlining

- **[NEW]** Simple leaf self-recursive functions (like `fib`) can now be partially inlined, cutting call overhead significantly.
- **[CHG]** The recursion depth limit still applies but is measured in physical (post-inlining) frames, so equivalent programs may now recurse a bit deeper before hitting it.

---

## Language/runtime: documented-behavior audit fixes

A round of fixes for behavior that was documented but not actually implemented correctly, found via a scenario-based audit:

- **[FIX]** Date `.hour`/`.minute` now consistently use local time (previously mismatched `.format()`'s local view vs. properties reading UTC); default date formatting and local-midnight storage also corrected to match documentation.
- **[FIX]** `string.size()` was silently returning `false` instead of erroring or working; it is now correctly a compile-time error, matching the documented string API.
- **[FIX]** Database `.remove(table).where(filter)` now implemented as documented.
- **[FIX]** Database `@optional` columns with SQL NULL now materialize as the column type's proper default instead of always `false`.
- **[FIX]** HTTP response objects now include the documented `text` field (raw response body) in all cases (success, error status, and connection failure).
- **[FIX]** Adding floats to strings (`"x" + 0.5 + 0.5`) in JIT mode could silently produce garbage instead of proper string concatenation; fixed to match interpreter behavior.
- **[FIX]** `.toStr()` on certain scalar values incorrectly returned `false` in JIT mode; fixed.
- **[FIX]** JSON element mutation through `.get(i)` could silently fail to persist, and `.bind()` used as an expression silently did nothing; both are fixed, mutations now consistently propagate, and `.bind()` correctly writes its target.
- **[FIX]** Recovering from a `halt.error` inside a function with a return type used to hand the caller an untyped/incorrect value; it now correctly yields the type's proper default value.
- **[FIX]** Sequential `for` loops over fibers could corrupt fiber state (stale reused variable slot) and crash on a later `.isDone()` call; fixed by always allocating a fresh loop variable for fiber/array/set loops.

**Note (design decision, not a bug):** iterating a fiber with `for` intentionally yields the fiber's final return value as its last iteration item; this is deliberate and documented behavior, not a defect.

---

## Runtime: remaining crash-style failures + HTTP handler serialization

- **[FIX]** `.toInt()` / `.toFloat()` on a non-numeric string crashed the process in JIT mode with a raw Rust panic (exit 127); it now raises the same `halt.error` as the interpreter (whitespace-trimmed parse included), and failing casts count toward the process error tally in both modes (exit 1).
- **[FIX]** `push` through a JSON path that does not resolve to an array crashed with a raw panic (JIT exit 127; nested paths crashed the interpreter too); it now raises a proper `halt.error` ("push target is not an array") in both modes.
- **[FIX]** Concurrent HTTP requests raced on shared VM state: a counter handler under 400 parallel requests lost 29 updates. Handler execution is now serialized; request I/O (body reads, response writes) stays parallel. Trade-off: handlers no longer run in parallel with each other, and a handler calling the server's own HTTP endpoint from inside would deadlock; full concurrent-state support remains a design decision.
- **[FIX]** The JIT `json.parse` cache matched entries by string pointer alone; after allocator address reuse it could return the parsed tree of a different document. Entries now match by content only.

---

## Cleanup: duplicate json.parse cache and dead VM modules removed

- **[CHG]** The JIT maintained a second, private `json.parse` cache next to the interpreter's, with different size, eviction, and a pointer-keyed lookup that could match a freed-and-reused string address (returning the parsed tree of a different document). The JIT now uses the interpreter's parse path directly: one cache, one set of semantics; `json.parse` on strings with invalid UTF-8 no longer relies on unchecked UTF-8 decoding.
- **[CHG]** Removed the unused legacy `vm/frame` and `vm/stack` modules (`CallFrame`, `FrameStack`, `ValueStack`, `FiberFrame`), remnants of an older stack-based design that nothing instantiates anymore.
- **[FIX]** The JIT seed type-inference no longer invents operand facts: an arithmetic op's result type is derived from its operands only (same precedence rules as the main analysis, string-before-float), instead of copying the destination type back onto unknown sources, which could mark a string register as Int/Float for an unchecked specialization.

---

## JIT: globals written inside a function are now visible after the call

- **[FIX]** After a user-function call returned, the caller kept stale cached globals unless the callee touched the heap: a function writing a global was invisible after return. Repro: `func bump() { g = g + 1; }` called three times left `g = 0` under the JIT (`--no-jit`: 3); the ledger program's transfers and error counters silently did not apply. Callers now refresh cached globals after every user-function call, narrowed to the callee's transitive global-slot write set (fixpoint over the Call graph, recursion-safe); callees that write no globals add zero reload cost.
- **[FIX]** The JIT inferred parameter types with a flow-insensitive bytecode scan that could back-propagate a wrong type onto a parameter (a `string` parameter typed `Bool` via a comparison's destination register). The poisoned type corrupted spill-time tag reconstruction (int values spilled with a Bool tag), misrouted method dispatch, and made string-returning functions return garbage. Parameters are now seeded from their declared signature types; other registers keep the existing flat inference.

---

## Halt parity: JIT fast paths follow interpreter error semantics

- **[FIX]** `"str".replace("", x)` silently performed a Rust replace instead of raising R307; out-of-bounds string `slice` returned an empty string instead of raising R303; `map.get` on a missing key returned `0` instead of raising halt.error. All three now match the interpreter (same codes, same messages, error tally incremented).
- **[FIX]** Fallible JIT fast paths (`replace`, `slice`, `map.get`, array `get`/`insert`/`delete`/`update` out-of-bounds branches) now check the error tally like the generic dispatch does; the in-bounds array get/set fast paths remain poll-free, so loop-heavy benchmarks are unaffected.

---

## Database and tables: `@default(v)` is honored everywhere

- **[FIX]** `@default(v)` was reduced to a boolean flag before code generation, so `CREATE TABLE` carried no `DEFAULT` clause: inserts omitting the column stored SQL NULL, `NULL < x` filters never matched (`remove(...).where(...)` silently kept rows), arithmetic on such columns returned NULL, and string defaults read back empty. The declared constant now reaches the SQL schema (`DEFAULT 0`, `DEFAULT 'user'`, …).
- **[FIX]** In-memory table rows filled omitted `@default(v)` cells with a bare `false` regardless of column type; omitted cells now take the declared default (or the column type's zero default when no `@default` is declared), matching the SQL-side behavior.

---

## JIT diagnostics, net error typing, and spill-store elision

- **[FIX]** JIT code-finalization failures now report the chunk name and the real Cranelift/module error at all three compile sites; the guessed "likely FreeBSD W^X policy" wording (misleading on Windows) is gone. Fallback-to-interpreter behavior unchanged.
- **[FIX]** `net.*` with a malformed URL (no scheme / empty host) is no longer reported as an SSRF attack. Malformed URLs return a plain `INVALID` error and surface as a normal request failure (`ok=false` + `error`); real SSRF detections keep their exact halt semantics (`file://` and link-local → `halt.fatal`, private ranges → `halt.error`).
- **[NEW]** `spill_all` dead-store elision: a dirty register that is never read anywhere in the chunk (explicit per-opcode read-set analysis, argument windows included) and statically holds a scalar skips its memory store at every call/dispatch boundary.

---

## Fixes: global string append ownership + bare math-constant resolution

Both defects are old (reproduced identically on the 4.2 and 4.3 release binaries).

- **[FIX]** Appending to a global string (`s = s + x`, interpreter `--no-jit`) could mutate a buffer shared with the chunk's constant pool: a string initialized from a literal **after** another string had been appended to started life containing that string's content (silent wrong results). Repro: accumulate one string in a loop, then `s: dec = "";` and append into it; `dec` began with the first string's content. The in-place append now requires the globals slot to be the buffer's sole owner, and the re-allocation path releases the construction reference so the ownership count reflects real owners (previously a phantom construction reference is what the old `<= 2` threshold implicitly compensated for). The JIT fast append keeps its own threshold: there the second reference is the JIT's managed register copy, released immediately after the call.
- **[FIX]** Bare identifiers named `PI`, `E`, `TAU`, `PHI`, `SQRT2`, `LN2`, `LN10`, `INF`, `INT_MAX`, `INT_MIN` failed to resolve (`S101: Undefined variable: <alias>.X`, diagnostic printed twice) in any program containing at least one aliased `include`, even when the name was defined locally or merged from an unaliased include. The include expander now rewrites such names only towards an aliased module that actually declares the symbol (first match in include order, deterministic); otherwise the name resolves normally.

---

## Language: `elf` and `els` keyword aliases removed

- **[CHG]** The conditional keyword aliases `elf` (of `elseif`) and `els` (of `else`) no longer parse; code using them fails to compile with a plain unknown-identifier error. `elseif` / `elif` / `else` remain. The language now has exactly one optional short form per keyword instead of three spellings. Update affected sources by replacing `elf` with `elseif` (or `elif`) and `els` with `else`.

---

## Language: block semicolons are optional, bare style recommended

- **[CHG]** The semicolon after block syntax is now optional everywhere: after `then` (in `if`/`elseif`), after `do` (in `while`/`for`), after `else`, after `end`, and after the closing `}` of `func`/`fiber` definitions and `{ }`-shaped declarations (`array`/`set` literals, `map`/`table`/`database`/`serve`). Previously the optional form worked only after `if`'s `then`; the other positions required `;`.
- **Recommended style:** write blocks without the decorative semicolon: `if (ok) then … end`, `func f() { … }`. Statement separators inside block bodies are unchanged and still required.
- **[PLANNED 5.0a]** The semicolon form is removed in 5.0a (see the note at the end of this changelog); the bare form is its only successor.

---

## Coming in XCX 5.0a (syntax): blocks take no semicolons

- **[PLANNED]** In 5.0a the semicolon after block keywords and closing braces is removed: write `then`, `do`, `else`, `end` and the closing `}` of `func`/`fiber` definitions and `{ }`-shaped declarations without `;`. Statement semicolons are unchanged. Both forms compile in 4.4 (optional since this 4.4 change); the semicolon form is removed in 5.0a, which will ship an automated migration script. New code should use the bare form.

---

## Verification

- `cargo test --release`: 251 tests / 21 files, 0 failed (incl. RUN-004 R307 under the JIT); scenario suite 104/104 passed (SUITE-25..46 + MIN-25); every `programs/` suite passes in JIT and `--no-jit`.

---

## Performance

Official benchmark run, Main Suite, XCX 4.4 vs XCX 4.3 (JIT and `--no-jit`):

| Suite | FIB (30) | LCG (100M) | SIEVE | JSON |
|---|---|---|---|---|
| XCX 4.3, JIT | 11.11 ms | 107.81 ms | 36.542 ms | 0.69 ms |
| **XCX 4.4, JIT** | **6.53 ms** | **105.87 ms** | **34.287 ms** | **0.55 ms** |
| XCX 4.3, `--no-jit` | 202.93 ms | 4303.23 ms | 2043.74 ms | 0.86 ms |
| **XCX 4.4, `--no-jit`** | **179.41 ms** | **4265.23 ms** | **2031.74 ms** | **0.61 ms** |

- **JIT mode improved on all four tests:** fib −41.2% (shallow self-recursion inlining), lcg −1.8%, sieve −6.2%, json −20.3%.
- **Interpreter mode improved on all four tests:** fib −11.6%, lcg −0.9%, sieve −0.6%, json −29.1%. The headline interpreter result of the cycle is the loop suite: total **−17.3%** vs 4.3 (see the interpreter performance section above).
- Full comparison against 26 languages/runtimes (XCX 4.4 ranks 11th by geometric mean): see the table in [`README.md`](../README.md).

---

## Known issues at the end of the 4.4 cycle

- **JIT code finalization can fail intermittently on Windows.** Symptom: the process occasionally prints `[XCX] JIT disabled: code finalization failed for `<chunk>`: <error>` and continues in interpreter mode (results are correct, just slower). Observed twice under a batch runner, never in direct single-file runs. All three compile sites now log the real underlying Cranelift/module error (previously a guessed "FreeBSD W^X" wording), so the next occurrence identifies the cause. The fallback itself is safe and unchanged.
- **`json.parse` leak in JIT mode when the parse result is carried across a loop back-edge.** Minimal situation: JIT mode (default), a loop that parses into a variable surviving the loop:
  ```xcx
  for i in 1 to 50 do;
      json: data = json.parse(big_payload);
  end;
  ```
  Process private memory grows ~48 MB per iteration and is never released (interpreter `--no-jit` is fixed and stays flat). Workaround until the fix lands in 5.0a: parse outside the loop, or limit iterations; the memory watchdog (8 GB default ceiling, `XCX_MAX_RAM_MB`) terminates a runaway process with a fatal error.
- **HTTP handler execution is serialized.** Request I/O (body reads, response writes) stays parallel, but handlers share unsynchronized VM state and run one at a time: a counter handler sees no lost updates, but two handlers cannot execute simultaneously, and a handler calling the server's own endpoint from inside would deadlock the server. Full concurrent-state support is a deliberate 5.0a design decision.
- **Map keys containing `.`, containing `[`, or starting with `/` are interpreted as nested paths, not literal keys.** Repro: `m.insert("a.b", 1)` does not create a top-level key `"a.b"`; `m.size()` stays `0` and `m.contains("a.b")` is `false`. URLs and dotted identifiers therefore cannot be used as keys; key such values by a safe slug/id and keep the string as the value (also documented in the language docs, Maps section). The end-state is a 5.0a breaking-change decision.
- **Residual code-layout sensitivity.** The loop-header alignment pass removes the dominant per-process bimodality, but a hot loop can still land in one of two layouts across process runs of the same binary (observed as a ~1.7× flip in one sieve phase). For strict benchmarking, take a minimum across a few process launches.
- **`while` loop over a function-local limit variable silently decrements the limit.** Inside a function, a loop like `while (d < n)` (also `<=`, `>`, `>=`) where the limit `n` is a parameter or local rewrites `<` to `<=` by decrementing the limit register in the loop prologue; since a bare identifier compiles to the variable's own register, the decrement writes through to the variable itself. Every later read of `n` (in the body and after the loop) sees `n - 1`, with no error in either execution mode. Workaround: copy the limit to a fresh local before the loop (`i: limit = n; while (d < limit) …`). The identical shape at top level and `for` loops are unaffected. Scheduled for a fix in 5.0a.
