use crate::frontend::ast::{Expr, ExprKind, Type, Argument};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub fn check_expr(&mut self, expr: &Expr, symbols: &mut SymbolTable<'_>, errors: &mut Vec<TypeError>) -> Type {
        self.check_expr_with_context(expr, symbols, errors, None)
    }

    pub fn check_expr_with_context(&mut self, expr: &Expr, symbols: &mut SymbolTable<'_>, errors: &mut Vec<TypeError>, context: Option<Type>) -> Type {
        let span = expr.span.clone();
        match &expr.kind {
            &ExprKind::TerminalCommand(_, _) => Type::Unknown,
            &ExprKind::Tag(_) => Type::String,
            ExprKind::IntLiteral(_) => Type::Int,
            ExprKind::FloatLiteral(_) => Type::Float,
            ExprKind::StringLiteral(_) => Type::String,
            ExprKind::BoolLiteral(_) => Type::Bool,
            &ExprKind::Identifier(id) => {
                self.check_identifier(id, symbols, errors, &span)
            }
            ExprKind::RawBlock(_) => Type::Json,
            ExprKind::ArrayLiteral { elements } => {
                self.check_array_literal(elements, symbols, errors, context)
            }
            ExprKind::Binary { left, op, right } => {
                self.check_binary_expr(left, op, right, symbols, errors, &span)
            }
            ExprKind::Unary { op, right } => {
                self.check_unary_expr(op, right, symbols, errors, &span)
            }
            ExprKind::FunctionCall { name, args } => {
                self.check_function_call(name, args, symbols, errors, &span)
            }
            ExprKind::SetLiteral { set_type, elements, range } => {
                self.check_set_literal(set_type, elements, range, symbols, errors)
            }
            ExprKind::ArrayOrSetLiteral { elements } => {
                self.check_array_or_set_literal(elements, symbols, errors, context)
            }
            ExprKind::RandomChoice { set } => {
                self.check_random_choice(set, symbols, errors)
            }
            ExprKind::RandomInt { min, max, step } => {
                self.check_random_int(min, max, step, symbols, errors)
            }
            ExprKind::RandomFloat { min, max, step } => {
                self.check_random_float(min, max, step, symbols, errors)
            }
            ExprKind::MapLiteral { key_type, value_type, elements } => {
                self.check_map_literal(key_type, value_type, elements, symbols, errors)
            }
            ExprKind::DateLiteral { .. } => Type::Date,
            ExprKind::TableLiteral { columns, rows } => {
                self.check_table_literal(columns, rows, symbols, errors, &span)
            }
            ExprKind::DatabaseLiteral(fields) => {
                for (_, v) in fields {
                    self.check_expr(v, symbols, errors);
                }
                Type::Database
            }
            ExprKind::Yield(expr) => {
                self.check_yield_expr(expr, symbols, errors, &span)
            }
            ExprKind::As { expr, name } => {
                let ty = self.check_expr(expr, symbols, errors);
                let name_str = self.interner.lookup(*name).trim().to_string();
                symbols.define(name_str, ty.clone(), false);
                ty
            }
            ExprKind::Index { receiver, index } => {
                let rec_ty = self.check_expr(receiver, symbols, errors);
                let idx_ty = self.check_expr(index, symbols, errors);
                match rec_ty {
                    Type::Array(inner) => {
                        if idx_ty != Type::Int && idx_ty != Type::Unknown {
                            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: idx_ty }, span: index.span.clone() });
                        }
                        *inner
                    }
                    Type::Table(columns) => {
                        if idx_ty != Type::Int && idx_ty != Type::Unknown {
                            errors.push(TypeError { kind: TypeErrorKind::TypeMismatch { expected: Type::Int, actual: idx_ty }, span: index.span.clone() });
                        }
                        Type::Table(columns)
                    }
                    Type::Builtin(id) if self.interner.lookup(id) == "net" => Type::Json,
                    Type::Json => Type::Json,
                    Type::Unknown => Type::Unknown,
                    _ => {
                        errors.push(TypeError { kind: TypeErrorKind::IndexAccessNotSupported(rec_ty), span });
                        Type::Unknown
                    }
                }
            }
            ExprKind::ModuleCall { module, method, args } => {
                self.check_module_call(module, *method, args, symbols, errors, &span)
            }
            ExprKind::Lambda { .. } => Type::Unknown,
            ExprKind::Tuple(exprs) => {
                for e in exprs { self.check_expr(e, symbols, errors); }
                Type::Array(Box::new(Type::Unknown))
            }
            ExprKind::MemberAccess { receiver, member } => {
                self.check_member_access(receiver, *member, symbols, errors, &span)
            }
            ExprKind::MethodCall { receiver, method, args, .. } => {
                let method_str = self.interner.lookup(*method).trim().to_string();
                self.check_method_call(receiver, &method_str, args, symbols, errors, &span)
            }
        }
    }

    pub(crate) fn check_argument_list(&mut self, args: &Vec<Argument>, symbols: &mut SymbolTable<'_>, errors: &mut Vec<TypeError>, allow_named: bool) {
        let mut seen_named = false;
        let mut seen_names = std::collections::HashSet::new();
        
        for arg_node in args {
            match arg_node {
                Argument::Positional(expr) => {
                    if seen_named {
                        errors.push(TypeError {
                            kind: TypeErrorKind::Other("Positional arguments must come before named arguments".to_string()),
                            span: expr.span.clone(),
                        });
                    }
                    let ty = self.check_expr(expr, symbols, errors);
                    if let Type::DatabaseOperation(crate::frontend::ast::DatabaseOpKind::Remove, _) = ty {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::Other("Rule D401: remove() requires .where() filter before passing as argument".to_string()), 
                            span: expr.span.clone() 
                        });
                    }
                }
                Argument::Named(name, expr) => {
                    if !allow_named {
                        errors.push(TypeError {
                            kind: TypeErrorKind::Other(format!("Named arguments are not allowed here (method: {})", self.interner.lookup(*name).trim())),
                            span: expr.span.clone(),
                        });
                    }
                    seen_named = true;
                    let name_str = self.interner.lookup(*name).trim().to_string();
                    if !seen_names.insert(name_str.clone()) {
                        errors.push(TypeError {
                            kind: TypeErrorKind::Other(format!("Duplicate named argument: {}", name_str)),
                            span: expr.span.clone(),
                        });
                    }
                    let ty = self.check_expr(expr, symbols, errors);
                    if let Type::DatabaseOperation(crate::frontend::ast::DatabaseOpKind::Remove, _) = ty {
                        errors.push(TypeError { 
                            kind: TypeErrorKind::Other("Rule D401: remove() requires .where() filter before passing as argument".to_string()), 
                            span: expr.span.clone() 
                        });
                    }
                }
            }
        }
    }
}
