use crate::frontend::ast::{Expr, ExprKind};
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use crate::vm::opcode::OpCode;
use crate::frontend::lexer::TokenKind;

pub fn compile(fc: &mut FunctionCompiler, expr: &Expr, ctx: &mut CompileContext) -> u8 {
    if let ExprKind::Unary { op, right } = &expr.kind {
        match op {
            TokenKind::Not | TokenKind::Bang => {
                let src = fc.compile_expr(right, ctx);
                fc.next_local = src as usize;
                let dst = fc.push_reg();
                fc.emit(OpCode::Not { dst, src }, &expr.span);
                return dst;
            }
            TokenKind::Minus => {
                let src = fc.compile_expr(right, ctx);
                let dst = src;
                fc.emit(OpCode::Neg { dst, src }, &expr.span);
                return dst;
            }
            _ => return fc.push_reg(),
        }
    }
    0
}
