XCX 4.3 — Runtime & Compiler (FreeBSD) — EXPERIMENTAL
======================================================

This is an experimental FreeBSD build of XCX 4.3. It is not covered by the
same stability guarantees as the Windows, Linux, and macOS builds.

Contents:
  xcx          — the XCX compiler / VM binary
  install.sh   — user-space installer (plain sh, no extra dependencies)
  lib/         — standard library modules (pax, mathlib, doc)
  resources/   — license

Install:
  ./install.sh

The JIT automatically falls back to the interpreter if the kernel's W^X
policy blocks executable page mapping, so the binary remains usable on
strict configurations.

XCX 4.3 is under active development.
