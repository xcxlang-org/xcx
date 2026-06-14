use crate::frontend::ast::{Type, Argument, ExprKind};
use crate::error::Span;
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use super::checker::Checker;
use std::borrow::Cow;

impl<'a> Checker<'a> {
    pub(crate) fn check_table_operation_args(
        &mut self,
        cols: &crate::sema::types::TableType,
        args: &[Argument],
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
        _method: &str,
    ) {
        if cols.is_empty() {
            for arg in args {
                self.check_expr(arg.expr(), symbols, errors);
            }
            return;
        }
        let non_auto_cols: Vec<_> = cols.iter().filter(|c| !c.is_auto).collect();
        let mut positional_count = 0;
        let mut named_cols = std::collections::HashSet::new();
        let mut seen_named = false;

        let mut processed_args = Cow::Borrowed(args);
        if args.len() == 1 {
            if let Argument::Positional(expr) = &args[0] {
                match &expr.kind {
                    ExprKind::ArrayLiteral { elements } | ExprKind::Tuple(elements) => {
                        if elements.len() == non_auto_cols.len() {
                            let mut unrolled = Vec::with_capacity(elements.len());
                            for e in elements {
                                unrolled.push(Argument::Positional(e.clone()));
                            }
                            processed_args = Cow::Owned(unrolled);
                        }
                    }
                    _ => {}
                }
            }
        }

        for arg in processed_args.as_ref() {
            match arg {
                Argument::Positional(expr) => {
                    if seen_named {
                        errors.push(TypeError {
                            kind: TypeErrorKind::Other("Positional arguments must come before named arguments".to_string()),
                            span: expr.span.clone(),
                        });
                    }
                    if positional_count >= non_auto_cols.len() {
                        errors.push(TypeError { kind: TypeErrorKind::Other("Too many positional arguments for table operation".to_string()), span: expr.span.clone() });
                    } else {
                        let expected = &non_auto_cols[positional_count].ty;
                        let actual = self.check_expr(expr, symbols, errors);
                        if actual != Type::Unknown && !self.is_compatible(expected, &actual) {
                            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: expected.clone(), actual: actual.clone() }, span: expr.span.clone() });
                        }
                        positional_count += 1;
                    }
                }
                Argument::Named(name, expr) => {
                    seen_named = true;
                    let name_str = self.interner.lookup(*name).trim();
                    if let Some(col) = cols.iter().find(|c| self.interner.lookup(c.name).trim() == name_str) {
                        if col.is_auto {
                            errors.push(TypeError { kind: TypeErrorKind::Other(format!("Cannot provide value for @auto column '{}'", name_str)), span: expr.span.clone() });
                        }
                        if let Some(pos) = non_auto_cols.iter().position(|c| self.interner.lookup(c.name).trim() == name_str) {
                            if pos < positional_count {
                                errors.push(TypeError { kind: TypeErrorKind::Other(format!("Column '{}' already provided via positional argument", name_str)), span: expr.span.clone() });
                            }
                        }
                        if !named_cols.insert(name_str.to_string()) {
                            errors.push(TypeError { kind: TypeErrorKind::Other(format!("Duplicate named argument: {}", name_str)), span: expr.span.clone() });
                        }
                        let actual = self.check_expr(expr, symbols, errors);
                        if actual != Type::Unknown && !self.is_compatible(&col.ty, &actual) {
                            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: col.ty.clone(), actual: actual.clone() }, span: expr.span.clone() });
                        }
                    } else {
                        errors.push(TypeError { kind: TypeErrorKind::Other(format!("Column '{}' not found in table", name_str)), span: expr.span.clone() });
                    }
                }
            }
        }
        for col in &non_auto_cols {
            let col_name = self.interner.lookup(col.name).trim();
            let pos_idx = non_auto_cols.iter().position(|c| self.interner.lookup(c.name).trim() == col_name).unwrap();
            let covered_by_pos = pos_idx < positional_count;
            if !covered_by_pos && !named_cols.contains(col_name) {
                 let can_omit = col.is_optional || col.has_default;
                 if !can_omit {
                      errors.push(TypeError { kind: TypeErrorKind::Other(format!("Missing value for required column '{}'", col_name)), span: span.clone() });
                 }
            }
        }

    }
}
