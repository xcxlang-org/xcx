use cranelift::prelude::*;

pub struct LoopContext {
    pub header: Option<Block>,
    pub entry_sealed: bool,
}

impl LoopContext {
    pub fn new(header: Option<Block>) -> Self {
        Self {
            header,
            entry_sealed: false,
        }
    }

    pub fn is_active(&self) -> bool {
        self.header.is_some()
    }
}
