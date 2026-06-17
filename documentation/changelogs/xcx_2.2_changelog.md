# Changelog – XCX 2.2

Changes relative to XCX 2.1. Mainly affects the execution layer (bytecode compiler and virtual machine) and adds a JIT backend.

## New features

### JIT compilation for hot loops (Cranelift)

Added a new `backend::jit` module, based on Cranelift, which compiles repeatedly executed loops to native machine code.

- The virtual machine counts the number of passes through each instruction address that is the target of a backward jump (a loop). After exceeding a threshold of 50 iterations, the VM starts recording the sequence of operations executed in the loop (`Trace`).
- The recorded sequence is translated into Cranelift IR and compiled to native machine code, which is cached and used on subsequent entries into the same loop instead of interpreting bytecode.
- The JIT covers a limited set of operations: integer and floating-point arithmetic, comparisons, reading/writing local and global variables, logical operations, and loop control. Operations outside this set (e.g. collection access, method calls, I/O) are not recorded.
- Every operation dependent on a value's type is preceded by a guard (`GuardInt`/`GuardFloat`/`GuardTrue`/`GuardFalse`). If the type assumption doesn't hold during execution of the native code, control falls back to the bytecode interpreter at the corresponding instruction address.

## VM architecture and performance

### Register-based bytecode instead of stack-based

`OpCode` was redesigned from a stack-based model (e.g. `Add`, with operands popped from the stack) to a register-based model, where instructions carry explicit `dst`/`src1`/`src2` indices. This applies to the entire instruction set: arithmetic, comparisons, collection operations, function calls, HTTP/store/JSON/fiber operations, etc.

### `Value` representation as NaN-boxing

The `Value` type, previously a tagged enum (`Int`, `Float`, `String`, `Array`, etc., with some variants heap-allocated), was replaced with a single 64-bit word using the NaN-boxing technique (`struct Value(pub u64)`). Integers, floats, and booleans are encoded directly in the word; composite types (string, array, set, map, table, JSON, fiber, date) are encoded as a pointer with a tag in the upper bits.

### New compiler-generated loop instructions

The compiler detects the pattern of incrementing a local/global variable at the end of a loop body (`x = x + 1`) and replaces it with dedicated instructions (`IncLocal`, `IncVar`), merging the loop-continuation check with the increment into a single instruction (`IncLocalLoopNext`, `IncVarLoopNext`, `LoopNext`). These same instructions form the basis for trace recording in the JIT.

## Bug fixes

- **`for` iteration over a `Set` value.** In 2.1, the `for x in mySet { ... }` loop used the same compilation path as iteration over an `Array`, which assumed the array's internal data layout. In 2.2, a separate branch (`ForIterType::Set`) was introduced, which first converts the set to an array via `.values()` before iterating.
- **`Int - Date` subtraction.** In 2.1, this type combination wasn't handled in any branch of the subtraction operator and as a result returned the default value `Bool(false)` instead of a result. In 2.2, both the type checker and the VM correctly handle `Int - Date` (analogous to the already-existing `Date - Int` and `Date - Date`), returning the difference in days.
- **CLI process exit code.** In 2.1, the process always exited with code 0, regardless of whether runtime errors occurred during program execution (e.g. via `halt.error`). In 2.2, if the VM's error counter is greater than zero after execution finishes, the process exits with code 1.

## Dependencies and build configuration

- Added dependencies required by the JIT backend: `cranelift`, `cranelift-codegen`, `cranelift-frontend`, `cranelift-native`, `cranelift-module`, `cranelift-jit` (0.130.0), and `target-lexicon` (0.13.5).
- Added a `[profile.release]` section (`opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `panic = "unwind"`, `strip = false`, `debug = false`) — previously the release build used Cargo's default settings.

## Known limitations

- Register indices in the new bytecode are typed as `u8`, which limits the number of simultaneously live local/temporary variables within a function to 256. There is currently no explicit validation of this limit in the compiler — exceeding it isn't reported as a compile error.
