use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool};
use super::opcode::OpCode;
use crate::vm::value::Value;
use super::opcode::TypeTag;

#[derive(Clone)]
pub struct Chunk {
    pub bytecode: Arc<Vec<OpCode>>,
    pub spans: Arc<Vec<crate::error::Span>>,
    pub is_fiber: bool,
    pub max_locals: usize,
    pub call_count: Arc<AtomicUsize>,
    pub jit_ptr: Arc<AtomicPtr<std::ffi::c_void>>,
    pub name: String,
    pub arity: usize,
    pub uses_heap: Arc<AtomicBool>,
    pub used_locals: Arc<Vec<u8>>,
    /// Default value of the declared return type. Frame recovery after a
    /// halt.error writes it into the caller's destination register so a
    /// recovered call still yields a value of the declared type. `None` for
    /// chunks without a single declared return type (main, lambdas, fibers,
    /// void functions).
    pub ret_default: Option<Value>,
    /// Declared types of this chunk's parameters, indexed by parameter
    /// register (param i -> register i). Empty when the declared types are
    /// not statically known (main chunk, lambdas). The JIT's register type
    /// analysis is seeded with these: inferring param types from the
    /// bytecode alone is flow-insensitive and can mark a string parameter
    /// as Bool, which then corrupts spill-time tag reconstruction and
    /// refcount-elision decisions.
    pub param_types: Vec<TypeTag>,
}

impl Chunk {
    pub fn new(bytecode: Vec<OpCode>, spans: Vec<crate::error::Span>, is_fiber: bool, max_locals: usize, name: String, arity: usize) -> Self {
        let used_locals = crate::jit::analysis::analyze_chunk_locals(&bytecode);
        Self {
            bytecode: Arc::new(bytecode),
            spans: Arc::new(spans),
            is_fiber,
            max_locals,
            call_count: Arc::new(AtomicUsize::new(0)),
            jit_ptr: Arc::new(AtomicPtr::new(std::ptr::null_mut())),
            name,
            arity,
            uses_heap: Arc::new(AtomicBool::new(true)),
            used_locals: Arc::new(used_locals),
            ret_default: None,
            param_types: Vec::new(),
        }
    }
}
