use crate::intern::StringId;
use super::ty::Type;

// Represents a function or fiber signature.
#[derive(Debug, Clone, PartialEq)]
pub struct FnSig {
    pub params: Vec<(Type, StringId)>,
    pub return_type: Option<Type>,
    pub is_variadic: bool, // Placeholder for future expansion
}
