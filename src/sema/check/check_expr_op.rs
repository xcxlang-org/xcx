use crate::frontend::ast::{Expr, Type, SetType};
use crate::frontend::lexer::TokenKind;
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_binary_expr(
        &mut self,
        left: &Expr,
        op: &TokenKind,
        right: &Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let l_ty = self.check_expr(left, symbols, errors);
        let r_ty = self.check_expr(right, symbols, errors);
        if l_ty == Type::Unknown || r_ty == Type::Unknown { return Type::Unknown; }
        
        match op {
            TokenKind::Plus => {
                if l_ty == Type::Date && r_ty == Type::Int { return Type::Date; }
                if l_ty == Type::String || r_ty == Type::String {
                    Type::String
                } else if (l_ty == Type::Int || l_ty == Type::Float) && (r_ty == Type::Int || r_ty == Type::Float) {
                    if l_ty == Type::Float || r_ty == Type::Float { Type::Float } else { Type::Int }
                } else {
                    errors.push(TypeError { kind: TypeErrorKind::InvalidBinaryOp { op: op.clone(), left: l_ty, right: r_ty }, span: span.clone() });
                    Type::Unknown
                }
            }
            TokenKind::PlusPlus => {
                if (l_ty == Type::Int || l_ty == Type::Unknown) && (r_ty == Type::Int || r_ty == Type::Unknown) {
                    Type::Int
                } else {
                    Type::String
                }
            }
            TokenKind::Minus | TokenKind::Star | TokenKind::Slash | TokenKind::Percent | TokenKind::Caret => {
                if op == &TokenKind::Minus {
                    match (&l_ty, &r_ty) {
                        (Type::Date, Type::Int) => return Type::Date,
                        (Type::Date, Type::Date) => return Type::Int,
                        (Type::Int, Type::Date) => return Type::Int,
                        (Type::Set(_), Type::Set(_)) if l_ty == r_ty => return l_ty.clone(),
                        _ => {}
                    }
                }
                if (l_ty == Type::Int || l_ty == Type::Float) && (r_ty == Type::Int || r_ty == Type::Float) {
                    if l_ty == Type::Float || r_ty == Type::Float { Type::Float } else { Type::Int }
                } else if matches!(l_ty, Type::Set(_)) && l_ty == r_ty && op == &TokenKind::Minus {
                    l_ty.clone()
                } else {
                    errors.push(TypeError { kind: TypeErrorKind::InvalidBinaryOp { op: op.clone(), left: l_ty, right: r_ty }, span: span.clone() });
                    Type::Unknown
                }
            }
            TokenKind::EqualEqual | TokenKind::BangEqual | TokenKind::Greater | TokenKind::Less | TokenKind::GreaterEqual | TokenKind::LessEqual => {
                if l_ty == Type::Json || r_ty == Type::Json { return Type::Bool; }
                if !self.is_compatible(&l_ty, &r_ty) && !self.is_compatible(&r_ty, &l_ty) {
                    errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: l_ty, actual: r_ty }, span: span.clone() });
                }
                Type::Bool
            }
            TokenKind::And | TokenKind::Or => {
                if l_ty != Type::Bool { errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Bool, actual: l_ty }, span: left.span.clone() }); }
                if r_ty != Type::Bool { errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Bool, actual: r_ty }, span: right.span.clone() }); }
                Type::Bool
            }
            TokenKind::Has => {
                if l_ty == Type::String && r_ty == Type::String { return Type::Bool; }
                let inner_ty = match &r_ty {
                    Type::Array(inner) => Some((**inner).clone()),
                    Type::Set(st) => Some(match st {
                        SetType::N | SetType::Z => Type::Int,
                        SetType::Q => Type::Float,
                        SetType::S | SetType::C => Type::String,
                        SetType::B => Type::Bool,
                    }),
                    _ => None,
                };
                if let Some(expected) = inner_ty {
                    if !self.is_compatible(&expected, &l_ty) {
                        errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected, actual: l_ty }, span: left.span.clone() });
                    }
                } else {
                    errors.push(TypeError { kind: TypeErrorKind::InvalidBinaryOp { op: op.clone(), left: l_ty, right: r_ty.clone() }, span: span.clone() });
                }
                Type::Bool
            }
            TokenKind::Union | TokenKind::Intersection | TokenKind::Difference | TokenKind::SymDifference => {
                if matches!(l_ty, Type::Set(_)) && l_ty == r_ty {
                    l_ty.clone()
                } else {
                    errors.push(TypeError { kind: TypeErrorKind::InvalidBinaryOp { op: op.clone(), left: l_ty, right: r_ty }, span: span.clone() });
                    Type::Unknown
                }
            }
            TokenKind::DoubleColon => {
                Type::Map(Box::new(l_ty), Box::new(r_ty))
            }
            _ => {
                errors.push(TypeError { kind: TypeErrorKind::InvalidBinaryOp { op: op.clone(), left: l_ty, right: r_ty }, span: span.clone() });
                Type::Unknown
            }
        }
    }

    pub(crate) fn check_unary_expr(
        &mut self,
        op: &TokenKind,
        right: &Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let r_ty = self.check_expr(right, symbols, errors);
        if r_ty == Type::Unknown { return Type::Unknown; }
        match op {
            TokenKind::Minus => {
                if r_ty != Type::Int && r_ty != Type::Float {
                    errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: r_ty }, span: right.span.clone() });
                    Type::Unknown
                } else { r_ty }
            }
            TokenKind::Not | TokenKind::Bang => {
                if r_ty != Type::Bool {
                    errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Bool, actual: r_ty }, span: right.span.clone() });
                }
                Type::Bool
            }
            _ => {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: r_ty }, span: span.clone() });
                Type::Unknown
            }
        }
    }
}
