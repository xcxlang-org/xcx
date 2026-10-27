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
        let failed_span = self.current.span;
        while self.current.kind != TokenKind::EOF {
            if self.current.kind == TokenKind::Semicolon {
                self.advance();
                return;
            }

            match self.current.kind {
                TokenKind::Func | TokenKind::Fiber | TokenKind::Const | TokenKind::If | TokenKind::While | TokenKind::For | TokenKind::Return | TokenKind::GreaterBang => {
                    // A statement keyword is a sync point only when it starts
                    // a new statement. When synchronization begins on the very
                    // token the failed statement started from, returning here
                    // would retry the same failure forever, so that token is
                    // skipped and scanning continues to the next boundary.
                    if self.current.span != failed_span {
                        return;
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::Parser;

    /// Malformed `const` forms must terminate with a reported compile error.
    /// Regression test: `const(` used to loop forever in parse_program —
    /// synchronize() returned at the failed statement's own keyword without
    /// consuming it — until the leaked depth counter panicked the parser.
    #[test]
    fn malformed_const_reports_error_and_terminates() {
        for src in ["const(\n", "const(1);\n", "const = 5;\n"] {
            let mut parser = Parser::new(src);
            let program = parser.parse_program();
            assert!(parser.has_error, "expected a compile error for {src:?}");
            assert!(program.stmts.is_empty(), "expected no statements for {src:?}");
        }

        // Mid-file: parsing must terminate and flag the error; statements
        // after it may legitimately be dropped once an error is recorded.
        let mut parser = Parser::new(">! \"a\";\nconst(1);\n>! \"b\";\n");
        let _program = parser.parse_program();
        assert!(parser.has_error);
    }

    #[test]
    fn valid_const_declaration_still_parses() {
        let mut parser = Parser::new("const i: MAX = 42;\n");
        let program = parser.parse_program();
        assert!(!parser.has_error);
        assert_eq!(program.stmts.len(), 1);
    }
}
