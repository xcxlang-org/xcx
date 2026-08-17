# Compiler — Register Allocation & Scope Tracking

The XCX compile pipeline uses a dense register-based architecture. Locals are allocated sequentially by `FunctionCompiler`; scoped constructs (`if`/`while`/`for` bodies) reuse slots through the scope tracker. There is no separate register-compression pass: `FunctionCompiler` keeps `next_local` and `max_locals_used` in sync during emission, and the chunk's `max_locals` is computed once at `Chunk::new` time.

---

## Module Layout

```
src/compiler/
└── scope_tracker.rs     — scoped local-slot reuse and lambda capture slots (impl FunctionCompiler)
```

---

## Sequential Allocation (`FunctionCompiler`)

- `next_local: usize` — the next free register slot. `push_reg()` returns `next_local as u8` and advances it; `pop_reg()` restores it when a temporary's lifetime ends within an expression.
- `max_locals_used: usize` — the high-water mark of `next_local`, updated on every push. Temporary pops do not lower it.
- Frame sizing: a chunk's `max_locals` (what the executor allocates for the frame) is `max(max_locals_used, next_local)` at chunk construction. Because scoped slots are reused (below), this equals the maximum number of simultaneously live values, without any post-pass over the bytecode.

## Scoped Slot Reuse (`scope_tracker.rs`)

`enter_scope() -> usize` pushes a fresh variable scope and returns the current `next_local`; `exit_scope(saved)` pops the scope and resets `next_local` to the saved value. Registers allocated inside the block (block-local temporaries, condition registers) are therefore available for reuse by later sibling blocks — this is what keeps frames dense despite strictly increasing allocation *within* a block.

`lookup_local(id)` resolves a name to its slot through the scope stack, innermost first. If the name is not local but exists in a parent compiler's locals (lambda capture), the tracker assigns it a leading argument slot: captured variables occupy slots `1..=captures.len()` of the sub-compiler (`0` is the query/row parameter where applicable), appended in first-use order via `self.captures`. `define_local(id, slot)` binds a name to an explicit slot (used for parameters and captures).

## Variable Allocation Rationale

XCX chooses simple sequential allocation with scope-based reuse over graph-coloring allocators. This simplifies bytecode generation at the cost of slightly higher register pressure inside deeply nested expressions, which the 256-register frame budget absorbs comfortably.
