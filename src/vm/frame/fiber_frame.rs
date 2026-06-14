use super::frame_stack::FrameStack;
use crate::vm::stack::ValueStack;
use crate::vm::object::FiberStatus;
use crate::vm::value::Value;

pub struct FiberFrame {
    pub frame_stack: FrameStack,
    pub value_stack: ValueStack,
    pub status: FiberStatus,
    pub yielded_value: Option<Value>,
}

impl FiberFrame {
    pub fn new() -> Self {
        Self {
            frame_stack: FrameStack::new(),
            value_stack: ValueStack::new(),
            status: FiberStatus::Suspended,
            yielded_value: None,
        }
    }
}
