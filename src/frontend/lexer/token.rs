use crate::error::Span;
use super::token_kind::TokenKind;

// A single lexical unit in the source code.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    // Creates a new token with the given kind and source location.
    pub fn new(kind: TokenKind, line: usize, col: usize, len: usize) -> Self {
        Self {
            kind,
            span: Span { line, col, len },
        }
    }
}
