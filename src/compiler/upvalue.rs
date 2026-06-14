use crate::frontend::ast::{Expr, ExprKind};
use crate::intern::StringId;
use crate::compiler::compiler::FunctionCompiler;
use std::collections::HashMap;

impl FunctionCompiler {
    pub fn collect_captures(&self, expr: &Expr, parent_locals: &HashMap<StringId, usize>, out: &mut Vec<StringId>) {
        match &expr.kind {
            ExprKind::Identifier(id) => {
                if parent_locals.contains_key(id) && !out.contains(id) {
                    out.push(*id);
                }
            }
            ExprKind::Binary { left, right, .. } => {
                self.collect_captures(left, parent_locals, out);
                self.collect_captures(right, parent_locals, out);
            }
            ExprKind::Unary { right, .. } => {
                self.collect_captures(right, parent_locals, out);
            }
            ExprKind::FunctionCall { args, .. } => {
                for arg in args { self.collect_captures(arg.expr(), parent_locals, out); }
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                self.collect_captures(receiver, parent_locals, out);
                for arg in args { self.collect_captures(arg.expr(), parent_locals, out); }
            }
            ExprKind::ArrayLiteral { elements } => {
                for e in elements { self.collect_captures(e, parent_locals, out); }
            }
            ExprKind::MapLiteral { elements, .. } => {
                for (k, v) in elements {
                    self.collect_captures(k, parent_locals, out);
                    self.collect_captures(v, parent_locals, out);
                }
            }
            ExprKind::SetLiteral { elements, range, .. } => {
                for e in elements { self.collect_captures(e, parent_locals, out); }
                if let Some(r) = range {
                    self.collect_captures(&r.start, parent_locals, out);
                    self.collect_captures(&r.end, parent_locals, out);
                    if let Some(s) = &r.step { self.collect_captures(s, parent_locals, out); }
                }
            }
            ExprKind::Index { receiver, index } => {
                self.collect_captures(receiver, parent_locals, out);
                self.collect_captures(index, parent_locals, out);
            }
            ExprKind::MemberAccess { receiver, .. } => {
                self.collect_captures(receiver, parent_locals, out);
            }
            ExprKind::Lambda { body, .. } => {
                self.collect_captures(body, parent_locals, out);
            }
            _ => {}
        }
    }
}
