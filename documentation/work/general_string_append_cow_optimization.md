# General String Concatenation COW Optimization

## Problem Statement
The general string concatenation path (using `Value::add`) had an $O(n^2)$ allocation behavior because it created a new string object and cloned the payload on every operation, even when the left-hand string operand had unique ownership.

## Proposed Solution
Introduce unique-ownership check (`Arc::strong_count == 1 && Arc::weak_count == 0`) inside `Value::add` for `TAG_STR` operands. When ownership is unique, mutate the string payload in-place by extending its character array directly, avoiding redundant allocations.

## Modified Files
*   `src/vm/value/value.rs`: Optimized string addition path in `Value::add`.
*   `src/vm/core/tests.rs`: Added `test_general_string_append_cow` verifying correctness and copy-on-write isolation.

## Benchmarks Results
Testing `bench_general_str.xcx` under 100,000 iterations:
*   **JIT (SajaJIT) Before**: 10,789.916 ms
*   **JIT (SajaJIT) After**: 3,604.117 ms (3x speedup)
*   **Interpreter Before**: 322.107 ms
*   **Interpreter After**: 332.407 ms (virtually unchanged)
*   **LuaJIT (baseline)**: 1032.729 ms
