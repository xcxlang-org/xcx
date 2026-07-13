# Register Corruption Clamping and Default Variable Initialization Fix

## Problem Description
1. **Loop Register Corruption**: Range `For` and optimized `While` loops did not protect their loop bounds / limits (`limit_reg`) from being overridden by temporary register allocations within function bodies (particularly during nested calls and function inlining).
2. **Incorrect Clamping**: Restoring `next_local` to `entry_next_local` unconditionally at the end of statement compilation clashed with fiber actions (`Yield`/`YieldFrom`), which intentionally register persistent local variables that grew the stack frame.
3. **Shared Default Constants**: Default initialization of variables with complex types (`Array`, `Set`, `Map`, `Table`) loaded global compiler constants via `LoadConst`, sharing the same heap-allocated structures across separate loop rounds and accumulating mutations.

## Proposed Resolution
1. **Protected Register Boundaries**: Restructured loop limit reservation (`limit_reg + 1`) to only increase `next_local` (using `.max`), preventing clobbering of local variables or loop bounds when the loop limit is already a local variable/constant.
2. **Safe Clamping**: Altered statement-level register restoration to clamp `next_local` only if an underflow occurred (`compiler.next_local < entry_next_local`), preserving legitimate stack expansions like target allocations in `Yield`/`YieldFrom`.
3. **Dynamic Default initialization**: Implemented runtime initialization (via `ArrayInit`, `SetInit`, `MapInit`, `TableInit` with zero element counts) for default local declarations of complex types instead of fetching references to shared compiler constants.

## Modified Files
* [compile_hir.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/hir/compile_hir.rs)
  * Restructured statement evaluation `compiler.next_local` restoration.
  * Corrected loop boundary protection logic.
  * Added type branches for dynamic local variables default initialization.

## Verification Results

### Regression Testing
Executed `cargo test --release` resulting in complete success:
```bash
test result: ok. 159 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 37.25s
```

### Benchmark Run (N=1000)
Timings and checksum results are clean and uniform across all rounds:
```bash
Round 1: build=3ms  stringify=2ms  parse=4ms  process=0ms  TOTAL=9ms   bytes=196045  active=500  scoreSum=90650
Round 2: build=2ms  stringify=2ms  parse=1ms  process=0ms  TOTAL=5ms   bytes=196045  active=500  scoreSum=90650
Round 3: build=2ms  stringify=2ms  parse=1ms  process=0ms  TOTAL=5ms   bytes=196045  active=500  scoreSum=90650
```

### Full Benchmark Run (N=500,000)
Executed without any memory growth or loops hanging:
```bash
Round 1: build=1201ms  stringify=1191ms  parse=2171ms  process=234ms  TOTAL=4797ms  bytes=103564813  active=250000  scoreSum=22662499999.999996
Round 2: build=1180ms  stringify=1108ms  parse=273ms   process=235ms  TOTAL=2796ms  bytes=103564813  active=250000  scoreSum=22662499999.999996
```
JIT cache correctly speeds up the second and consecutive rounds.
