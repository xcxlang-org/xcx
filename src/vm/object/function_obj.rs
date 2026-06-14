use crate::intern::StringId;

// Function object representation.
pub struct FunctionObj {
    pub name: StringId,
    pub arity: u8,
    pub upvalue_count: u16,
    pub chunk_idx: u32,
}

impl FunctionObj {
    pub fn new(name: StringId, arity: u8, upvalue_count: u16, chunk_idx: u32) -> Self {
        Self { name, arity, upvalue_count, chunk_idx }
    }
}
