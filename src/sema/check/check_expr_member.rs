use crate::frontend::ast::{Expr, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::intern::StringId;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_member_access(
        &mut self,
        receiver: &Expr,
        member: StringId,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let rec_ty = self.check_expr(receiver, symbols, errors);
        let member_str = self.interner.lookup(member).trim();
        match rec_ty {
            Type::Table(cols) => {
                if let Some(col) = cols.iter().find(|c| self.interner.lookup(c.name) == member_str) {
                    col.ty.clone()
                } else {
                    match member_str {
                        "count" | "size" | "length" => Type::Int,
                        _ => {
                            errors.push(TypeError { 
                                kind: TypeErrorKind::PropertyNotFound { base_type: Type::Table(cols.clone()), property: member_str.to_string() }, 
                                span: span.clone() 
                            });
                            Type::Unknown
                        }
                    }
                }
            }
            Type::Date => {
                match member_str {
                    "year" | "month" | "day" | "hour" | "minute" | "second" | "ms" => Type::Int,
                    _ => {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::PropertyNotFound { base_type: Type::Date, property: member_str.to_string() }, 
                            span: span.clone() 
                        });
                        Type::Unknown
                    }
                }
            }
            Type::Builtin(bid) => {
                let bname = self.interner.lookup(bid).trim();
                if bname == "date" && member_str == "now" {
                    Type::Date
                } else if bname == "net" {
                    match member_str {
                        "request" | "query" | "headers" | "body" | "form" => Type::Json,
                        "method" | "url" | "path" => Type::String,
                        "ip" | "remote_addr" => Type::String,
                        _ => Type::Json,
                    }
                } else {
                    errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(format!("property {} for builtin service {}", member_str, bname)), span: span.clone() });
                    Type::Unknown
                }
            }
            Type::Database => {
                if let Some(Type::Table(cols)) = symbols.lookup(member_str) {
                    Type::Table(cols.clone())
                } else {
                    errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(format!("Table {} not found for database", member_str)), span: span.clone() });
                    Type::Unknown
                }
            }
            Type::Array(_) | Type::Map(_, _) | Type::Set(_) | Type::String | Type::Json => {
                if member_str == "length" || member_str == "size" || member_str == "count" {
                    Type::Int
                } else if matches!(rec_ty, Type::Json) {
                    match member_str {
                        "status" | "code" => Type::Int,
                        "ok" => Type::Bool,
                        "body" | "json" | "headers" => Type::Json,
                        "method" | "path" | "query" | "url" | "text" => Type::String,
                        _ => Type::Json,
                    }
                } else {
                    errors.push(TypeError { 
                        kind: TypeErrorKind::PropertyNotFound { base_type: rec_ty, property: member_str.to_string() }, 
                        span: span.clone() 
                    });
                    Type::Unknown
                }
            }
            Type::Unknown => Type::Unknown,
            _ => {
                 // Try looking up namespaced functions e.g. Math.sqrt
                 if let crate::frontend::ast::ExprKind::Identifier(rec_id) = &receiver.kind {
                     let rec_str = self.interner.lookup(*rec_id).trim();
                     let namespaced_name = format!("{}.{}", rec_str, member_str);
                     if let Some(ty) = symbols.lookup(&namespaced_name) {
                         return ty.clone();
                     }
                 }
                 errors.push(TypeError { 
                     kind: TypeErrorKind::PropertyNotFound { base_type: rec_ty, property: member_str.to_string() }, 
                     span: span.clone() 
                 });
                 Type::Unknown
            }
        }
    }
}
