use crate::frontend::ast::{Type, Argument, DatabaseOpKind};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_table_method(
        &mut self,
        cols: &crate::sema::types::TableType,

        method_str: &str,
        args: &Vec<Argument>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        match method_str {
            "where" | "on" => {
                if let Some(arg_node) = args.first() {
                    let arg = arg_node.expr();
                    let col_names: Vec<String> = cols.iter()
                        .map(|c| self.interner.lookup(c.name).trim().to_string())
                        .collect();
                    let prev_lambda = self.is_table_lambda;
                    self.is_table_lambda = true;
                    
                    symbols.enter_scope();
                    symbols.define("__row_tmp".to_string(), Type::Table(cols.clone()), false);

                    
                    let mut pred_idents = Vec::new();
                    self.collect_value_ref_idents(arg, &mut pred_idents);
                    for ident_name in &pred_idents {
                        if symbols.has(ident_name) && ident_name != "__row_tmp" && col_names.contains(ident_name) {
                            errors.push(TypeError {
                                kind: TypeErrorKind::WherePredicateNameCollision {
                                    var_name: ident_name.clone(),
                                    column_name: ident_name.clone(),
                                },
                                span: arg.span.clone(),
                            });
                        }
                    }

                    let pred_ty = self.check_expr(arg, symbols, errors);
                    if pred_ty != Type::Bool && pred_ty != Type::Unknown {
                        errors.push(TypeError {
                            kind: TypeErrorKind::TypeMismatch { expected: Type::Bool, actual: pred_ty },
                            span: arg.span.clone(),
                        });
                    }
                    
                    symbols.exit_scope();
                    self.is_table_lambda = prev_lambda;
                }
                Type::Table(cols.clone())
            }
            "join" | "show" | "delete" | "get" | "at" => {
                if method_str == "get" || method_str == "delete" || method_str == "at" {
                    if let Some(arg_node) = args.first() {
                        let arg = arg_node.expr();
                        let arg_ty = self.check_expr(arg, symbols, errors);
                        if arg_ty != Type::Int && arg_ty != Type::Unknown {
                            errors.push(TypeError {
                                kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: arg_ty },
                                span: arg.span.clone(),
                            });
                        }
                    } else {
                        errors.push(TypeError {
                            kind: TypeErrorKind::InvalidArgumentCount { expected: 1, actual: 0 },
                            span: span.clone(),
                        });
                    }
                } else {
                    for arg in args.iter() {
                        self.check_expr(arg.expr(), symbols, errors);
                    }
                }
                if method_str == "get" || method_str == "at" {
                    Type::Table(cols.clone())
                } else if method_str == "join" {
                    if let Some(other_arg) = args.get(0) {
                        let other_ty = self.check_expr(other_arg.expr(), symbols, errors);
                        
                        if args.len() == 3 {
                            let key1_ty = self.check_expr(args[1].expr(), symbols, errors);
                            let key2_ty = self.check_expr(args[2].expr(), symbols, errors);
                            if key1_ty != Type::String && key1_ty != Type::Unknown {
                                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::String, actual: key1_ty }, span: args[1].expr().span.clone() });
                            }
                            if key2_ty != Type::String && key2_ty != Type::Unknown {
                                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::String, actual: key2_ty }, span: args[2].expr().span.clone() });
                            }
                        } else if args.len() == 2 {
                            self.check_expr(args[1].expr(), symbols, errors);
                        }

                        match other_ty {
                            Type::Table(other_cols) => {
                                let mut combined = cols.clone();
                                for oc in other_cols {
                                    if let Some(existing) = combined.iter_mut().find(|c| c.name == oc.name) {
                                        *existing = oc;
                                    } else {
                                        combined.push(oc);
                                    }
                                }
                                Type::Table(combined)
                            }
                            Type::Unknown => Type::Table(cols.clone()),
                            _ => {
                                errors.push(TypeError {
                                    kind: TypeErrorKind::TypeMismatch { expected: Type::Table(crate::sema::types::TableType::empty()), actual: other_ty },
                                    span: other_arg.expr().span.clone(),
                                });

                                Type::Table(cols.clone())
                            }
                        }
                    } else {
                        errors.push(TypeError {
                            kind: TypeErrorKind::InvalidArgumentCount { expected: 1, actual: 0 },
                            span: span.clone(),
                        });
                        Type::Table(cols.clone())
                    }
                } else {
                    Type::Bool
                }
            }
            "add" | "insert" | "update" => {
                let start_idx = if method_str == "update" { 1 } else { 0 };

                if method_str == "update" {
                    if let Some(idx_arg) = args.first() {
                        let idx_ty = self.check_expr(idx_arg.expr(), symbols, errors);
                        if idx_ty != Type::Int && idx_ty != Type::Unknown {
                            errors.push(TypeError {
                                kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: idx_ty },
                                span: idx_arg.expr().span.clone(),
                            });
                        }
                    }
                }

                if !cols.is_empty() {
                    self.check_table_operation_args(cols, &args[start_idx..], symbols, errors, &span, method_str);
                } else {
                    for arg in args.iter().skip(start_idx) {
                        self.check_expr(arg.expr(), symbols, errors);
                    }
                }
                Type::Bool
            }
            "count" => Type::Int,
            "clear" => Type::Bool,
            "toJson" => Type::Json,
            "first" => Type::Json,
            _ => {
                errors.push(TypeError {
                    kind: TypeErrorKind::MethodNotFound { base_type: Type::Table(cols.clone()), method: method_str.to_string() },

                    span: span.clone()
                });
                Type::Unknown
            }
        }
    }

    pub(crate) fn check_database_method(
        &mut self,
        method_str: &str,
        args: &Vec<Argument>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let io_methods = ["fetch", "insert", "save", "push", "query", "queryRaw", "remove", "truncate", "exec", "sync", "drop"];
        if io_methods.contains(&method_str) {
            self.last_expr_was_db_io = true;
        }
        if self.fiber_context.is_some() && !self.in_yield_expr && io_methods.contains(&method_str) {
             errors.push(TypeError { 
                 kind: TypeErrorKind::Other(format!("Database I/O method '{}' must be yielded inside a fiber", method_str)), 
                 span: span.clone() 
             });
        }
        match method_str {
            "queryRaw" => {
                for arg in args.iter() { self.check_expr_with_context(arg.expr(), symbols, errors, Some(Type::Array(Box::new(Type::Unknown)))); }
                Type::Json
            }
            "fetch" | "sync" | "drop" | "has" | "truncate" | "remove" => {
                if let Some(arg) = args.first() {
                    let res_ty = self.check_expr(arg.expr(), symbols, errors);
                    if !matches!(res_ty, Type::Table(_)) && res_ty != Type::Unknown {
                        errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Table(crate::sema::types::TableType::empty()), actual: res_ty.clone() }, span: arg.expr().span.clone() });
                    }

                    if method_str == "has" { 
                        Type::Bool 
                    } else if method_str == "remove" {
                        let columns = if let Type::Table(cols) = res_ty { cols } else { crate::sema::types::TableType::empty() };
                        Type::DatabaseOperation(DatabaseOpKind::Remove, columns)
                    } else if method_str == "fetch" {
                        res_ty 
                    } else {
                        Type::Json
                    }
                } else {
                    errors.push(TypeError { kind: TypeErrorKind::InvalidArgumentCount { expected: 1, actual: 0 }, span: span.clone() });
                    Type::Unknown
                }
            }
            "insert" | "save" | "push" | "exec" => {
                if let Some(table_arg) = args.first() {
                    let ty = self.check_expr(table_arg.expr(), symbols, errors);
                    if method_str == "exec" {
                        if ty != Type::String && ty != Type::Unknown {
                            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::String, actual: ty }, span: table_arg.expr().span.clone() });
                        }
                        if args.len() > 1 {
                            self.check_expr_with_context(args[1].expr(), symbols, errors, Some(Type::Array(Box::new(Type::Unknown))));
                        }
                    } else {
                        if let Type::Table(ref cols) = ty {
                            if method_str == "push" || method_str == "save" {
                                if args.len() != 1 {
                                    errors.push(TypeError { kind: TypeErrorKind::InvalidArgumentCount { expected: 1, actual: args.len() }, span: span.clone() });
                                }
                            } else {
                                self.check_table_operation_args(cols, &args[1..], symbols, errors, &span, method_str);
                            }
                        } else if ty != Type::Unknown {
                            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Table(crate::sema::types::TableType::empty()), actual: ty }, span: table_arg.expr().span.clone() });
                        }

                    }
                } else {
                    errors.push(TypeError { kind: TypeErrorKind::InvalidArgumentCount { expected: 1, actual: 0 }, span: span.clone() });
                }
                Type::Json
            }
            "begin" | "commit" | "rollback" | "close" | "isOpen" => {
                for arg in args.iter() { self.check_expr(arg.expr(), symbols, errors); }
                Type::Bool
            }
            _ => {
                errors.push(TypeError { kind: TypeErrorKind::MethodNotFound { base_type: Type::Database, method: method_str.to_string() }, span: span.clone() });
                Type::Unknown
            }
        }
    }

    pub(crate) fn check_database_operation_method(
        &mut self,
        kind: DatabaseOpKind,
        cols: &crate::sema::types::TableType,

        method_str: &str,
        args: &Vec<Argument>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        match (kind, method_str) {
            (DatabaseOpKind::Remove, "where") => {
                if let Some(arg_node) = args.first() {
                     let arg = arg_node.expr();
                     {
                         let col_names: Vec<String> = cols.iter()
                             .map(|c| self.interner.lookup(c.name).trim().to_string())
                             .collect();
                         let prev_lambda = self.is_table_lambda;
                         self.is_table_lambda = true;
                         
                         symbols.enter_scope();
                         symbols.define("__row_tmp".to_string(), Type::Table(cols.clone()), false);

                         
                         let mut pred_idents = Vec::new();
                         self.collect_value_ref_idents(arg, &mut pred_idents);
                         for ident_name in &pred_idents {
                             if symbols.has(ident_name) && ident_name != "__row_tmp" && col_names.contains(ident_name) {
                                 errors.push(TypeError {
                                     kind: TypeErrorKind::WherePredicateNameCollision {
                                         var_name: ident_name.clone(),
                                         column_name: ident_name.clone(),
                                     },
                                     span: arg.span.clone(),
                                 });
                             }
                         }

                         let pred_ty = self.check_expr(arg, symbols, errors);
                         if pred_ty != Type::Bool && pred_ty != Type::Unknown {
                             errors.push(TypeError {
                                 kind: TypeErrorKind::TypeMismatch { expected: Type::Bool, actual: pred_ty },
                                 span: arg.span.clone(),
                             });
                         }
                         
                         symbols.exit_scope();
                         self.is_table_lambda = prev_lambda;
                     }
                 }
                Type::Json
            }
            _ => {
                errors.push(TypeError {
                    kind: TypeErrorKind::MethodNotFound { base_type: Type::DatabaseOperation(kind, cols.clone()), method: method_str.to_string() },
                    span: span.clone()
                });

                Type::Unknown
            }
        }
    }
}
