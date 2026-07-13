# CLI Flag Combinations Support

Support combining compiler CLI option flags using the pipe symbol `|` to allow combinations like `--no-jit | --bytecode` in a single command line argument.

## What was changed
- Preprocessing of argument variables in the main function of the compiler entrypoint.
- Command-line arguments containing `|` are parsed by splitting on `|`, trimming whitespace, and inserting the processed components back into the active `args` list.
- Standalone `|` arguments are discarded.
- Updated the compiler help message (`--help`) with guidance on using `|` to group options.

## Why
- Implemented to enhance command-line ergonomics and support grouping execution/diagnostic flags.

## Which files were modified
- [main.rs](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/src/main.rs)

## Benchmark Results Before and After
- No regressions recorded. All 159 compiler unit and integration tests passed successfully.
- Manual execution tests with combined flag arguments (`--no-jit | --bytecode`) verified successful option parsing behavior.
