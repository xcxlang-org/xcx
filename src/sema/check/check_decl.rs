use crate::frontend::ast::{Type, Expr, ExprKind};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::error::span::Span;
use crate::intern::StringId;
use super::checker::Checker;

impl<'a> Checker<'a> {

    pub(crate) fn check_var_decl(
        &mut self,
        is_const: bool,
        ty: &mut Box<Type>,
        name: &StringId,
        value: &mut Option<Box<Expr>>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let name_str_raw = self.interner.lookup(*name);
        let name_str = name_str_raw.trim().to_string();
        if symbols.has(&name_str) {
            errors.push(TypeError {
                kind: TypeErrorKind::RedefinedVariable(name_str.clone()),
                span: span.clone(),
            });
        }
        if let Some(val) = value {
            let val_ty = self.check_expr_with_context(val, symbols, errors, Some(*ty.clone()));
            if **ty != Type::Unknown && val_ty != Type::Unknown {
                if !self.is_compatible(ty, &val_ty) {
                    errors.push(TypeError {
                        kind: TypeErrorKind::TypeMismatch {
                            expected: (**ty).clone(),
                            actual: val_ty.clone(),
                        },
                        span: val.span.clone(),
                    });
                }
                if let (Type::Table(e_cols), Type::Table(a_cols)) = (&**ty, &val_ty) {
                    if e_cols.is_empty() && !a_cols.is_empty() {
                        **ty = val_ty.clone();
                    }
                }
            }
            if **ty == Type::Unknown {
                **ty = val_ty;
            }
        }
        let var_ty = *ty.clone();
        symbols.define(name_str, var_ty, is_const);
    }

    pub(crate) fn check_routes_expr(&mut self, expr: &Expr, symbols: &mut SymbolTable<'_>, errors: &mut Vec<TypeError>) {
        match &expr.kind {
            ExprKind::ArrayLiteral { elements } | ExprKind::Tuple(elements) => {
                for elem in elements {
                    self.check_routes_expr(elem, symbols, errors);
                }
            }
            ExprKind::Binary { right, .. } => {
                self.check_routes_expr(right, symbols, errors);
            }
            ExprKind::Identifier(id) => {
                let name = self.interner.lookup(*id).trim();
                if self.functions.get(name).is_none() && !symbols.has(name) {
                    if name != "*" && name != "_" {
                        errors.push(TypeError {
                            kind: TypeErrorKind::UndefinedVariable(name.to_string()),
                            span: expr.span.clone(),
                        });
                    }
                }
            }
            ExprKind::StringLiteral(_) => {}
            _ => {}
        }
    }
    pub(crate) fn check_database_decl(
        &mut self,
        name: &StringId,
        fields: &mut Vec<(StringId, Box<Expr>)>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        _span: &Span,
    ) {
        let name_str = self.interner.lookup(*name).trim().to_string();
        symbols.define(name_str, Type::Database, false);

        for (_, val) in fields {
            self.check_expr(val, symbols, errors);
        }
    }
}
