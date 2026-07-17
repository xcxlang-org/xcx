use crate::frontend::ast::{Expr, ExprKind, Type};
use crate::frontend::lexer::TokenKind;
use crate::intern::StringId;
use crate::frontend::ast::argument::Argument;
use super::hir::{
    HirExpr, HirExprKind, HirLocal, HirBinOp, HirUnOp, HirArgument,
    HirRange, HirParam, HirLocalDef, LAMBDA_LOCAL_OFFSET,
};
use std::collections::HashMap;

fn lower_bin_op(op: TokenKind) -> HirBinOp {
    match op {
        TokenKind::Plus => HirBinOp::Add,
        TokenKind::Minus => HirBinOp::Sub,
        TokenKind::Star => HirBinOp::Mul,
        TokenKind::Slash => HirBinOp::Div,
        TokenKind::Percent => HirBinOp::Mod,
        TokenKind::Caret => HirBinOp::Pow,
        TokenKind::EqualEqual => HirBinOp::Equal,
        TokenKind::BangEqual => HirBinOp::NotEqual,
        TokenKind::Greater => HirBinOp::Greater,
        TokenKind::Less => HirBinOp::Less,
        TokenKind::GreaterEqual => HirBinOp::GreaterEqual,
        TokenKind::LessEqual => HirBinOp::LessEqual,
        TokenKind::And => HirBinOp::And,
        TokenKind::Or => HirBinOp::Or,
        TokenKind::Has => HirBinOp::Has,
        TokenKind::Union => HirBinOp::SetUnion,
        TokenKind::Intersection => HirBinOp::SetIntersection,
        TokenKind::Difference => HirBinOp::SetDifference,
        TokenKind::SymDifference => HirBinOp::SetSymDifference,
        TokenKind::PlusPlus => HirBinOp::IntConcat,
        TokenKind::DoubleColon => HirBinOp::MapConcat,
        _ => unreachable!("unhandled binary op: {:?}", op),
    }
}

fn lower_un_op(op: TokenKind) -> HirUnOp {
    match op {
        TokenKind::Not | TokenKind::Bang => HirUnOp::Not,
        TokenKind::Minus => HirUnOp::Neg,
        _ => unreachable!("unhandled unary op: {:?}", op),
    }
}

fn infer_literal_type(kind: &ExprKind) -> Type {
    match kind {
        ExprKind::IntLiteral(_) => Type::Int,
        ExprKind::FloatLiteral(_) => Type::Float,
        ExprKind::StringLiteral(_) => Type::String,
        ExprKind::BoolLiteral(_) => Type::Bool,
        _ => Type::Unknown,
    }
}

fn lower_arg(
    arg: &Argument,
    locals: &HashMap<StringId, HirLocal>,
    func_indices: &HashMap<StringId, usize>,
    globals: &HashMap<StringId, usize>,
) -> HirArgument {
    match arg {
        Argument::Positional(expr) => {
            HirArgument::Positional(lower_expr(expr, locals, func_indices, globals))
        }
        Argument::Named(name, expr) => {
            HirArgument::Named(*name, lower_expr(expr, locals, func_indices, globals))
        }
    }
}

fn lower_range(
    range: &crate::frontend::ast::expr::SetRange,
    locals: &HashMap<StringId, HirLocal>,
    func_indices: &HashMap<StringId, usize>,
    globals: &HashMap<StringId, usize>,
) -> HirRange {
    HirRange {
        start: Box::new(lower_expr(&range.start, locals, func_indices, globals)),
        end: Box::new(lower_expr(&range.end, locals, func_indices, globals)),
        step: range.step.as_ref().map(|s| Box::new(lower_expr(s, locals, func_indices, globals))),
    }
}

