use crate::frontend::ast::{Expr, ExprKind};
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use crate::vm::opcode::OpCode;
use crate::frontend::lexer::TokenKind;

pub fn compile(fc: &mut FunctionCompiler, expr: &Expr, ctx: &mut CompileContext) -> u8 {
    if let ExprKind::Binary { left, op, right } = &expr.kind {
        let src1 = fc.compile_expr(left, ctx);
        let src2 = fc.compile_expr(right, ctx);
        let dst = src1;
        match op {
            TokenKind::Plus => fc.emit(OpCode::Add { dst, src1, src2 }, &expr.span),
            TokenKind::Minus => fc.emit(OpCode::Sub { dst, src1, src2 }, &expr.span),
            TokenKind::Star => fc.emit(OpCode::Mul { dst, src1, src2 }, &expr.span),
            TokenKind::Slash => fc.emit(OpCode::Div { dst, src1, src2 }, &expr.span),
            TokenKind::Percent => fc.emit(OpCode::Mod { dst, src1, src2 }, &expr.span),
            TokenKind::Caret => fc.emit(OpCode::Pow { dst, src1, src2 }, &expr.span),
            TokenKind::EqualEqual => fc.emit(OpCode::Equal { dst, src1, src2 }, &expr.span),
            TokenKind::BangEqual => fc.emit(OpCode::NotEqual { dst, src1, src2 }, &expr.span),
            TokenKind::Greater => fc.emit(OpCode::Greater { dst, src1, src2 }, &expr.span),
            TokenKind::Less => fc.emit(OpCode::Less { dst, src1, src2 }, &expr.span),
            TokenKind::GreaterEqual => fc.emit(OpCode::GreaterEqual { dst, src1, src2 }, &expr.span),
            TokenKind::LessEqual => fc.emit(OpCode::LessEqual { dst, src1, src2 }, &expr.span),
            TokenKind::And => fc.emit(OpCode::And { dst, src1, src2 }, &expr.span),
            TokenKind::Or => fc.emit(OpCode::Or { dst, src1, src2 }, &expr.span),
            TokenKind::Has => fc.emit(OpCode::Has { dst, src1, src2 }, &expr.span),
            TokenKind::Union => fc.emit(OpCode::SetUnion { dst, src1, src2 }, &expr.span),
            TokenKind::Intersection => fc.emit(OpCode::SetIntersection { dst, src1, src2 }, &expr.span),
            TokenKind::Difference => fc.emit(OpCode::SetDifference { dst, src1, src2 }, &expr.span),
            TokenKind::SymDifference => fc.emit(OpCode::SetSymDifference { dst, src1, src2 }, &expr.span),
            TokenKind::PlusPlus => fc.emit(OpCode::IntConcat { dst, src1, src2 }, &expr.span),
            TokenKind::DoubleColon => {
                let base = fc.next_local as u8;
                fc.next_local += 2;
                fc.sync_max_locals();
                fc.emit(OpCode::Move { dst: base + 1, src: src2 }, &expr.span);
                fc.emit(OpCode::Move { dst: base, src: src1 }, &expr.span);
                fc.emit(OpCode::MapInit { dst, base, count: 1 }, &expr.span);
            }
            _ => {}
        }
        fc.next_local = (dst + 1) as usize;
        fc.sync_max_locals();
        return dst;
    }
    0
}
