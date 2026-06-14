use crate::frontend::ast::{Type, Expr, DatabaseOpKind};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::intern::StringId;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_assign(
        &mut self,
        name: &StringId,
        value: &mut Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let name_str = self.interner.lookup(*name).trim().to_string();
        let var_ty = match symbols.lookup(&name_str) {
            Some(ty) => ty,
            None => {
                errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(name_str.clone()), span: span.clone() });
                Type::Unknown
            }
        };
        if var_ty != Type::Unknown && symbols.is_const(&name_str) {
            errors.push(TypeError { kind: TypeErrorKind::ConstReassignment(name_str.clone()), span: span.clone() });
        }
        let val_ty = self.check_expr_with_context(value, symbols, errors, if var_ty != Type::Unknown { Some(var_ty.clone()) } else { None });
        if let Type::DatabaseOperation(crate::frontend::ast::DatabaseOpKind::Remove, _) = val_ty {
            errors.push(TypeError { 
                kind: TypeErrorKind::Other("Rule D401: remove() requires .where() filter. Cannot assign incomplete operation.".to_string()), 
                span: value.span.clone() 
            });
        }
        if var_ty != Type::Unknown && val_ty != Type::Unknown {
            if !self.is_compatible(&var_ty, &val_ty) {
                errors.push(TypeError {
                    kind: TypeErrorKind::TypeMismatch { expected: var_ty.clone(), actual: val_ty.clone() },
                    span: value.span.clone()
                });
            }
            if let (Type::Table(e_cols), Type::Table(a_cols)) = (&var_ty, &val_ty) {
                if e_cols.is_empty() && !a_cols.is_empty() {
                    symbols.define(name_str.clone(), val_ty.clone(), symbols.is_const(&name_str));
                }
            }
        }
    }

    pub(crate) fn check_expr_stmt(
        &mut self,
        expr: &mut Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        let ty = self.check_expr(expr, symbols, errors);
        if let Type::DatabaseOperation(DatabaseOpKind::Remove, _) = ty {
            errors.push(TypeError { 
                kind: TypeErrorKind::Other("Rule D401: remove() requires .where() filter".to_string()), 
                span: expr.span.clone() 
            });
        }
    }
}
