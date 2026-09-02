# Sema — Type System & Compatibility

The semantic phase validates and restricts runtime types through the representations defined in `src/sema/types/`. XCX attempts to guarantee basic static correctness while remaining massively open to implicit dynamic resolutions via heavily overloaded specific nodes.

---

## Type Typology (`ty.rs`)

The compiler maintains a strict abstraction tree of data shapes:
- **Primitives**: `Type::Int`, `Type::Float`, `Type::String`, `Type::Bool`, `Type::Date`
- **Linear Collections**: `Type::Array(Box<Type>)` and `Type::Set(SetType)`
- **Composite Types**: `Type::Map(Box<Type>, Box<Type>)`
- **Engine Components**: `Type::Fiber(Option<Box<Type>>)`, `Type::Table(TableType)`, `Type::Database`, `Type::DatabaseOperation(DatabaseOpKind, TableType)` (pending DB operations such as an unfiltered `.remove()`)
- **Dynamic Bindings**: `Type::Json`, `Type::Unknown`, `Type::Builtin(StringId)`

---

## The Dynamic Interoperability Philosophy (`compat.rs`)

`compat.rs` executes the `is_compatible(expected, actual)` function. Rather than rigid structural typing, XCX prioritizes fluid interoperability across specific boundaries:

### The `Unknown` Bypass
If either the expected type or the actual type evaluates as `Type::Unknown`, they are universally compatible. `Unknown` represents an unauditable expression (like reading an unspecified database query or a custom function return missing a type sig). The semantic checker implicitly surrenders evaluation down to the runtime JIT to avoid halting builds.

### The `Json` Chimera
`Type::Json` is mathematically compatible with absolutely any expectation across primitive or object assignments. Built-in network outputs and data mutations explicitly return `Json`, and XCX will accept its mapping into integer arguments or array lists dynamically.

### Collection Casting
Arrays and Sets can implicitly down-cast against each other. If an `Array:int` is assigned or passed into an argument expecting a `Set:N`, it triggers no error. `compat.rs` translates the Set's internal abstraction (`N`, `Q`, `Z`, `S`, `B`, `C`) into primitive representations and verifies alignment against the Array's subtype.
Maps enforce standard key-to-key and value-to-value compatibility.

### Table Schema Adherence (`table_type.rs`)
`TableType` columns carry `name`, `type`, `is_optional`, and `has_default`, plus `is_auto`, `is_pk`, and `is_unique` (`is_auto` is load-bearing for row-count checks). For two tables to be compatible, their column layouts must align in length with pairwise `is_compatible` columns (so `Unknown` still bypasses); an empty `TableType` is compatible with any table. Length disparity triggers a type conflict. Set↔Set compatibility is evaluated by base class (`N≡Z`, `S≡C`, `Q`, `B`); `Fiber↔Fiber` checks payload compatibility.

### Numeric Forgiveness (`primitive.rs`)
`is_numeric_compatible` enables specific seamless bridging. `Int` and `Float` perfectly overlap. More surprisingly, `Int` and `Date` intrinsically interoperate, enabling raw timestamp integer mathematics to implicitly compile directly along raw primitives without explicit casting.
