pub mod opcode;
pub mod value;
pub mod object;
pub mod stack;
pub mod frame;

#[cfg(feature = "jit")]
pub mod trace;

pub mod core;
pub mod utils;

pub static SHUTDOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub use opcode::{OpCode, MethodKind, TypeTag};
pub use value::{Value, Tag, QNAN_BASE, TAG_INT, TAG_BOOL, TAG_DATE, TAG_STR, TAG_ARR, TAG_SET, TAG_MAP, TAG_TBL, TAG_FUNC, TAG_ROW, TAG_JSON, TAG_FIB, TAG_DB};
pub use object::{TableObj as TableData, SetObj as SetData, FiberObj as FiberState, RowObj as RowRef, DatabaseObj as DatabaseData, VMColumn, SqlBinding, FiberStatus};
pub use stack::ValueStack;
pub use frame::{CallFrame, FrameStack, UpvalueCell};
#[cfg(feature = "jit")]
pub use trace::{Trace, TraceOp};

pub use core::vm::{VM, SharedContext, Chunk, OpResult};
pub use core::executor::Executor;
#[cfg(feature = "jit")]
pub use crate::jit::{JIT, JITFunction, MethodJitFunction};

