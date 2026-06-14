use crate::vm::opcode::Chunk;

pub struct CallFrame {
    pub chunk: Chunk,
    pub ip: usize,
    pub stack_base: usize,
}

impl CallFrame {
    pub fn new(chunk: Chunk, stack_base: usize) -> Self {
        Self {
            chunk,
            ip: 0,
            stack_base,
        }
    }
}
