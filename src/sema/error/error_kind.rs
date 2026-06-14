use crate::frontend::ast::Type;
use crate::frontend::lexer::TokenKind;

#[derive(Debug, PartialEq)]
pub enum TypeErrorKind {
    UndefinedVariable(String),
    RedefinedVariable(String),
    ConstReassignment(String),
    TypeMismatch { expected: Type, actual: Type },
    InvalidBinaryOp { op: TokenKind, left: Type, right: Type },
    BreakOutsideLoop,
    ContinueOutsideLoop,
    YieldOutsideFiber,
    FiberTypeMismatch,
    ReturnTypeMismatchInFiber,
    WherePredicateNameCollision { var_name: String, column_name: String },
    IndexAccessNotSupported(Type),
    PropertyNotFound { base_type: Type, property: String },
    MethodNotFound { base_type: Type, method: String },
    TableRowCountMismatch { expected: usize, actual: usize },
    InvalidArgumentCount { expected: usize, actual: usize },
    CannotIterateOverVoidFiber,
    CannotRunTypedFiber,
    Other(String),
}

impl TypeErrorKind {
    pub fn to_diagnostic_message(&self) -> String {
        match self {
            TypeErrorKind::UndefinedVariable(name) =>
                format!("[S101] Undefined variable: {}", name),
            TypeErrorKind::RedefinedVariable(name) =>
                format!("[S102] Redefined variable: {}", name),
            TypeErrorKind::TypeMismatch { expected, actual } =>
                format!("[S103] Type mismatch: expected {}, got {}", expected, actual),
            TypeErrorKind::InvalidBinaryOp { op, left, right } =>
                format!("[S104] Invalid operation {:?} between {} and {}", op, left, right),
            TypeErrorKind::ConstReassignment(name) =>
                format!("[S105] Cannot reassign to constant variable: {}", name),
            TypeErrorKind::BreakOutsideLoop =>
                "[S106] Break statement outside of loop".to_string(),
            TypeErrorKind::ContinueOutsideLoop =>
                "[S107] Continue statement outside of loop".to_string(),
            TypeErrorKind::IndexAccessNotSupported(ty) =>
                format!("[S108] Index access not supported for type {}", ty),
            TypeErrorKind::PropertyNotFound { base_type, property } =>
                format!("[S109] Property '{}' not found on type {}", property, base_type),
            TypeErrorKind::MethodNotFound { base_type, method } =>
                format!("[S110] Method '{}' not found on type {}", method, base_type),
            TypeErrorKind::InvalidArgumentCount { expected, actual } =>
                format!("[S111] Incorrect number of arguments: expected {}, got {}", expected, actual),
            TypeErrorKind::YieldOutsideFiber =>
                "[S208] 'yield' used outside a fiber body".to_string(),
            TypeErrorKind::FiberTypeMismatch =>
                "[S209] Cannot use 'yield expr;' inside a void fiber — use 'yield;' instead".to_string(),
            TypeErrorKind::ReturnTypeMismatchInFiber =>
                "[S210] Typed fiber requires 'return expr;' not plain 'return;'".to_string(),
            TypeErrorKind::CannotIterateOverVoidFiber =>
                "[S211] Cannot iterate over a void fiber (fiber without yield)".to_string(),
            TypeErrorKind::CannotRunTypedFiber =>
                "[S212] Cannot call .run() on a typed fiber (use for loop instead)".to_string(),
            TypeErrorKind::WherePredicateNameCollision { var_name, column_name } =>
                format!("[S301] Variable name '{}' conflicts with column '{}' in .where() predicate — rename the local variable",
                    var_name, column_name),
            TypeErrorKind::TableRowCountMismatch { expected, actual } =>
                format!("[S302] Table row has {} columns, but schema expects {}", actual, expected),
            TypeErrorKind::Other(msg) => msg.clone(),
        }
    }
}
