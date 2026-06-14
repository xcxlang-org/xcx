# Virtual Machine (`src/vm/`)

A summary of documentation defining the executable version of the native virtual machine execution pipeline, which relies on a stack-based structure utilizing quiet NaN-boxing for variable storage (values and objects).

## Directory Index

- **[vm_value.md](vm_value.md):** Specification of machine data memory layout based on quiet NaN-boxing architectural rules, alongside the implementation of base targets in mapped value tags.
- **[vm_opcode.md](vm_opcode.md):** A comprehensive outline mapping the byte rules of Virtual Machine opcodes that control inputs, variable control structures, modification tables, and loop iteration structures.
- **[vm_objects.md](vm_objects.md):** Reference interfaces for memory structures tied to the heap, including abstract objects mapping SQL tables, arrays, maps, and independent assistance iterations mapped to Fibers.
- **[vm_executor.md](vm_executor.md):** Execution loops of the base interpreter driving the execution system. Modifies VM stack pointers with specific allocations while managing Frame dispatch routines.
