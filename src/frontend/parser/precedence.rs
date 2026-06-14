use crate::frontend::lexer::TokenKind;

// Binding power levels for the XCX Pratt parser.
#[derive(Debug, PartialOrd, PartialEq, Clone, Copy)]
pub enum Precedence {
    Lowest,
    Lambda,      // ->
    Assignment,  // =
    LogicalOr,   // OR, ||
    LogicalAnd,  // AND, &&
    Equals,      // == !=
    LessGreater, // > < >= <= HAS
    Sum,         // + -
    SetOp,       // UNION, INTERSECTION, DIFFERENCE, SYMMETRIC_DIFFERENCE, ∪, ∩, \, ⊕
    Product,     // * / %
    Power,       // ^
    Prefix,      // -x
    Concatenation, // ::
    Call,        // f(x)
    AsPrec,      // as
}

impl Precedence {
    // Returns the precedence level for a given token type.
    pub fn for_token(kind: &TokenKind) -> Precedence {
        match kind {
            TokenKind::Arrow => Precedence::Lambda,
            TokenKind::Equal => Precedence::Assignment,
            TokenKind::Or => Precedence::LogicalOr,
            TokenKind::And => Precedence::LogicalAnd,
            TokenKind::EqualEqual | TokenKind::BangEqual => Precedence::Equals,
            TokenKind::Less | TokenKind::Greater | TokenKind::LessEqual | TokenKind::GreaterEqual | TokenKind::Has => Precedence::LessGreater,
            TokenKind::Plus | TokenKind::Minus | TokenKind::PlusPlus => Precedence::Sum,
            TokenKind::Union | TokenKind::Intersection | TokenKind::Difference | TokenKind::SymDifference => Precedence::SetOp,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Product,
            TokenKind::Caret => Precedence::Power,
            TokenKind::DoubleColon => Precedence::Concatenation,
            TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::Dot => Precedence::Call,
            TokenKind::As => Precedence::AsPrec,
            _ => Precedence::Lowest,
        }
    }
}