pub fn lower_expr(
    expr: &Expr,
    locals: &HashMap<StringId, HirLocal>,
    func_indices: &HashMap<StringId, usize>,
    globals: &HashMap<StringId, usize>,
) -> HirExpr {
    let ty = infer_literal_type(&expr.kind);
    let kind = match &expr.kind {
        ExprKind::IntLiteral(val) => HirExprKind::IntLiteral(*val),
        ExprKind::FloatLiteral(val) => HirExprKind::FloatLiteral(*val),
        ExprKind::StringLiteral(id) => HirExprKind::StringLiteral(*id),
        ExprKind::BoolLiteral(val) => HirExprKind::BoolLiteral(*val),
        ExprKind::Identifier(id) => {
            if let Some(&local_idx) = locals.get(id) {
                HirExprKind::Local(local_idx)
            } else {
                HirExprKind::Global(*id)
            }
        }
        ExprKind::RawBlock(id) => HirExprKind::RawBlock(*id),
        ExprKind::ArrayLiteral { elements } => HirExprKind::ArrayLiteral {
            elements: elements.iter().map(|e| lower_expr(e, locals, func_indices, globals)).collect(),
        },
        ExprKind::Binary { left, op, right } => HirExprKind::Binary {
            left: Box::new(lower_expr(left, locals, func_indices, globals)),
            op: lower_bin_op(op.clone()),
            right: Box::new(lower_expr(right, locals, func_indices, globals)),
        },
        ExprKind::Unary { op, right } => HirExprKind::Unary {
            op: lower_un_op(op.clone()),
            right: Box::new(lower_expr(right, locals, func_indices, globals)),
        },
        ExprKind::FunctionCall { name, args } => HirExprKind::FunctionCall {
            name: *name,
            args: args.iter().map(|a| lower_arg(a, locals, func_indices, globals)).collect(),
        },
        ExprKind::MethodCall { receiver, method, args, wait_after } => HirExprKind::MethodCall {
            receiver: Box::new(lower_expr(receiver, locals, func_indices, globals)),
            method: *method,
            args: args.iter().map(|a| lower_arg(a, locals, func_indices, globals)).collect(),
            wait_after: *wait_after,
        },
        ExprKind::SetLiteral { set_type, elements, range } => HirExprKind::SetLiteral {
            set_type: set_type.clone(),
            elements: elements.iter().map(|e| lower_expr(e, locals, func_indices, globals)).collect(),
            range: range.as_ref().map(|r| lower_range(r, locals, func_indices, globals)),
        },
        ExprKind::ArrayOrSetLiteral { elements } => HirExprKind::ArrayOrSetLiteral {
            elements: elements.iter().map(|e| lower_expr(e, locals, func_indices, globals)).collect(),
        },
        ExprKind::RandomChoice { set } => HirExprKind::RandomChoice {
            set: Box::new(lower_expr(set, locals, func_indices, globals)),
        },
        ExprKind::RandomInt { min, max, step } => HirExprKind::RandomInt {
            min: Box::new(lower_expr(min, locals, func_indices, globals)),
            max: Box::new(lower_expr(max, locals, func_indices, globals)),
            step: step.as_ref().map(|s| Box::new(lower_expr(s, locals, func_indices, globals))),
        },
        ExprKind::RandomFloat { min, max, step } => HirExprKind::RandomFloat {
            min: Box::new(lower_expr(min, locals, func_indices, globals)),
            max: Box::new(lower_expr(max, locals, func_indices, globals)),
            step: step.as_ref().map(|s| Box::new(lower_expr(s, locals, func_indices, globals))),
        },
        ExprKind::MapLiteral { key_type, value_type, elements } => HirExprKind::MapLiteral {
            key_type: key_type.clone(),
            value_type: value_type.clone(),
            elements: elements
                .iter()
                .map(|(k, v)| {
                    (
                        lower_expr(k, locals, func_indices, globals),
                        lower_expr(v, locals, func_indices, globals),
                    )
                })
                .collect(),
        },
        ExprKind::DateLiteral { date_string, format } => HirExprKind::DateLiteral {
            date_string: *date_string,
            format: *format,
        },
        ExprKind::TableLiteral { columns, rows } => HirExprKind::TableLiteral {
            columns: columns.clone(),
            rows: rows
                .iter()
                .map(|row| row.iter().map(|e| lower_expr(e, locals, func_indices, globals)).collect())
                .collect(),
        },
        ExprKind::DatabaseLiteral(elements) => HirExprKind::DatabaseLiteral(
            elements
                .iter()
                .map(|(k, e)| (*k, lower_expr(e, locals, func_indices, globals)))
                .collect(),
        ),
        ExprKind::Index { receiver, index } => HirExprKind::Index {
            receiver: Box::new(lower_expr(receiver, locals, func_indices, globals)),
            index: Box::new(lower_expr(index, locals, func_indices, globals)),
        },
        ExprKind::MemberAccess { receiver, member } => HirExprKind::MemberAccess {
            receiver: Box::new(lower_expr(receiver, locals, func_indices, globals)),
            member: *member,
        },
        ExprKind::TerminalCommand(cmd, args) => HirExprKind::TerminalCommand(
            *cmd,
            args.iter().map(|e| lower_expr(e, locals, func_indices, globals)).collect(),
        ),
        ExprKind::Lambda { params, return_type, body } => {
            let mut lambda_locals = std::collections::HashMap::new();
            for (&name, &slot) in locals {
                lambda_locals.insert(name, slot + LAMBDA_LOCAL_OFFSET);
            }
            let mut hir_params = Vec::new();
            let mut locals_defs = Vec::new();

            for (i, (ty, name)) in params.iter().enumerate() {
                let local_idx = i as u32;
                lambda_locals.insert(*name, local_idx);
                hir_params.push(HirParam {
                    ty: ty.clone(),
                    local: local_idx,
                    name: *name,
                });
                locals_defs.push(HirLocalDef {
                    name: *name,
                    ty: ty.clone(),
                    is_const: false,
                });
            }

            let hir_body = lower_expr(body, &lambda_locals, func_indices, globals);
            HirExprKind::Lambda {
                params: hir_params,
                return_type: return_type.clone(),
                body: Box::new(hir_body),
                locals: locals_defs,
            }
        }
        ExprKind::Tuple(elements) => HirExprKind::Tuple(
            elements.iter().map(|e| lower_expr(e, locals, func_indices, globals)).collect(),
        ),
        ExprKind::ModuleCall { module, method, args } => HirExprKind::ModuleCall {
            module: module.clone(),
            method: *method,
            args: args.iter().map(|a| lower_arg(a, locals, func_indices, globals)).collect(),
        },
        ExprKind::As { expr, name } => HirExprKind::As {
            expr: Box::new(lower_expr(expr, locals, func_indices, globals)),
            name: *name,
        },
        ExprKind::Yield(expr) => HirExprKind::Yield(Box::new(lower_expr(
            expr,
            locals,
            func_indices,
            globals,
        ))),
        ExprKind::Tag(id) => HirExprKind::Tag(*id),
    };

    HirExpr {
        kind,
        span: expr.span.clone(),
        ty,
    }
}
