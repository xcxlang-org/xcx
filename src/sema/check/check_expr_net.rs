use crate::frontend::ast::{Expr, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_net_call(
        &mut self,
        url: &Expr,
        body: &Option<Box<Expr>>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) -> Type {
        let u_ty = self.check_expr(url, symbols, errors);
        if u_ty != Type::String && u_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::String, actual: u_ty }, span: url.span.clone() });
        }
        if let Some(b) = body {
            let b_ty = self.check_expr(b, symbols, errors);
            if b_ty != Type::Json && b_ty != Type::String && b_ty != Type::Unknown {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Json, actual: b_ty }, span: b.span.clone() });
            }
        }
        Type::Json
    }

    pub(crate) fn check_net_respond(
        &mut self,
        status: &Expr,
        body: &Expr,
        headers: &Option<Box<Expr>>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) -> Type {
        let s_ty = self.check_expr(status, symbols, errors);
        if s_ty != Type::Int && s_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: s_ty }, span: status.span.clone() });
        }
        let b_ty = self.check_expr(body, symbols, errors);
        if b_ty != Type::Json && b_ty != Type::String && b_ty != Type::Unknown {
            errors.push(TypeError {
                kind: TypeErrorKind::Other(format!("net.respond body must be String or Json, got {:?}", b_ty)),
                span: body.span.clone()
            });
        }
        if let Some(h) = headers {
            let h_ty = self.check_expr(h, symbols, errors);
            let expected_map = Type::Map(Box::new(Type::String), Box::new(Type::String));
            let expected_map_array = Type::Array(Box::new(expected_map.clone()));
            let expected_str_array = Type::Array(Box::new(Type::String));
            
            let ok = self.is_compatible(&expected_map, &h_ty) ||
                     self.is_compatible(&expected_map_array, &h_ty) ||
                     self.is_compatible(&expected_str_array, &h_ty);
            
            if h_ty != Type::Unknown && !ok {
                errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: expected_map, actual: h_ty }, span: h.span.clone() });
            }
        }
        Type::Json
    }
}
