# XCX 4.4.1 - Changelog (4.4 -> 4.4.1)
**Type: RELEASE**

---

Maintenance patch for the 4.4 series. Bug fixes only, no new functionality
and no language changes; the same bugs are also fixed in the 5.0a line.
Scope follows `VERSIONING.md`: PATCH is a pure correction that introduces
no new functionality.

## Compiler

- **[FIX]** A statement beginning with a malformed `const` form (`const(`,
  `const(1);`) no longer hangs the compiler in an infinite loop. It is an
  ordinary compile error, and statement recovery always makes progress.
- **[FIX]** Malformed `while`, `elseif`, `for`, and `include` statements no
  longer silently drop the keyword and re-parse the rest of the line. In
  4.4, `while 1 < 3 do ... end` without parentheses executed its body once;
  the mistake is now reported with the exact expectation.
- **[FIX]** A variable named `has` can be declared and used in expressions.
  4.4 silently dropped every use; the `HAS` infix operator is unchanged.
- **[FIX]** A `---` divider comment no longer corrupts string literals that
  contain `*---`; string contents are opaque to comment handling. Comment
  semantics are unchanged.
- **[FIX]** `.terminal !write` works inside function bodies. In 4.4 it
  compiled to nothing there, so terminal UIs rendered blank frames.
- **[FIX]** `!!` is a single NOT token, as documented. 4.4 lexed it as two
  separate `!` tokens, so `!!x` silently tested the opposite meaning.

## Standard library

- **[FIX]** mathlib's `is_pow2` returned `true` for every multiple of 4
  (`12`, `20`, `96`); it now halves the value until odd and compares
  against 1.
- **[FIX]** `store.zip` returns `false` for a source path without a file
  name, or a non-UTF-8 path, instead of crashing the process.
- **[FIX]** `env.get()` of an unset variable raises the documented
  catchable `halt.error` instead of silently returning an empty string or
  `0`.
- **[FIX]** Bundled packages resolve from any project: include lookup also
  searches the `lib/` directory of the XCX installation (next to the
  binary) after the project's own `lib/`. In 4.4 the bundled mathlib
  shipped but a project without a local copy could not include it.

## Runtime and JIT

- **[FIX]** Direct JIT calls no longer write every cached global before
  calling a function that cannot access those slots. Transitive access
  analysis preserves synchronization across recursion; indirect callbacks
  retain full synchronization.
- **[FIX]** Array method bounds-error paths are kept outside hot loops,
  and simple boolean-guarded integer counters avoid a conditional jump.
  Bounds checks, diagnostics, and integer update semantics are unchanged.
- **[FIX]** Internal security violations — `store` path traversal, absolute
  paths, and illegal `database:` paths — terminate the process immediately
  with the standard `XCX Fatal:` diagnostic and exit code 1, matching the
  `halt.fatal` statement. In 4.4 they raised a panic whose stack unwind
  could hang the VM thread or abort with a raw exception code instead of
  the documented error exit.
- **[FIX]** A `while`-limit variable stays readable and unchanged inside
  the loop body. In 4.4 the loop prologue could clobber it whenever the
  program re-read the limit.
- **[FIX]** Appending to a string variable initialized from a literal can
  no longer rewrite the literal itself; later uses of that literal return
  the original text.
- **[FIX]** Elements of a non-bool array built with `.insert()` read back
  their real values under the JIT; in 4.4 they could read as `true`.
- **[FIX]** Redirected output is plain text: printed lines no longer carry
  the terminal erase sequence, so scripts and CI read program output
  without stripping escape codes. Interactive terminals keep the
  redraw-friendly line endings.

## Performance

XCX 4.4.1 vs XCX 4.4:

| Suite | FIB (30) | LCG (100M) | SIEVE | JSON |
|---|---|---|---|---|
| XCX 4.4 | 6.53 ms | 105.87 ms | 34.287 ms | 0.55 ms |
| **XCX 4.4.1** | **6.32 ms** | **104.51 ms** | **28.74 ms** | **0.52 ms** |

- Main suite improved on all four tests: fib −3.2%, lcg −1.3%, sieve −16.2%, json −5.6%.
- Cross-function calls: 14.79 → 7.62 ms (−48.5%).
- Loop suite total: 1570.17 → 1544.30 ms (−1.6%).
