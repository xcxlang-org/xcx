use crate::frontend::ast::{Expr, ExprKind, Type, SetType, Argument};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_method_call(
        &mut self,
        receiver: &Expr,
        method_str: &str,
        args: &Vec<Argument>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let rec_ty = self.check_expr(receiver, symbols, errors);
        match rec_ty {
            Type::Int | Type::Float => {
                match method_str {
                    "toStr" | "toString" | "format" => {
                        if !args.is_empty() {
                            errors.push(TypeError { kind: TypeErrorKind::InvalidArgumentCount { expected: 0, actual: args.len() }, span: span.clone() });
                        }
                        Type::String
                    }
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::MethodNotFound { base_type: rec_ty, method: method_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            Type::Table(cols) => {
                self.check_table_method(&cols, method_str, args, symbols, errors, span)
            }

            Type::Builtin(bid) => {
                let bname = self.interner.lookup(bid).trim();
                match bname {
                    "json" => match method_str {
                        "parse" | "stringify" => Type::Json,
                        _ => {
                            errors.push(TypeError { kind: TypeErrorKind::MethodNotFound { base_type: Type::Json, method: method_str.to_string() }, span: span.clone() });
                            Type::Unknown
                        }
                    },
                    "env" => match method_str {
                        "get" => Type::String,
                        "args" => Type::Array(Box::new(Type::String)),
                        _ => {
                            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(format!("method {} for env builtin", method_str)), span: span.clone() });
                            Type::Unknown
                        }
                    },
                    "crypto" => match method_str {
                        "hash" | "token" => Type::String,
                        "verify" => Type::Bool,
                        _ => {
                            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(format!("method {} for crypto builtin", method_str)), span: span.clone() });
                            Type::Unknown
                        }
                    },
                    "date" => match method_str {
                        "now" => Type::Date,
                        _ => {
                            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(format!("method {} for date builtin", method_str)), span: span.clone() });
                            Type::Unknown
                        }
                    },
                    "store" => match method_str {
                        "read" => Type::String,
                        "write" | "append" | "exists" | "delete" | "mkdir" | "zip" | "unzip" | "isDir" => Type::Bool,
                        "size" => Type::Int,
                        "list" | "glob" => Type::Array(Box::new(Type::String)),
                        _ => {
                            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(format!("method {} for store", method_str)), span: span.clone() });
                            Type::Unknown
                        }
                    },
                    "halt" | "terminal" => Type::Bool,
                    _ => {
                        errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(format!("builtin service {}", bname)), span: span.clone() });
                        Type::Unknown
                    }
                }
            }
            Type::Json => {
                self.check_argument_list(args, symbols, errors, false);
                match method_str {
                    "size" | "count" | "len" | "length" => Type::Int,
                    "keys" => Type::Array(Box::new(Type::String)),
                    "exists" | "ok" | "status" => Type::Bool,
                    "get" | "parse" | "append" | "push" | "set" | "update" | "delete" | "remove" | "body" | "bind" | "inject" | "first" | "insertId" | "affected" => Type::Json,
                    "toStr" | "toString" | "to_str" | "format" => Type::String,
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::MethodNotFound { base_type: Type::Json, method: method_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            Type::Database => {
                self.check_database_method(method_str, args, symbols, errors, span)
            }
            Type::DatabaseOperation(kind, cols) => {
                self.check_database_operation_method(kind, &cols, method_str, args, symbols, errors, span)
            }

            Type::Fiber(ref inner) => {
                match method_str {
                     "next" => {
                         if let Some(inner_ty) = inner {
                             for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                             (**inner_ty).clone()
                         } else {
                            errors.push(TypeError { kind: TypeErrorKind::Other("Cannot call .next() on a void fiber".to_string()), span: span.clone() });
                            Type::Unknown
                        }
                    }
                    "run" => {
                        if inner.is_none() { Type::Bool }
                        else {
                            errors.push(TypeError { kind: TypeErrorKind::CannotRunTypedFiber, span: span.clone() });
                            Type::Unknown
                        }
                    }
                    "isDone" | "close" => Type::Bool,
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::MethodNotFound { base_type: Type::Fiber(inner.clone()), method: method_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            Type::Set(ref st) => {
                let inner_ty = match st {
                    SetType::N | SetType::Z => Type::Int,
                    SetType::Q => Type::Float,
                    SetType::S | SetType::C => Type::String,
                    SetType::B => Type::Bool,
                };
                match method_str {
                    "size" | "count" | "length" => Type::Int,
                    "contains" | "add" | "remove" => {
                        if let Some(arg) = args.first() {
                            let arg_ty = self.check_expr(arg.expr(), symbols, errors);
                            if arg_ty != Type::Unknown && arg_ty != inner_ty {
                                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: inner_ty, actual: arg_ty }, span: arg.expr().span.clone() });
                            }
                        }
                        Type::Bool
                    }
                    "isEmpty" | "clear" | "show" => Type::Bool,
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::MethodNotFound { base_type: Type::Set(st.clone()), method: method_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            Type::Array(ref inner) => {
                match method_str {
                    "size" | "length" | "count" => Type::Int,
                    "isEmpty" => Type::Bool,
                    "get" | "delete" | "remove" | "contains" | "find" | "indexOf" | "pop" => {
                        if args.len() == 1 {
                            let arg_ty = self.check_expr(args[0].expr(), symbols, errors);
                            if (method_str == "get" || method_str == "delete" || method_str == "remove") && arg_ty != Type::Int && arg_ty != Type::Unknown {
                                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: arg_ty }, span: args[0].expr().span.clone() });
                            }
                        }
                        if method_str == "get" || method_str == "pop" { (**inner).clone() }
                        else if method_str == "find" || method_str == "indexOf" { Type::Int }
                        else { Type::Bool }
                    }
                    "push" | "insert" | "update" | "set" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::Bool
                    }
                    "toStr" | "toString" | "toJson" | "show" | "clear" | "sort" | "reverse" => Type::Bool,
                    "slice" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::Array(inner.clone())
                    }
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::MethodNotFound { base_type: Type::Array(inner.clone()), method: method_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            Type::Map(ref k, ref v) => {
                match method_str {
                    "size" | "count" | "isEmpty" | "clear" | "show" => {
                        if method_str == "size" || method_str == "count" { Type::Int } else { Type::Bool }
                    }
                    "get" | "contains" | "remove" | "delete" => {
                        if !args.is_empty() {
                            let key_ty = self.check_expr(args[0].expr(), symbols, errors);
                            if key_ty != Type::Unknown && !self.is_compatible(k, &key_ty) {
                                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: (**k).clone(), actual: key_ty }, span: args[0].expr().span.clone() });
                            }
                        }
                        if method_str == "get" { (**v).clone() } else { Type::Bool }
                    }
                    "insert" | "set" | "update" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::Bool
                    }
                    "keys" => Type::Array(k.clone()),
                    "values" => Type::Array(v.clone()),
                    "toStr" | "toString" | "to_str" => Type::String,
                    "toJson" => Type::Json,
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::MethodNotFound { base_type: Type::Map(k.clone(), v.clone()), method: method_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            Type::Date => {
                for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                match method_str {
                    "format" => Type::String,
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::MethodNotFound { base_type: Type::Date, method: method_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            Type::String => {
                if self.is_table_lambda {
                    errors.push(TypeError { kind: TypeErrorKind::Other("Rule DB-005b: String methods are not allowed inside .where() predicates".to_string()), span: span.clone() });
                }
                match method_str {
                    "size" | "length" | "indexOf" | "lastIndexOf" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::Int
                    }
                    "upper" | "lower" | "trim" => {
                        if !args.is_empty() {
                            errors.push(TypeError { kind: TypeErrorKind::InvalidArgumentCount { expected: 0, actual: args.len() }, span: span.clone() });
                        }
                        Type::String
                    }
                    "toInt" | "toFloat" => {
                        if !args.is_empty() {
                            errors.push(TypeError { kind: TypeErrorKind::InvalidArgumentCount { expected: 0, actual: args.len() }, span: span.clone() });
                        }
                        if method_str == "toInt" { Type::Int } else { Type::Float }
                    }
                    "startsWith" | "endsWith" | "char" | "charAt" | "replace" | "slice" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        if method_str == "startsWith" || method_str == "endsWith" { Type::Bool } else { Type::String }
                    }
                    "split" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::Array(Box::new(Type::String))
                    }
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::MethodNotFound { base_type: Type::String, method: method_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            _ => {
                if let ExprKind::Identifier(rec_id) = &receiver.kind {
                    let rec_str = self.interner.lookup(*rec_id).trim();
                    let namespaced_name = format!("{}.{}", rec_str, method_str);
                    if let Some(sig) = self.functions.get(&namespaced_name).cloned() {
                        for (i, arg) in args.iter().enumerate() {
                            let arg_ty = self.check_expr(arg.expr(), symbols, errors);
                            if i < sig.params.len() && !self.is_compatible(&sig.params[i], &arg_ty) {
                                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: sig.params[i].clone(), actual: arg_ty }, span: arg.expr().span.clone() });
                            }
                        }
                        return sig.return_type.unwrap_or(Type::Unknown);
                    }
                }
                if rec_ty != Type::Unknown {
                    errors.push(TypeError { 
                        kind: TypeErrorKind::MethodNotFound { base_type: rec_ty, method: method_str.to_string() }, 
                        span: span.clone() 
                    });
                }
                Type::Unknown
            }
        }
    }
}
