use crate::frontend::ast::{Expr, Type, SetType};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_random_choice(
        &mut self,
        set: &Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) -> Type {
        let s_ty = self.check_expr(set, symbols, errors);
        match s_ty {
            Type::Set(_) => Type::Unknown,
            Type::Array(_) => Type::Unknown,
            Type::Unknown => Type::Unknown,
            _ => {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Set(SetType::N), actual: s_ty }, span: set.span.clone() });
                Type::Unknown
            }
        }
    }

    pub(crate) fn check_random_int(
        &mut self,
        min: &Expr,
        max: &Expr,
        step: &Option<Box<Expr>>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) -> Type {
        let min_ty = self.check_expr(min, symbols, errors);
        let max_ty = self.check_expr(max, symbols, errors);
        if min_ty != Type::Int && min_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: min_ty }, span: min.span.clone() });
        }
        if max_ty != Type::Int && max_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: max_ty }, span: max.span.clone() });
        }
        if let Some(s) = step {
            let s_ty = self.check_expr(s, symbols, errors);
            if s_ty != Type::Int && s_ty != Type::Unknown {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: s_ty }, span: s.span.clone() });
            }
        }
        Type::Int
    }

    pub(crate) fn check_random_float(
        &mut self,
        min: &Expr,
        max: &Expr,
        step: &Option<Box<Expr>>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) -> Type {
        let min_ty = self.check_expr(min, symbols, errors);
        let max_ty = self.check_expr(max, symbols, errors);
        if min_ty != Type::Float && min_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Float, actual: min_ty }, span: min.span.clone() });
        }
        if max_ty != Type::Float && max_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Float, actual: max_ty }, span: max.span.clone() });
        }
        if let Some(s) = step {
            let s_ty = self.check_expr(s, symbols, errors);
            if s_ty != Type::Float && s_ty != Type::Unknown && s_ty != Type::Int {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Float, actual: s_ty }, span: s.span.clone() });
            }
        }
        Type::Float
    }
}
