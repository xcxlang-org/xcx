use std::sync::Arc;
use crate::vm::frame::upvalue_cell::UpvalueCell;
use super::function_obj::FunctionObj;

// Closure object representation.
pub struct ClosureObj {
    pub function: Arc<FunctionObj>,
    pub upvalues: Vec<Arc<UpvalueCell>>,
}

impl ClosureObj {
    pub fn new(function: Arc<FunctionObj>, upvalues: Vec<Arc<UpvalueCell>>) -> Self {
        Self { function, upvalues }
    }
}
