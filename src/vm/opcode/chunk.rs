use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool};
use parking_lot::RwLock;
use std::collections::HashMap;
use super::opcode::OpCode;

#[derive(Clone)]
pub struct Chunk {
    pub bytecode: Arc<Vec<OpCode>>,
    pub spans: Arc<Vec<crate::error::Span>>,
    pub is_fiber: bool,
    pub max_locals: usize,
    pub has_loops: bool,
    pub call_count: Arc<AtomicUsize>,
    pub jit_ptr: Arc<AtomicPtr<std::ffi::c_void>>,
    pub jit_segments: Arc<RwLock<HashMap<usize, usize>>>,
    pub name: String,
    pub arity: usize,
    pub uses_heap: Arc<AtomicBool>,
    pub used_locals: Arc<Vec<u8>>,
}

impl Chunk {
    pub fn new(bytecode: Vec<OpCode>, spans: Vec<crate::error::Span>, is_fiber: bool, max_locals: usize, has_loops: bool, name: String, arity: usize) -> Self {
        let used_locals = crate::jit::analysis::analyze_chunk_locals(&bytecode);
        Self {
            bytecode: Arc::new(bytecode),
            spans: Arc::new(spans),
            is_fiber,
            max_locals,
            has_loops,
            call_count: Arc::new(AtomicUsize::new(0)),
            jit_ptr: Arc::new(AtomicPtr::new(std::ptr::null_mut())),
            jit_segments: Arc::new(RwLock::new(HashMap::new())),
            name,
            arity,
            uses_heap: Arc::new(AtomicBool::new(true)),
            used_locals: Arc::new(used_locals),
        }
    }
}
