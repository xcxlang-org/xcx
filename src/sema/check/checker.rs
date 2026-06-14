use crate::frontend::ast::{Program, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::intern::Interner;
use crate::sema::error::type_error::TypeError;
use std::collections::HashMap;

/// Core semantic analyzer and type checker for the XCX language.
/// Maintains state about current fiber context, loop depth, and interner
/// to perform thorough validation of the AST before compilation.
pub struct Checker<'a> {
    pub(crate) interner: &'a Interner,
    pub(crate) loop_depth: usize,
    pub(crate) functions: HashMap<String, FunctionSignature>,
    pub(crate) fiber_context: Option<Option<Type>>,
    pub(crate) is_fiber_context: bool,
    pub(crate) is_table_lambda: bool,
    pub(crate) fiber_has_yield: bool,
    pub(crate) in_yield_expr: bool,
    pub(crate) last_expr_was_db_io: bool,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Option<Type>,
    pub is_fiber: bool,
}

impl<'a> Checker<'a> {
    pub fn new(interner: &'a Interner) -> Self {
        Self {
            interner,
            loop_depth: 0,
            functions: HashMap::new(),
            fiber_context: None,
            is_fiber_context: false,
            is_table_lambda: false,
            fiber_has_yield: false,
            in_yield_expr: false,
            last_expr_was_db_io: false,
        }
    }

    pub fn check(&mut self, program: &mut Program, symbols: &mut SymbolTable<'_>) -> Vec<TypeError> {
        let mut errors = Vec::new();

        symbols.define("input".to_string(), Type::Unknown, true);

        self.functions.insert("i".to_string(), FunctionSignature { params: vec![Type::Unknown], return_type: Some(Type::Int), is_fiber: false });
        self.functions.insert("f".to_string(), FunctionSignature { params: vec![Type::Unknown], return_type: Some(Type::Float), is_fiber: false });
        self.functions.insert("s".to_string(), FunctionSignature { params: vec![Type::Unknown], return_type: Some(Type::String), is_fiber: false });
        self.functions.insert("b".to_string(), FunctionSignature { params: vec![Type::Unknown], return_type: Some(Type::Bool), is_fiber: false });

        self.pre_scan_stmts(&program.stmts, symbols);

        let mut serve_found = false;

        for stmt in &mut program.stmts {
            if serve_found {
                errors.push(TypeError {
                    kind: crate::sema::error::error_kind::TypeErrorKind::Other("Rule S401: serve: must be the terminal statement. No code allowed after serve:".to_string()),
                    span: stmt.span.clone(),
                });
            }

            if matches!(stmt.kind, crate::frontend::ast::StmtKind::Serve { .. }) {
                if serve_found {
                    errors.push(TypeError {
                        kind: crate::sema::error::error_kind::TypeErrorKind::Other("Only one serve: statement is allowed in a program".to_string()),
                        span: stmt.span.clone(),
                    });
                }
                serve_found = true;
            }

            self.check_stmt(stmt, symbols, &mut errors);
        }
        errors
    }

    pub(crate) fn is_compatible(&self, expected: &Type, actual: &Type) -> bool {
        crate::sema::types::compat::is_compatible(expected, actual)
    }
}
