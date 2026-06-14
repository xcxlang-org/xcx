// Binary operators supported by XCX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    EqualEqual,
    BangEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    And,
    Or,
    PlusPlus,   // String concatenation
    DoubleColon, // Collection concatenation
    Union,
    Intersection,
    Difference,
    SymDifference,
    Has,
    Bridge,
}

// Unary operators supported by XCX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Minus,
    Not,
}
