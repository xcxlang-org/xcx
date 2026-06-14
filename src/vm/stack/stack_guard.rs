// Stack guard implementation for XCX 4.0.
// Used to prevent stack overflow by capping call depth.
pub struct StackGuard {
    pub max_depth: usize,
    pub current_depth: usize,
}

impl StackGuard {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth, current_depth: 0 }
    }

    pub fn enter(&mut self) -> Result<(), String> {
        if self.current_depth >= self.max_depth {
            return Err("Stack overflow: maximum call depth exceeded".to_string());
        }
        self.current_depth += 1;
        Ok(())
    }

    pub fn exit(&mut self) {
        if self.current_depth > 0 {
            self.current_depth -= 1;
        }
    }
}
