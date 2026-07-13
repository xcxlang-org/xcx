# Removal of Deprecated -d and --dump CLI Flags

## What was changed

Removed support for the deprecated `-d` and `--dump` CLI options from the compiler's entry points.

## Why

As part of standardizing the compiler interface, the `-d` and `--dump` flags (which printed bytecode and proceeded with VM execution) are deprecated in favor of `--bytecode` (which dumps bytecode and exits immediately).

## Modified Files

- [main.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/main.rs) (removed `dump_bytecode` checking and parameters)

## Verification Results

Verified that:
- `--bytecode` functions correctly to dump the bytecode.
- `--dump` / `-d` are no longer recognized as valid actions.
- `cargo test --release` passes successfully.
