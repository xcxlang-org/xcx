use crate::frontend::ast::{Stmt, Type, Expr, Argument, DatabaseOpKind};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::intern::StringId;
use crate::error::span::Span;
use super::checker::{Checker, FunctionSignature};

impl<'a> Checker<'a> {
    pub(crate) fn check_fiber_def(
        &mut self,
        name: &StringId,
        params: &mut Vec<(Type, StringId)>,
        return_type: &mut Option<Box<Type>>,
        body: &mut Vec<Stmt>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        _span: &Span,
    ) {
        let name_str = self.interner.lookup(*name).trim().to_string();
        let var_type = Type::Fiber(return_type.clone());
        if !symbols.has(&name_str) {
            symbols.define(name_str.clone(), var_type.clone(), false);
        }

        let prev_fiber_ctx = self.fiber_context.take();
        let prev_has_yield = self.fiber_has_yield;
        let prev_is_fib = self.is_fiber_context;

        self.fiber_has_yield = false;
        self.fiber_context = Some(return_type.as_ref().map(|t| *t.clone()));
        self.is_fiber_context = true;

        let mut child = SymbolTable::new_with_parent(symbols);
        child.enter_scope();
        child.define(name_str, var_type, false);

        for (ty, pname) in params {
            let pname_str = self.interner.lookup(*pname).trim().to_string();
            child.define(pname_str, ty.clone(), false);
        }
        let prev_loop = self.loop_depth;
        self.loop_depth = 0;
        for s in &mut *body {
            self.check_stmt(s, &mut child, errors);
        }
        self.loop_depth = prev_loop;

        if return_type.is_some() {
            let has_ret = match body.last() {
                Some(stmt) => matches!(stmt.kind, crate::frontend::ast::StmtKind::Return(_)),
                None => false,
            };
            if !has_ret {
                errors.push(TypeError {
                    kind: TypeErrorKind::ReturnTypeMismatchInFiber,
                    span: _span.clone(),
                });
            }
        }

        self.fiber_context = prev_fiber_ctx;
        self.fiber_has_yield = prev_has_yield;
        self.is_fiber_context = prev_is_fib;
    }

    pub(crate) fn check_fiber_decl(
        &mut self,
        inner_type: &mut Option<Box<Type>>,
        name: &StringId,
        fiber_name: &StringId,
        args: &mut Vec<Argument>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let fiber_name_str = self.interner.lookup(*fiber_name).trim().to_string();

        self.check_argument_list(args, symbols, errors, false);

        let resolved_sig: Option<FunctionSignature> = if let Some(sig) = self.functions.get(&fiber_name_str).cloned() {
            Some(sig)
        } else if let Some(ty) = symbols.lookup(&fiber_name_str) {
            match ty {
                Type::Fiber(inner_ret) => {
                    Some(FunctionSignature {
                        params: vec![Type::Unknown; args.len()],
                        return_type: inner_ret.map(|t| *t),
                        is_fiber: true,
                    })
                }
                _ => None,
            }
        } else {
            None
        };

        if let Some(sig) = resolved_sig {
            if !sig.is_fiber {
                errors.push(TypeError {
                    kind: TypeErrorKind::UndefinedVariable(format!("{} is a func, not a fiber", fiber_name_str)),
                    span: span.clone(),
                });
            }
        } else {
            errors.push(TypeError {
                kind: TypeErrorKind::UndefinedVariable(format!("fiber '{}' not defined", fiber_name_str)),
                span: span.clone(),
            });
        }

        let name_str = self.interner.lookup(*name).trim().to_string();
        if symbols.has_in_current_scope(&name_str) {
            errors.push(TypeError { kind: TypeErrorKind::RedefinedVariable(name_str.clone()), span: span.clone() });
        }
        let var_type = Type::Fiber(inner_type.clone());
        symbols.define(name_str, var_type, false);
    }

