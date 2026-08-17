# Runtime & Language Checker Scripts for XCX Benchmarks

**Date:** 2026-08-08  
**Target Directory:** `B:\workspace\xcx_compiler_workspace\xcx-benchmarks`

## Added Files

1. `check_runtimes.ps1` (PowerShell script for Windows / PowerShell Core)
2. `check_runtimes.sh` (POSIX Bash script for Linux / macOS / Git Bash / WSL)

## Purpose and Functionality

The scripts scan system `PATH` for all 22 compilers, runtimes, and interpreters used across the XCX benchmark suites (`Main_Suite`, `loop_suite`, `func & arith`).

### Checked Runtimes (22 Total)

- **XCX** (`xcx`)
- **C** (`gcc`, `clang`, `cl`)
- **C++** (`g++`, `clang++`, `cl`)
- **Rust** (`rustc`, `cargo`)
- **Go** (`go`)
- **Zig** (`zig`)
- **V** (`v`)
- **Nim** (`nim`)
- **Crystal** (`crystal`)
- **C# / .NET** (`dotnet`, `csc`)
- **Java** (`java`, `javac`)
- **Node.js** (`node`)
- **Bun** (`bun`)
- **LuaJIT** (`luajit`)
- **Lua** (`lua`, `lua5.4`, `lua5.3`, `lua5.1`)
- **Python** (`python`, `python3`)
- **PyPy** (`pypy`, `pypy3`)
- **Ruby** (`ruby`)
- **PHP** (`php`)
- **Perl** (`perl`)
- **Erlang** (`erl`, `erlc`)
- **R** (`Rscript`, `R`)

### Execution Details & Safety

- Uses non-blocking command location resolution (`Get-Command` in PowerShell, `command -v` in Bash).
- Prevents script hanging or sub-shell execution locks.
- Outputs status indicators (`[ OK ]` / `[MISS]`), target executable paths, and summary counts.
- Displays verified official download links for any missing runtimes at the end of output.
