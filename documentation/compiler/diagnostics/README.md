# Error Diagnostics System (`src/error/`, `src/sema/error/`)

This directory documents the error and diagnostics system for compile-time syntax lexing/parsing, semantic logic validation, and VM runtime execution errors.

## Directory Index

- **[compiler_errors.md](compiler_errors.md):** Documents visual error reporting via the `Reporter` CLI (ANSI color codes and code squiggles), semantic checking error kinds (`S101`–`S302`), dynamic VM/runtime error codes (`R-Series`), preprocessor inclusion failures, and the thread-local FFI error reporting context (`ACTIVE_VM`).