    pub(crate) fn check_yield_stmt_with_target(
        &mut self,
        expr: &mut Expr,
        target: Option<&StringId>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        self.fiber_has_yield = true;
        let context = self.fiber_context.clone();
        
        // S208 is conditionally disabled later if the expression evaluated an I/O builtin
        let mut _yielded_ty = Type::Unknown;

        // Process None directly to allow evaluating expr if it's top-level DB operation
        if context.is_none() {
             let prev_yield = self.in_yield_expr;
             self.in_yield_expr = true;
             self.last_expr_was_db_io = false;
             _yielded_ty = self.check_expr(expr, symbols, errors);
             self.in_yield_expr = prev_yield;
             
             if let Type::DatabaseOperation(DatabaseOpKind::Remove, _) = _yielded_ty {
                  errors.push(TypeError { 
                      kind: TypeErrorKind::Other("Rule D401: remove() requires .where() filter before yielding".to_string()), 
                      span: expr.span.clone() 
                  });
             }
        }

        match context {
            None => {}
            Some(None) => {
                let prev_yield = self.in_yield_expr;
                self.in_yield_expr = true;
                self.last_expr_was_db_io = false;
                _yielded_ty = self.check_expr(expr, symbols, errors);
                self.in_yield_expr = prev_yield;
                
                if let Type::DatabaseOperation(DatabaseOpKind::Remove, _) = _yielded_ty {
                    errors.push(TypeError { 
                        kind: TypeErrorKind::Other("Rule D401: remove() requires .where() filter before yielding".to_string()), 
                        span: expr.span.clone() 
                    });
                }
            }
            Some(Some(expected_yield_ty)) => {
                let prev_yield = self.in_yield_expr;
                self.in_yield_expr = true;
                self.last_expr_was_db_io = false;
                _yielded_ty = self.check_expr(expr, symbols, errors);
                self.in_yield_expr = prev_yield;
                
                if let Type::DatabaseOperation(DatabaseOpKind::Remove, _) = _yielded_ty {
                    errors.push(TypeError { 
                        kind: TypeErrorKind::Other("Rule D401: remove() requires .where() filter before yielding".to_string()), 
                        span: expr.span.clone() 
                    });
                }
                if _yielded_ty != Type::Unknown && !self.is_compatible(&expected_yield_ty, &_yielded_ty) {
                    if !self.last_expr_was_db_io {
                        errors.push(TypeError {
                            kind: TypeErrorKind::TypeMismatch { expected: expected_yield_ty.clone(), actual: _yielded_ty.clone() },
                            span: expr.span.clone(),
                        });
                    }
                }
            }
        }

        if let Some(t_id) = target {
            let t_name = self.interner.lookup(*t_id).trim().to_string();
            symbols.define(t_name, _yielded_ty.clone(), false);
        }

        if !self.is_fiber_context && !self.last_expr_was_db_io {
            errors.push(TypeError { kind: TypeErrorKind::YieldOutsideFiber, span: span.clone() });
        }
    }

    pub(crate) fn check_yield_expr(
        &mut self,
        expr: &Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        self.fiber_has_yield = true;
        let prev_yield = self.in_yield_expr;
        self.in_yield_expr = true;
        self.last_expr_was_db_io = false;
        let ty = self.check_expr(expr, symbols, errors);
        self.in_yield_expr = prev_yield;
        
        if !self.is_fiber_context && !self.last_expr_was_db_io {
            errors.push(TypeError { kind: TypeErrorKind::YieldOutsideFiber, span: span.clone() });
        }
        ty
    }

    pub(crate) fn check_yield_from(
        &mut self,
        expr: &mut Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        self.fiber_has_yield = true;
        if !self.is_fiber_context {
            errors.push(TypeError { kind: TypeErrorKind::YieldOutsideFiber, span: span.clone() });
        }
        let expr_ty = self.check_expr(expr, symbols, errors);
        match &expr_ty {
            Type::Fiber(_) | Type::Unknown => {}
            _ => {
                errors.push(TypeError {
                    kind: TypeErrorKind::Other("'yield from' expects a fiber expression".to_string()),
                    span: expr.span.clone(),
                });
            }
        }
    }

    pub(crate) fn check_yield_void(&mut self, errors: &mut Vec<TypeError>, span: &Span) {
        if !self.is_fiber_context {
            errors.push(TypeError { kind: TypeErrorKind::YieldOutsideFiber, span: span.clone() });
        }
    }
    pub(crate) fn check_return(
        &mut self,
        expr: &mut Option<Box<Expr>>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let context = self.fiber_context.clone();
        match context {
            Some(Some(expected)) => {
                if let Some(e) = expr {
                    let actual = self.check_expr(e, symbols, errors);
                    if let Type::DatabaseOperation(crate::frontend::ast::DatabaseOpKind::Remove, _) = actual {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::Other("Rule D401: remove() requires .where() filter before returning".to_string()), 
                            span: e.span.clone() 
                        });
                    }
                    if actual != Type::Unknown && !self.is_compatible(&expected, &actual) {
                        errors.push(TypeError {
                            kind: TypeErrorKind::TypeMismatch { expected, actual },
                            span: e.span.clone(),
                        });
                    }
                } else {
                    errors.push(TypeError {
                        kind: TypeErrorKind::ReturnTypeMismatchInFiber,
                        span: span.clone(),
                    });
                }
            }
            Some(None) => {
                if let Some(e) = expr {
                    errors.push(TypeError {
                        kind: TypeErrorKind::Other("Void fiber cannot return a value".to_string()),
                        span: e.span.clone(),
                    });
                    let _ = self.check_expr(e, symbols, errors);
                }
            }
            None => {
                if let Some(e) = expr {
                    self.check_expr(e, symbols, errors);
                }
            }
        }
    }
}
