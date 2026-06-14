use crate::frontend::ast::{Argument, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use crate::intern::StringId;
use crate::error::span::Span;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_function_call_stmt(
        &mut self,
        name: &StringId,
        args: &mut Vec<Argument>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) {
        let name_str = self.interner.lookup(*name).trim().to_string();
        let resolved_sig = self.functions.get(&name_str).cloned();

        self.check_argument_list(args, symbols, errors, false);

        if let Some(sig) = resolved_sig {
            if args.len() != sig.params.len() {
                errors.push(TypeError {
                    kind: TypeErrorKind::InvalidArgumentCount { expected: sig.params.len(), actual: args.len() },
                    span: span.clone(),
                });
            }
        } else if !symbols.has(&name_str) {
            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(name_str.to_string()), span: span.clone() });
        }
    }

    pub(crate) fn check_function_call(
        &mut self,
        name: &StringId,
        args: &Vec<Argument>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
        span: &Span,
    ) -> Type {
        let name_str = self.interner.lookup(*name).trim().to_string();
        let mut resolved_sig = self.functions.get(&name_str).cloned();

        if resolved_sig.is_none() {
            if let Some(ty) = symbols.lookup(&name_str) {
                match ty {
                    Type::Fiber(inner) => {
                        resolved_sig = Some(super::checker::FunctionSignature {
                            params: vec![Type::Unknown; args.len()],
                            return_type: inner.map(|t| *t),
                            is_fiber: true,
                        });
                    }
                    _ => {
                        resolved_sig = Some(super::checker::FunctionSignature {
                            params: vec![Type::Unknown; args.len()],
                            return_type: Some(Type::Unknown),
                            is_fiber: false,
                        });
                    }
                }
            }
        }

        if let Some(sig) = resolved_sig {
            let params = sig.params.clone();
            let ret = sig.return_type.clone().unwrap_or(Type::Unknown);
            if args.len() != sig.params.len() {
                errors.push(TypeError {
                    kind: TypeErrorKind::InvalidArgumentCount { expected: sig.params.len(), actual: args.len() },
                    span: span.clone(),
                });
            }

            if name_str == "i" {
                if let Some(arg) = args.first() {
                    let arg_ty = self.check_expr(arg.expr(), symbols, errors);
                    if self.is_compatible(&Type::Bool, &arg_ty) {
                        errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: arg_ty }, span: arg.expr().span.clone() });
                    }
                    return Type::Int;
                }
            }

            for (arg, expected) in args.iter().zip(params) {
                let arg_ty = self.check_expr(arg.expr(), symbols, errors);
                if arg_ty != Type::Unknown && !self.is_compatible(&expected, &arg_ty) {
                    errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected, actual: arg_ty }, span: arg.expr().span.clone() });
                }
            }
            if sig.is_fiber {
                Type::Fiber(Some(Box::new(ret)))
            } else {
                ret
            }
        } else {
            for arg in args {
                self.check_expr(arg.expr(), symbols, errors);
            }
            errors.push(TypeError { kind: TypeErrorKind::UndefinedVariable(name_str.to_string()), span: span.clone() });
            Type::Unknown
        }
    }
}
