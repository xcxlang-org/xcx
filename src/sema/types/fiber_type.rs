use super::ty::Type;

#[derive(Debug, Clone, PartialEq)]
pub struct FiberType {
    pub return_type: Option<Box<Type>>,
}
