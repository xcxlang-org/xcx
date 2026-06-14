use crate::frontend::lexer::TokenKind;
use super::parser::Parser;

impl<'a> Parser<'a> {
    // Reports a syntax error at the current token's location.
    pub fn error(&mut self, message: &str) {
        let mut reporter = crate::error::Reporter::new(self.source);
        reporter.error(self.current.span.line, self.current.span.col, self.current.span.len, message);
        self.has_error = true;
    }

    // Synchronizes the parser state after an error by skipping until a synchronization point.
    pub fn synchronize(&mut self) {
        while self.current.kind != TokenKind::EOF {
            if self.current.kind == TokenKind::Semicolon {
                self.advance();
                return;
            }

            match self.current.kind {
                TokenKind::Func | TokenKind::Fiber | TokenKind::Const | TokenKind::If | TokenKind::While | TokenKind::For | TokenKind::Return | TokenKind::GreaterBang => return,
                _ => {}
            }

            self.advance();
        }
    }

    // Consumes the current token if it matches the kind, otherwise reports an error.
    pub fn expect(&mut self, kind: TokenKind, message: &str) -> bool {
        if self.current.kind == kind {
            self.advance();
            true
        } else {
            self.error(message);
            false
        }
    }

    // Specialized check for semicolon with improved error messaging.
    pub fn expect_semicolon(&mut self) -> bool {
        if self.current.kind == TokenKind::Semicolon {
            self.advance();
            true
        } else {
            let msg = match self.previous.kind {
                TokenKind::End => "Expected ';' after 'end'. Every block must be terminated with 'end;'.".to_string(),
                TokenKind::Then => "Expected ';' after 'then'. Did you forget the semicolon?".to_string(),
                TokenKind::Do => "Expected ';' after 'do'. Did you forget the semicolon?".to_string(),
                _ => "Expected ';' at the end of the statement.".to_string(),
            };
            self.error(&msg);
            false
        }
    }
}
