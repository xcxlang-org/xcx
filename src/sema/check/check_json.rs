use crate::frontend::ast::{Expr, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::error::span::Span;
use crate::intern::StringId;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_json_bind(
        &mut self,
        json: &mut Expr,
        path: &mut Expr,
        target: StringId,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let j_ty = self.check_expr(json, symbols, errors);
        if j_ty != Type::Json && j_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Json, actual: j_ty }, span: json.span.clone() });
        }
        let p_ty = self.check_expr(path, symbols, errors);
        if p_ty != Type::String && p_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::String, actual: p_ty }, span: path.span.clone() });
        }
        let name_str = self.interner.lookup(target).trim().to_string();
        if !symbols.has(&name_str) {
            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(name_str), span: span.clone() });
        }
    }

    pub(crate) fn check_json_inject(
        &mut self,
        json: &mut Expr,
        mapping: &mut Expr,
        table: StringId,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let j_ty = self.check_expr(json, symbols, errors);
        if j_ty != Type::Json && j_ty != Type::Unknown {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Json, actual: j_ty }, span: json.span.clone() });
        }
        let m_ty = self.check_expr(mapping, symbols, errors);
        if m_ty != Type::Unknown && !matches!(m_ty, Type::Map(_, _)) {
            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Map(Box::new(Type::String), Box::new(Type::String)), actual: m_ty }, span: mapping.span.clone() });
        }
        let table_str = self.interner.lookup(table).trim().to_string();
        if !symbols.has(&table_str) {
            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(table_str), span: span.clone() });
        }
    }
}
