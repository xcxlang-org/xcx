use super::parser::Parser;

impl<'a> Parser<'a> {
    // Advances the parser by one token.
    pub fn advance(&mut self) {
        self.previous = self.current.clone();
        self.current = self.peek.clone();
        self.peek = self.lexer.next_token(&mut self.interner);
    }
}
