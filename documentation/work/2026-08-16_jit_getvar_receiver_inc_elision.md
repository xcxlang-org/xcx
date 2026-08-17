# JIT: elide the receiver inc_ref for global→MethodCall patterns

**Date:** 2026-08-16
**Area:** JIT code generation (`src/jit/`)

## Problem

The compiler emits every method call on a global collection as:

```
GetVar G -> r          ; load receiver
<argument ops>         ; LoadConst / GetVar / arithmetic into r+1..r+n
MethodCall { dst: r, base: r, kind: K, arg_count: n }
```

`emit_get_var` increments the refcount of every pointer global it loads.
The specialized fast-path branches in `emit_method_call` for
`Update`/`Set`/`Push` overwrite the receiver register with the call result
**without releasing the old value**, so that inc_ref was never matched by a
dec_ref — it leaked one reference per call. In a loop this also put one
`inc_ref` FFI call (tag dispatch + atomic RMW + call overhead) on every
iteration, which was the dominant cost of the `sieve` benchmark: `GetVar`
of the boolean array receiver executed ~23M times in the marking loop and
~10M times in the counting loop.

## Change

1. **`src/jit/analysis.rs` — `getvar_inc_elidable(bytecode, types, ip)`**
   Detects the shape above statically: a `GetVar` into register `r`
   followed (through only argument-setup ops that never spill registers,
   never run user code, and never touch `r`) by a `MethodCall` with
   `dst == base == r` whose kind/arg-count/receiver-type selects a
   specialized fast-path branch in `emit_method_call`:
   - `Update`/`Set`, 2 args, receiver typed `Array|BoolArray|Json|Map`
   - `Push`, 1 arg, receiver typed `Array|BoolArray|Json`
   - `Get`, 1 arg, receiver typed `Array|BoolArray|Json|Map`

2. **`src/jit/emit_load_store.rs` — `emit_get_var`** takes `elide_inc`.
   When elided, no `inc_ref` is emitted; the register is marked as an
   *un-owned borrow* in the new `CodegenCtx::unowned_recv_regs` bit set
   (set after `def_local`, which clears the bit for redefined registers —
   setting it before was a bug that caused heap corruption, fixed).

3. **`src/jit/emit_call.rs` — Get branches.** The `Get` fast paths release
   the receiver through their old-value `dec_ref` (dst == base). When the
   receiver is an un-owned borrow, that dec_ref is skipped, making the
   [GetVar without inc] + [Get without dec] pair exactly refcount-neutral.
   `Update`/`Set`/`Push` branches never dec the receiver, so for those the
   elided inc alone removes the previous leak and makes the pair
   refcount-neutral as well.

4. **`src/jit/codegen_ctx.rs`** carries `unowned_recv_regs: [bool; 256]`;
   `def_local` clears a register's bit whenever the register is redefined,
   so a borrow can never outlive its consumer statement.

Callers: `compiler_method.rs` and `compiler_fiber.rs` compute the elision
per GetVar; the (currently dormant) trace compiler passes `false`.

## Soundness

- The borrow exists only between the GetVar and the consumer MethodCall.
  The window scan only permits ops that cannot spill (`spill_all` would
  write the un-owned pointer into the locals array, whose Rust-side
  cleanup assumes ownership) and cannot run user code (a fiber resume or
  lambda could reassign the global and drop the last owned reference).
- The consumer arm is predicted from the same per-IP type analysis
  (`types_at_ip`) that drives emission, so the emit-time branch always
  matches the predicted one.
- Net refcount effect of each elided pair is zero, which also fixes the
  pre-existing +1 reference leak per `x.push(...)` / `x.update(...)`
  statement on global collections (observable as unbounded strong-count
  growth in long-running loops).

## What was tried and rejected

An inlined native `atomic_rmw` inc_ref (replacing the FFI call) was
implemented and benchmarked. It improved sieve to ~76 ms but
systematically landed the nested integer loops (`triple for`, `while up`)
in their slow code-layout mode (~2x slower — see below), a measurable
regression. It was reverted; the layout facts it uncovered are kept as
`ARC_STRONG_COUNT_OFFSET` plus tests in `nan_ops.rs`.

Note: `triple for` / `while up` / `while down` are **bimodal on this
machine independent of any change** (fast ~215 ms / slow ~430 ms modes,
per-process, most likely from randomized `HashMap` iteration order in the
codegen prologue changing register allocation). Verified by sampling the
unmodified FFI-inc build: both modes occur there too. 20-run
distributions before (mean 303, median 219, 5/15 slow) and after
(mean 309, median 255, 5/20 slow) are statistically indistinguishable.

## Numbers

Main Suite (official config: 20 warmup / 100 runs), final binary:

| Benchmark | baseline.json | before (this machine) | after |
|-----------|---------------|----------------------|-------|
| **sieve** | 97.814 ms | 99.19 ms | **34.63 ms** |
| fib(30) | 11.92 ms | 11.30 ms | 11.16 ms |
| lcg(100m) | 106.48 ms | 109.33 ms | 108.52 ms |
| json | 0.15 ms | 0.126 ms | 0.120 ms |

Decomposition (`scratch/sieve_parts.xcx`): marking loops 67–85 ms →
~28 ms, counting loop ~30 ms → ~17 ms.

Func & arith (100 runs): array_alloc 29.58 → 24.7 ms (improved);
cross_func 15.32 → 15.7 ms; inline_arith 0.438 → 0.43 ms.

Loop suite (8 runs): all within the pre-existing bimodal noise band of
the before-measurements; `for continue` improved (603.8 → 523.8 ms).

`cargo test --release`: 199 tests passed, 0 failed (includes the two new
layout/predicate tests in `src/jit/nan_ops.rs`).

## Existing documentation impact

`documentation/compiler/jit/jit_emitters.md` and `jit_codegen.md` remain
accurate: the fast paths they describe are unchanged; this change only
removes the redundant receiver `inc_ref` that fed them. No other work-doc
is invalidated by this change.

## Verification tooling

`xcx-benchmarks/run_xcx_only.py` added — runs only the `xcx` language
across all three suites with the exact warmup/run methodology of
`run_benchmarks.py` and compares against baseline.json.
