use crate::frontend::ast::{Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::intern::StringId;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_identifier(
        &mut self,
        id: StringId,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let name_raw = self.interner.lookup(id);
        let name = name_raw.trim();
        match name {
            "json" | "date" | "store" | "halt" | "terminal" | "net" | "env" | "crypto" => return Type::Builtin(id),
            _ => {}
        }
        let name_trimmed = name.to_string();
        
        if let Some(ty) = symbols.lookup(&name_trimmed) { return ty; }
        
        if let Some(sig) = self.functions.get(&name_trimmed) {
            if sig.is_fiber {
                return Type::Fiber(sig.return_type.clone().map(Box::new));
            }
            return sig.return_type.clone().unwrap_or(Type::Unknown);
        } else if self.is_table_lambda {
            if let Some(row_ty) = symbols.lookup("__row_tmp") {
                if let Type::Table(cols) = row_ty {
                    for col in &cols {
                        if self.interner.lookup(col.name) == name {
                            return col.ty.clone();
                        }
                    }
                }
            }
            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(name.to_string()), span: span.clone() });
            Type::Unknown
        } else {
            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(name.to_string()), span: span.clone() });
            Type::Unknown
        }
    }
}
