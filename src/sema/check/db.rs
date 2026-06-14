use crate::frontend::ast::{Expr, ExprKind};
use super::checker::Checker;

impl<'a> Checker<'a> {

    pub(crate) fn collect_value_ref_idents(&self, expr: &Expr, out: &mut Vec<String>) {
        use crate::frontend::lexer::TokenKind;
        match &expr.kind {
            ExprKind::Identifier(id) => {
                out.push(self.interner.lookup(*id).trim().to_string());
            }
            ExprKind::Binary { left, op, right } => {
                let is_comparison = matches!(
                    op,
                    TokenKind::EqualEqual | TokenKind::BangEqual
                    | TokenKind::Less     | TokenKind::LessEqual
                    | TokenKind::Greater  | TokenKind::GreaterEqual
                );
                if is_comparison {
                    self.collect_value_ref_idents(right, out); // tylko prawa strona
                } else {
                    self.collect_value_ref_idents(left, out);
                    self.collect_value_ref_idents(right, out);
                }
            }
            ExprKind::Unary { right, .. } => {
                self.collect_value_ref_idents(right, out);
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                self.collect_value_ref_idents(receiver, out);
                for a in args { self.collect_value_ref_idents(a.expr(), out); }
            }
            ExprKind::FunctionCall { args, .. } => {
                for a in args { self.collect_value_ref_idents(a.expr(), out); }
            }
            ExprKind::Lambda { body, .. } => {
                self.collect_value_ref_idents(body, out);
            }
            _ => {}
        }
    }
}
