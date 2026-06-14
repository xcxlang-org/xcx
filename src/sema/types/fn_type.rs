use crate::frontend::ast::Type;

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Option<Type>,
    pub is_fiber: bool,
}
