use std::sync::atomic::{AtomicPtr};
use crate::vm::trace::TraceOp;

#[derive(Debug)]
pub struct Trace {
    pub ops: Vec<TraceOp>,
    pub start_ip: usize,
    pub bytecode_ptr: usize,
    pub revision: usize,
    pub native_ptr: AtomicPtr<u8>,
    pub min_locals: usize,
}
