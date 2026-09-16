# HIR — Inlining

Code in `src/hir/pass.rs` (orchestration), `src/hir/inline.rs` (substitution mechanism), and `src/hir/inline_policy.rs` (inlining decision logic).

---

## Policy: `should_inline`

```rust
pub fn should_inline(callee: &HirFunc, caller: &HirFunc, depth: usize, in_loop: bool) -> bool
```

A function is **not** inlined if any of the following conditions are met:

- `in_loop` is true — the call site sits inside a loop body (`While`/`For` in the HIR tree). Substituted locals are remapped into the caller's register space and can overlap the loop's live collection registers, so calls inside loops are never inlined,
- Either `callee` or `caller` is a fiber (`is_fiber`),
- The nesting depth satisfies `depth >= 3`,
- The `callee` contains a recursive reference detected by `check_ref_recursion` (scanning the function body for occurrences of its own name beyond the shallow self-call case),
- Any `return` statement resides inside a loop (`is_return_in_loop`) — an early return from within a loop does not have a straightforward equivalent in the substituted block,
- The function cost (`calculate_func_cost`) is `>= 20`.

**Shallow self-recursion is allowed (4.4).** A simple leaf self-call (`callee.name == caller.name`) is no longer an unconditional bar: the inliner substitutes the body at the call site and leaves the inner self-call as a real call. This is what cuts call overhead for functions like `fib`. Because recursion depth is now measured in physical (post-inlining) frames, equivalent programs may recurse slightly deeper before hitting the limit.

The cost is calculated as the sum of the instruction costs in the body: each instruction has a base cost of `1`, and instructions containing nested blocks (`If`, `While`, `For`, `InlineBlock`) recursively add the costs of their branches/bodies.

---

## Orchestration: `run_inliner_pass`

```rust
pub fn run_inliner_pass(funcs: &mut HashMap<u32, HirFunc>)
```

The pass is iterative — executing at most 3 times (`for depth in 0..3`). This allows multi-level inlining (function A calls B, which in turn calls C; once B is inlined into A, the call to C is exposed and can be inlined in a subsequent iteration) while respecting the depth limit defined in `should_inline`. The loop breaks early if no functions are modified in a given iteration.

For each function, `inline_in_stmt`/`inline_in_block` recursively traverse their bodies, extracting calls suitable for substitution (`extract_calls_from_expr`). New locals required by the inserted callee body are appended to `func.locals`, starting from `next_local = func.locals.len()`.

---

## Substitution Mechanism: `clone_expr` / Local Variable Offset

Since HIR variables are represented as flat indices (`HirLocal = u32`) rather than names in scopes, substituting the body of a `callee` into a `caller` requires remapping **all** `HirExprKind::Local` references in the copied body by a constant offset:

```rust
pub fn clone_expr(expr: &HirExpr, offset: u32) -> HirExpr {
    match &expr.kind {
        HirExprKind::Local(local) => HirExprKind::Local(local + offset),
        // other variants are recursively cloned with the same offset
        ...
    }
}
```

`clone_expr` (and the corresponding `clone_arg`, `clone_range`, `clone_stmt`) recursively copies the entire expression tree, shifting each encountered `Local` by an `offset` equal to the number of local variables already present in the `caller` at the time of substitution. This ensures that the copied `callee` body does not collide with the existing variables of the `caller`. Finally, the function call is replaced by a `HirStmtKind::InlineBlock { stmts, result_local }` — a sequence of substituted statements ending (if the function returns a value) with an assignment to `result_local`, which replaces the original call expression at the call site.

---

## Integration with `Compiler`

The `Compiler` struct (in `src/compiler/compiler.rs`) includes a `disable_inline: bool` field — a global flag set prior to compiling the entire program (not per `CompileContext`) that allows disabling `run_inliner_pass` entirely. `FunctionCompiler` contains the `inline_stack`, `inline_result_locals`, and `local_regs` fields, which are used to allocate registers correctly for substituted `InlineBlock` constructs during the codegen phase (see [hir_codegen.md](hir_codegen.md)).