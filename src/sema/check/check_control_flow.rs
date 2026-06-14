use crate::frontend::ast::{Stmt, Type, ForIterType, Expr};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_if(
        &mut self,
        condition: &mut Box<Expr>,
        then_branch: &mut Vec<Stmt>,
        else_ifs: &mut Vec<(Box<Expr>, Vec<Stmt>)>,
        else_branch: &mut Option<Vec<Stmt>>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        let cond_ty = self.check_expr(condition, symbols, errors);
        if cond_ty != Type::Bool && cond_ty != Type::Unknown {
            errors.push(TypeError {
                kind: TypeErrorKind::TypeMismatch { expected: Type::Bool, actual: cond_ty },
                span: condition.span.clone()
            });
        }
        symbols.enter_scope();
        for stmt in then_branch {
            self.check_stmt(stmt, symbols, errors);
        }
        symbols.exit_scope();
        for (elif_cond, elif_branch) in else_ifs {
            let elif_ty = self.check_expr(elif_cond, symbols, errors);
            if elif_ty != Type::Bool && elif_ty != Type::Unknown {
                errors.push(TypeError {
                    kind: TypeErrorKind::TypeMismatch { expected: Type::Bool, actual: elif_ty },
                    span: elif_cond.span.clone()
                });
            }
            symbols.enter_scope();
            for stmt in elif_branch {
                self.check_stmt(stmt, symbols, errors);
            }
            symbols.exit_scope();
        }
        if let Some(branch) = else_branch {
            symbols.enter_scope();
            for stmt in branch {
                self.check_stmt(stmt, symbols, errors);
            }
            symbols.exit_scope();
        }
    }

    pub(crate) fn check_while(
        &mut self,
        condition: &mut Box<Expr>,
        body: &mut Vec<Stmt>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        let cond_ty = self.check_expr(condition, symbols, errors);
        if cond_ty != Type::Bool && cond_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Bool, actual: cond_ty }, span: condition.span.clone() });
        }
        self.loop_depth += 1;
        symbols.enter_scope();
        for s in body {
            self.check_stmt(s, symbols, errors);
        }
        symbols.exit_scope();
        self.loop_depth -= 1;
    }

    pub(crate) fn check_for(
        &mut self,
        var_name: &crate::intern::StringId,
        start: &mut Box<Expr>,
        end: &mut Box<Expr>,
        step: &mut Option<Box<Expr>>,
        body: &mut Vec<Stmt>,
        iter_type: &mut ForIterType,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let start_ty = self.check_expr(start, symbols, errors);

        if *iter_type != ForIterType::Range {
            let inner = match start_ty {
                Type::Array(inner) => {
                    *iter_type = ForIterType::Array;
                    *inner
                }
                Type::Set(st) => {
                    *iter_type = ForIterType::Set;
                    match st {
                        crate::frontend::ast::SetType::N | crate::frontend::ast::SetType::Z => Type::Int,
                        crate::frontend::ast::SetType::Q => Type::Float,
                        crate::frontend::ast::SetType::S | crate::frontend::ast::SetType::C => Type::String,
                        crate::frontend::ast::SetType::B => Type::Bool,
                    }
                }
                Type::Table(cols) => {
                    *iter_type = ForIterType::Array;
                    Type::Table(cols.clone())
                }
                Type::Fiber(inner) => {
                    *iter_type = ForIterType::Fiber;
                    if let Some(t) = inner {
                        *t.clone()
                    } else {
                        errors.push(TypeError {
                            kind: TypeErrorKind::CannotIterateOverVoidFiber,
                            span: span.clone(),
                        });
                        Type::Unknown
                    }
                }
                Type::Unknown => Type::Unknown,
                _ => {
                    errors.push(TypeError {
                        kind: TypeErrorKind::TypeMismatch {
                            expected: Type::Array(Box::new(Type::Int)),
                            actual: start_ty
                        },
                        span: start.span.clone()
                    });
                    Type::Unknown
                }
            };

            if let Some(step_expr) = step {
                let step_ty = self.check_expr(step_expr, symbols, errors);
                if step_ty != Type::Int && step_ty != Type::Unknown {
                    errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: step_ty }, span: step_expr.span.clone() });
                }
            }

            symbols.enter_scope();
            let name_str = self.interner.lookup(*var_name).trim().to_string();
            symbols.define(name_str, inner, false);
            self.loop_depth += 1;
            for s in body {
                self.check_stmt(s, symbols, errors);
            }
            self.loop_depth -= 1;
            symbols.exit_scope();
        } else {
            if start_ty != Type::Int && start_ty != Type::Unknown {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: start_ty }, span: start.span.clone() });
            }
            let e_ty = self.check_expr(end, symbols, errors);
            if e_ty != Type::Int && e_ty != Type::Unknown {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: e_ty }, span: end.span.clone() });
            }
            if let Some(step_expr) = step {
                let step_ty = self.check_expr(step_expr, symbols, errors);
                if step_ty != Type::Int && step_ty != Type::Unknown {
                    errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: step_ty }, span: step_expr.span.clone() });
                }
            }
            symbols.enter_scope();
            let name_str = self.interner.lookup(*var_name).trim().to_string();
            symbols.define(name_str, Type::Int, false);
            self.loop_depth += 1;
            for s in body {
                self.check_stmt(s, symbols, errors);
            }
            self.loop_depth -= 1;
            symbols.exit_scope();
        }
    }
}
