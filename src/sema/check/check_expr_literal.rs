use crate::frontend::ast::{Expr, Type, SetType};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_array_literal(
        &mut self,
        elements: &Vec<Expr>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        context: Option<Type>,
    ) -> Type {
        let mut expected_inner = match context {
            Some(Type::Array(inner)) => Some(*inner.clone()),
            _ => None,
        };

        if elements.is_empty() {
            return Type::Array(Box::new(expected_inner.unwrap_or(Type::Int)));
        }

        if expected_inner.is_none() {
            let first_ty = self.check_expr(&elements[0], symbols, errors);
            expected_inner = Some(first_ty);
        } else {
            let first_ty = self.check_expr_with_context(&elements[0], symbols, errors, expected_inner.clone());
            if first_ty != Type::Unknown && !self.is_compatible(expected_inner.as_ref().unwrap(), &first_ty) {
                 errors.push(TypeError { 
                     kind: TypeErrorKind::TypeMismatch { expected: expected_inner.clone().unwrap(), actual: first_ty }, 
                     span: elements[0].span.clone() 
                 });
            }
        }

        let target = expected_inner.unwrap();
        for elem in elements.iter().skip(1) {
            let ty = self.check_expr_with_context(elem, symbols, errors, Some(target.clone()));
            if target != Type::Unknown && ty != Type::Unknown && !self.is_compatible(&target, &ty) {
                errors.push(TypeError { 
                    kind: TypeErrorKind::TypeMismatch { expected: target.clone(), actual: ty }, 
                    span: elem.span.clone() 
                });
            }
        }
        Type::Array(Box::new(target))
    }

    pub(crate) fn check_set_literal(
        &mut self,
        set_type: &SetType,
        elements: &Vec<Expr>,
        range: &Option<crate::frontend::ast::SetRange>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) -> Type {
        let expected = match set_type {
            SetType::N | SetType::Z => Type::Int,
            SetType::Q => Type::Float,
            SetType::S | SetType::C => Type::String,
            SetType::B => Type::Bool,
        };
        for elem in elements {
            let ty = self.check_expr(elem, symbols, errors);
            if ty != Type::Unknown && ty != expected {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: expected.clone(), actual: ty }, span: elem.span.clone() });
            }
        }
        if let Some(r) = range {
            let s_ty = self.check_expr(&r.start, symbols, errors);
            let e_ty = self.check_expr(&r.end, symbols, errors);
            if s_ty != Type::Unknown && s_ty != expected {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: expected.clone(), actual: s_ty }, span: r.start.span.clone() });
            }
            if e_ty != Type::Unknown && e_ty != expected {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: expected.clone(), actual: e_ty }, span: r.end.span.clone() });
            }
            if let Some(step_expr) = &r.step {
                let step_ty = self.check_expr(step_expr, symbols, errors);
                if step_ty != Type::Unknown {
                    let step_ok = if set_type == &SetType::C {
                        step_ty == Type::Int
                    } else if set_type == &SetType::Q {
                        step_ty == Type::Float || step_ty == Type::Int
                    } else {
                        step_ty == expected
                    };
                    if !step_ok {
                        errors.push(TypeError {
                            kind: TypeErrorKind::TypeMismatch { expected: if set_type == &SetType::C { Type::Int } else { expected.clone() }, actual: step_ty },
                            span: step_expr.span.clone()
                        });
                    }
                }
            }
        }
        Type::Set(set_type.clone())
    }

    pub(crate) fn check_array_or_set_literal(
        &mut self,
        elements: &Vec<Expr>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        context: Option<Type>,
    ) -> Type {
        match context {
            Some(Type::Array(inner)) => {
                for e in elements {
                    self.check_expr_with_context(e, symbols, errors, Some(*inner.clone()));
                }
                Type::Array(inner)
            }
            Some(Type::Set(st)) => {
                let inner = match &st {
                    SetType::N | SetType::Z => Type::Int,
                    SetType::Q => Type::Float,
                    SetType::S | SetType::C => Type::String,
                    SetType::B => Type::Bool,
                };
                for e in elements {
                    self.check_expr_with_context(e, symbols, errors, Some(inner.clone()));
                }
                Type::Set(st)
            }
            _ => {
                if elements.is_empty() {
                    return Type::Array(Box::new(Type::Int));
                }
                let first_ty = self.check_expr_with_context(&elements[0], symbols, errors, context.clone());
                for e in elements.iter().skip(1) {
                    let ty = self.check_expr_with_context(e, symbols, errors, context.clone());
                    if first_ty != Type::Unknown && ty != Type::Unknown && !self.is_compatible(&first_ty, &ty) {
                        let is_db_param = match &context {
                            Some(Type::Array(_)) => false,
                            _ => self.last_expr_was_db_io
                        };
                        
                        if !is_db_param {
                            errors.push(TypeError { 
                                kind: TypeErrorKind::TypeMismatch { expected: first_ty.clone(), actual: ty }, 
                                span: e.span.clone() 
                            });
                        }
                    }
                }
                Type::Array(Box::new(first_ty))
            }
        }
    }

    pub(crate) fn check_map_literal(
        &mut self,
        key_type: &Box<Type>,
        value_type: &Box<Type>,
        elements: &Vec<(Expr, Expr)>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) -> Type {
        for (k, v) in elements {
            let k_ty = self.check_expr(k, symbols, errors);
            if k_ty != Type::Unknown && k_ty != **key_type {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: (**key_type).clone(), actual: k_ty }, span: k.span.clone() });
            }
            let v_ty = self.check_expr(v, symbols, errors);
            if v_ty != Type::Unknown && !self.is_compatible(value_type, &v_ty) {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: (**value_type).clone(), actual: v_ty }, span: v.span.clone() });
            }
        }
        Type::Map(key_type.clone(), value_type.clone())
    }

    pub(crate) fn check_table_literal(
        &mut self,
        columns: &Vec<crate::frontend::ast::ColumnDef>,
        rows: &Vec<Vec<Expr>>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let non_auto_cols: Vec<_> = columns.iter().filter(|c| !c.is_auto()).collect();
        let non_auto_count = non_auto_cols.len();
        
        for row in rows {
            if row.len() != non_auto_count {
                errors.push(TypeError { 
                    kind: TypeErrorKind::TableRowCountMismatch { expected: non_auto_count, actual: row.len() },
                    span: span.clone() 
                });
            }
            for (i, val) in row.iter().enumerate() {
                let ty = self.check_expr(val, symbols, errors);
                if i < non_auto_count {
                    let expected = &non_auto_cols[i].ty;
                    if ty != Type::Unknown && !self.is_compatible(expected, &ty) {
                        errors.push(TypeError {
                            kind: TypeErrorKind::TypeMismatch { expected: expected.clone(), actual: ty },
                            span: val.span.clone()
                        });
                    }
                }
            }
        }
        Type::Table(columns.clone().into())
    }
}
