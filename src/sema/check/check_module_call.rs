use crate::frontend::ast::{Type, Argument};
use crate::frontend::lexer::TokenKind;
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::intern::StringId;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_module_call(
        &mut self,
        module: &TokenKind,
        method: StringId,
        args: &Vec<Argument>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let method_name = self.interner.lookup(method).trim().to_string();
        
        match module {
            TokenKind::Net => {
                match method_name.as_str() {
                    "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
                        if args.is_empty() {
                            errors.push(TypeError { kind: TypeErrorKind::Other(format!("net.{} requires at least a URL argument", method_name)), span: span.clone() });
                            return Type::Json;
                        }
                        let url_expr = args[0].expr();
                        let body_expr = args.get(1).map(|a| Box::new(a.expr().clone()));
                        self.check_net_call(url_expr, &body_expr, symbols, errors)
                    }
                    "respond" => {
                        if args.len() < 2 {
                            errors.push(TypeError { kind: TypeErrorKind::Other("net.respond requires at least status and body".to_string()), span: span.clone() });
                            return Type::Json;
                        }
                        let status_expr = args[0].expr();
                        let body_expr = args[1].expr();
                        let headers_expr = args.get(2).map(|a| Box::new(a.expr().clone()));
                        self.check_net_respond(status_expr, body_expr, &headers_expr, symbols, errors)
                    }
                    _ => {
                        errors.push(TypeError { kind: TypeErrorKind::Other(format!("Unknown net method: {}", method_name)), span: span.clone() });
                        Type::Unknown
                    }
                }
            }
            TokenKind::Json => {
                match method_name.as_str() {
                    "parse" => {
                        if args.len() != 1 {
                             errors.push(TypeError { kind: TypeErrorKind::Other("json.parse requires 1 argument (string)".to_string()), span: span.clone() });
                        } else {
                            let arg_ty = self.check_expr(args[0].expr(), symbols, errors);
                            if arg_ty != Type::String && arg_ty != Type::Unknown {
                                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::String, actual: arg_ty }, span: args[0].expr().span.clone() });
                            }
                        }
                        Type::Json
                    }
                    "toStr" | "stringify" => {
                        if args.len() != 1 {
                             errors.push(TypeError { kind: TypeErrorKind::Other("json.stringify requires 1 argument".to_string()), span: span.clone() });
                        } else {
                            self.check_expr(args[0].expr(), symbols, errors);
                        }
                        Type::String
                    }
                    _ => {
                        errors.push(TypeError { kind: TypeErrorKind::Other(format!("Unknown json method: {}", method_name)), span: span.clone() });
                        Type::Unknown
                    }
                }
            }
            TokenKind::Crypto => {
                 match method_name.as_str() {
                    "hash" => {
                        if args.len() < 1 {
                             errors.push(TypeError { kind: TypeErrorKind::Other("crypto.hash requires at least 1 argument".to_string()), span: span.clone() });
                        } else {
                            for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        }
                        Type::String
                    }
                    "token" => {
                        if !args.is_empty() {
                            self.check_expr(args[0].expr(), symbols, errors);
                        }
                        Type::String
                    }
                    "verify" => {
                        if args.len() != 3 {
                             errors.push(TypeError { kind: TypeErrorKind::Other("crypto.verify requires 3 arguments (password, hash, algorithm)".to_string()), span: span.clone() });
                        } else {
                            for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        }
                        Type::Bool
                    }
                    _ => {
                        errors.push(TypeError { kind: TypeErrorKind::Other(format!("Unknown crypto method: {}", method_name)), span: span.clone() });
                        Type::Unknown
                    }
                 }
            }
            TokenKind::Store => {
                match method_name.as_str() {
                    "read" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::String
                    }
                    "exists" | "write" | "append" | "delete" | "isDir" | "mkdir" | "zip" | "unzip" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::Bool
                    }
                    "size" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::Int
                    }
                    "list" | "glob" | "save" | "get" => {
                        for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                        Type::Json
                    }
                    _ => {
                        errors.push(TypeError { kind: TypeErrorKind::Other(format!("Unknown store method: {}", method_name)), span: span.clone() });
                        Type::Unknown
                    }
                }
            }
            TokenKind::Perf => {
                match method_name.as_str() {
                    "ms" | "us" | "ns" => {
                        if !args.is_empty() {
                            errors.push(TypeError { kind: TypeErrorKind::Other(format!("perf.{} does not accept arguments", method_name)), span: span.clone() });
                        }
                        Type::Int
                    }
                    _ => {
                        errors.push(TypeError { kind: TypeErrorKind::Other(format!("Unknown perf method: {}", method_name)), span: span.clone() });
                        Type::Unknown
                    }
                }
            }
            _ => {
                // Fallback for other modules
                for arg in args { self.check_expr(arg.expr(), symbols, errors); }
                Type::Unknown
            }
        }
    }
}
