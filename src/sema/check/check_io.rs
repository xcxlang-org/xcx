use crate::frontend::ast::{Expr, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::error::span::Span;
use crate::intern::StringId;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_input(
        &mut self,
        name: StringId,
        ty: &mut Box<Type>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let name_str = self.interner.lookup(name).trim().to_string();
        if let Some(resolved_ty) = symbols.lookup(&name_str) {
            **ty = resolved_ty;
        } else {
            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(name_str), span: span.clone() });
        }
    }

    pub(crate) fn check_io_expr(
        &mut self,
        expr: &mut Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        self.check_expr(expr, symbols, errors);
    }

    pub(crate) fn check_halt(
        &mut self,
        message: &mut Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        self.check_expr(message, symbols, errors);
    }

    pub(crate) fn check_wait(
        &mut self,
        expr: &mut Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        self.check_expr(expr, symbols, errors);
    }
}
