use crate::vm::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiberStatus {
    Suspended,
    Running,
    Done,
    Error,
}

// Fiber object representation.
#[derive(Debug, Clone)]
pub struct FiberObj {
    pub func_id: usize,
    pub ip: usize,
    pub locals: Vec<Value>,
    pub status: FiberStatus,
    pub is_done: bool,
    pub yielded_value: Option<Value>,
    pub trace_revision: u64,
}

impl Drop for FiberObj {
    fn drop(&mut self) {
        for val in self.locals.iter() {
            unsafe { val.dec_ref(); }
        }
    }
}

impl PartialEq for FiberObj {
    fn eq(&self, other: &Self) -> bool { std::ptr::eq(self, other) }
}
impl Eq for FiberObj {}
