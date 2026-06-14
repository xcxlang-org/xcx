use crate::vm::opcode::OpCode;
use crate::compiler::compiler::FunctionCompiler;

impl FunctionCompiler {
    pub fn patch_jump(&mut self, offset: usize) {
        let target = self.bytecode.len() as u32;
        match &mut self.bytecode[offset] {
            OpCode::Jump { target: t } => *t = target,
            OpCode::JumpIfFalse { target: t, .. } => *t = target,
            OpCode::JumpIfTrue { target: t, .. } => *t = target,
            _ => {}
        }
    }

    pub fn patch_jump_to(&mut self, offset: usize, target: u32) {
        match &mut self.bytecode[offset] {
            OpCode::Jump { target: t } => *t = target,
            OpCode::JumpIfFalse { target: t, .. } => *t = target,
            OpCode::JumpIfTrue { target: t, .. } => *t = target,
            _ => {}
        }
    }
}
