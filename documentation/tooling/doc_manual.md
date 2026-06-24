# doc — XCX Error Code Reference Manual

`doc` is the official offline error code reference tool for XCX. It looks up compiler and runtime error codes and prints a formatted explanation with a short example, directly in your terminal.

---

## Overview

XCX raises errors using short alphanumeric codes (e.g. `S103`, `R305`, `D401`). The `doc` tool maps each code to a human-readable title, series, explanation, and example, without requiring an internet connection.

```sh
xcx doc S103
xcx doc R305
xcx doc list
```

No network access is required — the entire registry is bundled with the project.

---

## Directory Structure

```
doc/
├── project.pax        # Project configuration
├── doc.xcx             # Entry point — argument parsing & dispatch
└── src/
    ├── errors.xcx      # Full error registry (code -> packed entry)
    ├── lookup.xcx      # Code lookup & parsing logic
    └── format.xcx      # Terminal output formatting
```

---

## Installation

```sh
xcx pax clone doc
cd doc
xcx pax run
```

- `xcx pax clone doc` downloads the published package as a local project.
- No `xcx pax install` step is needed — `doc` declares zero dependencies.

---

## Command Reference

`doc` is invoked via `xcx doc <argument>`.

| Command            | Description                                              |
|---------------------|-----------------------------------------------------------|
| `xcx doc <CODE>`    | Look up a single error code and print its explanation.    |
| `xcx doc list`      | Print every registered code, grouped by series.            |
| `xcx doc`           | Print usage help (no argument supplied).                  |

---

## Command Details

### `xcx doc <CODE>`

Looks up a single error code (case-insensitive, whitespace-trimmed) and prints its title, series, explanation, and a short code example.

```sh
xcx doc S103
```

```
  S103  Type mismatch
  Series : Semantic

  The type of the value or expression does not match what is expected.
  Check that you are not assigning a string to an integer, or passing
  the wrong type to a function.

  Example
    i: age = "twenty";  --- S103: expected i, got s
```

If the code does not exist in the registry, `doc` prints a not-found message instead of an explanation:

```sh
xcx doc Z999
```

```
doc: unknown error code 'Z999'
Run 'xcx doc list' to see all available codes.
```

---

### `xcx doc list`

Prints every registered error code, sorted alphabetically and grouped under its series header (`Semantic`, `Runtime`, `Database`, etc.).

```sh
xcx doc list
```

```
  XCX Error Codes

  -- Database --
  D401  Remove without where

  -- Runtime --
  R103  Input type mismatch
  R303  Index out of bounds
  R304  Column not found
  R305  Invalid JSON
  ...

  -- Semantic --
  S101  Undefined variable
  S102  Redefined variable
  S103  Type mismatch
  ...
```

---

### `xcx doc` (no arguments)

Prints short usage help when no code is supplied.

```sh
xcx doc
```

```
doc — XCX Error Code Reference
Usage:  xcx doc <CODE>
        xcx doc list

Examples:
  xcx doc S103
  xcx doc R305
  xcx doc list
```

---

## Error Code Series

Codes are grouped into series by prefix letter:

| Prefix | Series      | Meaning                                                          |
|--------|-------------|-------------------------------------------------------------------|
| `S`    | Semantic    | Compile-time errors caught before execution (types, scope, syntax-adjacent rules). |
| `R`    | Runtime     | Errors raised while the program is executing (VM, I/O, parsing).  |
| `D`    | Database    | Compile-time errors specific to database/table operations.       |

---

## Extending the Registry

The registry lives entirely in `src/errors.xcx`, inside `buildRegistry()`. Each entry is a single pipe-delimited string:

```
CODE|TITLE|SERIES|EXPLANATION|EXAMPLE
```

To add a new code, insert a new line following the existing pattern:

```xcx
reg.insert("S112", "S112|My new error|Semantic|Explanation goes here.|example.code(); --- S112");
```

`src/lookup.xcx` splits this string on `|` into a `json` entry with `code`, `title`, `series`, `explanation`, and `example` fields; `src/format.xcx` renders it to the terminal. No changes to either file are needed when adding new codes — only `errors.xcx` needs updating.

---

## Notes

- `doc` works fully offline; the registry is compiled into the binary at build time, not fetched at runtime.
- Code lookup is case-insensitive and trims surrounding whitespace, so `xcx doc s103`, `xcx doc S103`, and `xcx doc " S103 "` all resolve identically.
- Unknown codes never crash `doc` — they fall through to a graceful "unknown error code" message pointing to `xcx doc list`.