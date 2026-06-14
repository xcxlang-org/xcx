use crate::frontend::ast::Expr;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};

pub mod leaf;
pub mod binary;
pub mod unary;
pub mod collection;
pub mod call;
pub mod control;
pub mod access;

impl FunctionCompiler {
    pub fn compile_expr(&mut self, expr: &Expr, ctx: &mut CompileContext) -> u8 {
        match &expr.kind {
            // Leaf / Literal expressions
            crate::frontend::ast::ExprKind::Tag(_) | 
            crate::frontend::ast::ExprKind::IntLiteral(_) | 
            crate::frontend::ast::ExprKind::FloatLiteral(_) | 
            crate::frontend::ast::ExprKind::StringLiteral(_) | 
            crate::frontend::ast::ExprKind::BoolLiteral(_) | 
            crate::frontend::ast::ExprKind::Identifier(_) |
            crate::frontend::ast::ExprKind::RawBlock(_) => {
                leaf::compile(self, expr, ctx)
            }

            // Binary & Unary
            crate::frontend::ast::ExprKind::Binary { .. } => binary::compile(self, expr, ctx),
            crate::frontend::ast::ExprKind::Unary { .. } => unary::compile(self, expr, ctx),

            // Collections & Generators
            crate::frontend::ast::ExprKind::ArrayLiteral { .. } |
            crate::frontend::ast::ExprKind::SetLiteral { .. } |
            crate::frontend::ast::ExprKind::MapLiteral { .. } |
            crate::frontend::ast::ExprKind::ArrayOrSetLiteral { .. } |
            crate::frontend::ast::ExprKind::TableLiteral { .. } |
            crate::frontend::ast::ExprKind::DatabaseLiteral(..) |
            crate::frontend::ast::ExprKind::RandomChoice { .. } |
            crate::frontend::ast::ExprKind::RandomInt { .. } |
            crate::frontend::ast::ExprKind::RandomFloat { .. } |
            crate::frontend::ast::ExprKind::DateLiteral { .. } => {
                collection::compile(self, expr, ctx)
            }

            // Calls
            crate::frontend::ast::ExprKind::FunctionCall { .. } |
            crate::frontend::ast::ExprKind::MethodCall { .. } |
            crate::frontend::ast::ExprKind::ModuleCall { .. } |
            crate::frontend::ast::ExprKind::TerminalCommand(..) => {
                call::compile(self, expr, ctx)
            }

            // Control & Scope
            crate::frontend::ast::ExprKind::Lambda { .. } |
            crate::frontend::ast::ExprKind::Yield(_) => {
                control::compile(self, expr, ctx)
            }

            // Access & Transform
            crate::frontend::ast::ExprKind::MemberAccess { .. } |
            crate::frontend::ast::ExprKind::Index { .. } |
            crate::frontend::ast::ExprKind::As { .. } |
            crate::frontend::ast::ExprKind::Tuple(_) => {
                access::compile(self, expr, ctx)
            }
        }
    }
}
