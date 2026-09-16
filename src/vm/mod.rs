pub mod opcode;
pub mod value;
pub mod object;

pub mod core;
pub mod utils;

/// Single Ctrl-C flag. Declared in `core::executor` — next to the dispatch
/// loop, the JIT call boundary and the HTTP accept loop that poll it — and
/// re-exported here so the Ctrl-C handler in `main` sets the same instance.
pub use core::executor::SHUTDOWN;

pub use opcode::{OpCode, MethodKind, TypeTag};
pub use value::{Value, Tag, QNAN_BASE, TAG_INT, TAG_BOOL, TAG_DATE, TAG_STR, TAG_ARR, TAG_SET, TAG_MAP, TAG_TBL, TAG_FUNC, TAG_ROW, TAG_JSON, TAG_FIB, TAG_DB};
pub use object::{TableObj as TableData, SetObj as SetData, FiberObj as FiberState, RowObj as RowRef, DatabaseObj as DatabaseData, VMColumn, SqlBinding, FiberStatus};

pub use core::vm::{VM, SharedContext, Chunk, OpResult};
pub use core::executor::Executor;
#[cfg(feature = "jit")]
pub use crate::jit::{JIT, JITFunction, MethodJitFunction};

