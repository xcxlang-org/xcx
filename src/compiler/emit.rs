use crate::vm::opcode::OpCode;
use crate::error::Span;
use crate::compiler::compiler::FunctionCompiler;

impl FunctionCompiler {
    pub fn emit(&mut self, op: OpCode, span: &Span) {
        self.bytecode.push(op);
        self.spans.push(span.clone());
    }
}
