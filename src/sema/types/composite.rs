use super::ty::Type;

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType {
    pub element: Box<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapType {
    pub key: Box<Type>,
    pub value: Box<Type>,
}
