use crate::intern::StringId;

// Represents all valid XCX literal values in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    Int(i64),
    Float(f64),
    String(StringId),
    Bool(bool),
    Null,
}
