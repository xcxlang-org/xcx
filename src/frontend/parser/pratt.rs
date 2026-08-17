use crate::frontend::lexer::TokenKind;
use crate::frontend::ast::Expr;
use super::parser::Parser;
use super::precedence::Precedence;

impl<'a> Parser<'a> {
    // Parses an expression based on the current precedence level.
    pub fn parse_expression(&mut self, precedence: Precedence) -> Option<Expr> {
        self.depth += 1;
        if self.depth > 500 {
            panic!("Parser recursion depth limit exceeded in parse_expression (current token: {:?})", self.current.kind);
        }
        let mut left = self.parse_prefix()?;

        while self.current.kind != TokenKind::Semicolon && self.current.kind != TokenKind::EOF && precedence < self.current_precedence() {
            left = self.parse_infix(left)?;
        }

        self.depth -= 1;
        Some(left)
    }

    // Calculates the precedence of the current token.
    pub fn current_precedence(&self) -> Precedence {
        match self.current.kind {
            TokenKind::Arrow => Precedence::Lambda,
            TokenKind::Equal => Precedence::Assignment,
            TokenKind::Or => Precedence::LogicalOr,
            TokenKind::And => Precedence::LogicalAnd,
            TokenKind::EqualEqual | TokenKind::BangEqual => Precedence::Equals,
            TokenKind::Less | TokenKind::Greater | TokenKind::LessEqual | TokenKind::GreaterEqual | TokenKind::Has => Precedence::LessGreater,
            TokenKind::Plus | TokenKind::Minus | TokenKind::PlusPlus => Precedence::Sum,
            TokenKind::DoubleColon => Precedence::Concatenation,
            TokenKind::Union | TokenKind::Difference | TokenKind::SymDifference | TokenKind::Intersection => Precedence::SetOp,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Product,
            TokenKind::Caret => Precedence::Power,
            TokenKind::Dot | TokenKind::LeftBracket => Precedence::Call,
            TokenKind::As => Precedence::AsPrec,
            _ => Precedence::Lowest,
        }
    }
}
