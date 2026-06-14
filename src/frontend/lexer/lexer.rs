use crate::intern::Interner;
use super::cursor::Cursor;
use super::token::Token;
use super::token_kind::TokenKind;
use super::{keyword, number, string_lit, trivia};

// The main lexical analyzer for XCX source code.
// Transforms source text into a stream of tokens.
pub struct Lexer<'a> {
    cursor: Cursor<'a>,
}

impl<'a> Lexer<'a> {
    // Creates a new lexer for the given source text.
    pub fn new(source: &'a str) -> Self {
        Self {
            cursor: Cursor::new(source),
        }
    }

    // Returns the next token from the source stream.
    pub fn next_token(&mut self, interner: &mut Interner) -> Token {
        trivia::skip(&mut self.cursor);

        let line = self.cursor.line();
        let col = self.cursor.col();

        if self.cursor.is_at_end() {
             return Token::new(TokenKind::EOF, line, col, 0);
        }

        let c = self.cursor.advance();

        if c.is_ascii_digit() {
            self.cursor.backtrack(1);
            return number::lex(&mut self.cursor, line, col);
        }

        if c.is_ascii_alphabetic() || c == b'_' {
            return self.identifier(line, col, interner);
        }

        // Handle UTF-8 symbols for set operations
        if c >= 128 {
            self.cursor.backtrack(1);
            let source = self.cursor.source;
            let pos = self.cursor.pos;
            let remain = &source[pos..];

            if remain.starts_with("∪".as_bytes()) {
                for _ in 0..3 { self.cursor.advance(); }
                return Token::new(TokenKind::Union, line, col, 1);
            } else if remain.starts_with("∩".as_bytes()) {
                for _ in 0..3 { self.cursor.advance(); }
                return Token::new(TokenKind::Intersection, line, col, 1);
            } else if remain.starts_with("⊕".as_bytes()) {
                for _ in 0..3 { self.cursor.advance(); }
                return Token::new(TokenKind::SymDifference, line, col, 1);
            }
            self.cursor.advance(); // consume the byte again
            return self.identifier(line, col, interner);
        }

        match c {
            b'(' => Token::new(TokenKind::LeftParen, line, col, 1),
            b')' => Token::new(TokenKind::RightParen, line, col, 1),
            b'{' => Token::new(TokenKind::LeftBrace, line, col, 1),
            b'}' => Token::new(TokenKind::RightBrace, line, col, 1),
            b'[' => Token::new(TokenKind::LeftBracket, line, col, 1),
            b']' => Token::new(TokenKind::RightBracket, line, col, 1),
            b',' => {
                if self.cursor.peek() == b',' {
                    self.cursor.advance();
                    Token::new(TokenKind::DoubleComma, line, col, 2)
                } else {
                    Token::new(TokenKind::Comma, line, col, 1)
                }
            }
            b'.' => {
                if self.cursor.peek() == b'.' {
                    self.cursor.advance();
                    Token::new(TokenKind::To, line, col, 2)
                } else {
                    Token::new(TokenKind::Dot, line, col, 1)
                }
            }
            b';' => Token::new(TokenKind::Semicolon, line, col, 1),
            b':' => {
                if self.cursor.peek() == b':' {
                    self.cursor.advance();
                    Token::new(TokenKind::DoubleColon, line, col, 2)
                } else {
                    Token::new(TokenKind::Colon, line, col, 1)
                }
            }
            b'+' => {
                if self.cursor.peek() == b'+' {
                    self.cursor.advance();
                    Token::new(TokenKind::PlusPlus, line, col, 2)
                } else {
                    Token::new(TokenKind::Plus, line, col, 1)
                }
            }
            b'-' => {
                if self.cursor.peek() == b'>' {
                    self.cursor.advance();
                    Token::new(TokenKind::Arrow, line, col, 2)
                } else {
                    Token::new(TokenKind::Minus, line, col, 1)
                }
            }
            b'*' => Token::new(TokenKind::Star, line, col, 1),
            b'/' => {
                let peek = self.cursor.peek();
                if peek == b'/' || peek == b'*' {
                    panic!("\n[XCX Error] C-style comments (// or /* */) are NOT supported in XCX.\nUse '---' for single-line and '---' ... '*---' for multi-line comments.\nConsult the documentation at: documentation/language/syntax.md\n");
                }
                Token::new(TokenKind::Slash, line, col, 1)
            },
            b'%' => Token::new(TokenKind::Percent, line, col, 1),
            b'^' => Token::new(TokenKind::Caret, line, col, 1),
            b'!' => {
                if self.cursor.peek() == b'=' {
                    self.cursor.advance();
                    Token::new(TokenKind::BangEqual, line, col, 2)
                } else if self.cursor.peek() == b'!' {
                    self.cursor.advance();
                    Token::new(TokenKind::Not, line, col, 2)
                } else {
                    Token::new(TokenKind::Bang, line, col, 1)
                }
            }
            b'=' => {
                if self.cursor.peek() == b'=' {
                    self.cursor.advance();
                    Token::new(TokenKind::EqualEqual, line, col, 2)
                } else {
                    Token::new(TokenKind::Equal, line, col, 1)
                }
            }
            b'<' => {
                if self.cursor.peek() == b'=' {
                    self.cursor.advance();
                    Token::new(TokenKind::LessEqual, line, col, 2)
                } else if self.cursor.peek() == b'-' && self.cursor.peek_next() == b'>' {
                    self.cursor.advance();
                    self.cursor.advance();
                    Token::new(TokenKind::Bridge, line, col, 3)
                } else if self.cursor.peek() == b'<' && self.cursor.peek_next() == b'<' {
                    self.cursor.advance();
                    self.cursor.advance();
                    return string_lit::lex_raw(&mut self.cursor, line, col, interner);
                } else {
                    Token::new(TokenKind::Less, line, col, 1)
                }
            }
            b'>' => {
                if self.cursor.peek() == b'=' {
                    self.cursor.advance();
                    Token::new(TokenKind::GreaterEqual, line, col, 2)
                } else if self.cursor.peek() == b'!' {
                    self.cursor.advance();
                    Token::new(TokenKind::GreaterBang, line, col, 2)
                } else if self.cursor.peek() == b'?' {
                    self.cursor.advance();
                    Token::new(TokenKind::GreaterQuestion, line, col, 2)
                } else {
                    Token::new(TokenKind::Greater, line, col, 1)
                }
            }
            b'@' => {
                let start_col = col;
                let mut name_bytes = Vec::new();
                let mut offset = 0;
                while self.cursor.peek_at(offset).is_ascii_alphabetic() {
                    name_bytes.push(self.cursor.peek_at(offset));
                    offset += 1;
                }
                let name = std::str::from_utf8(&name_bytes).unwrap_or("");
                match name {
                    "step" => {
                        for _ in 0..name.len() { self.cursor.advance(); }
                        Token::new(TokenKind::AtStep, line, start_col, name.len() + 1)
                    }
                    "auto" => {
                        for _ in 0..name.len() { self.cursor.advance(); }
                        Token::new(TokenKind::AtAuto, line, start_col, name.len() + 1)
                    }
                    "wait" => {
                        for _ in 0..name.len() { self.cursor.advance(); }
                        Token::new(TokenKind::AtWait, line, start_col, name.len() + 1)
                    }
                    "pk" => {
                        for _ in 0..name.len() { self.cursor.advance(); }
                        Token::new(TokenKind::AtPk, line, start_col, name.len() + 1)
                    }
                    "unique" => {
                        for _ in 0..name.len() { self.cursor.advance(); }
                        Token::new(TokenKind::AtUnique, line, start_col, name.len() + 1)
                    }
                    "optional" => {
                        for _ in 0..name.len() { self.cursor.advance(); }
                        Token::new(TokenKind::AtOptional, line, start_col, name.len() + 1)
                    }
                    "default" => {
                        for _ in 0..name.len() { self.cursor.advance(); }
                        Token::new(TokenKind::AtDefault, line, start_col, name.len() + 1)
                    }
                    "fk" => {
                        for _ in 0..name.len() { self.cursor.advance(); }
                        Token::new(TokenKind::AtFk, line, start_col, name.len() + 1)
                    }
                    _ => Token::new(TokenKind::Unknown('@'), line, start_col, 1),
                }
            }
            b'&' => {
                if self.cursor.peek() == b'&' {
                    self.cursor.advance();
                    Token::new(TokenKind::And, line, col, 2)
                } else {
                    Token::new(TokenKind::Unknown('&'), line, col, 1)
                }
            }
            b'|' => {
                if self.cursor.peek() == b'|' {
                    self.cursor.advance();
                    Token::new(TokenKind::Or, line, col, 2)
                } else {
                    Token::new(TokenKind::Unknown('|'), line, col, 1)
                }
            }
            b'\"' => {
                string_lit::lex(&mut self.cursor, line, col, interner)
            }
            b'#' => {
                let start_pos = self.cursor.pos;
                let start_char_pos = self.cursor.char_pos - 1;
                while self.cursor.peek().is_ascii_alphanumeric() || self.cursor.peek() == b'_' {
                    self.cursor.advance();
                }
                let span_len = self.cursor.char_pos - start_char_pos;
                let source = self.cursor.source;
                let tag_text = std::str::from_utf8(&source[start_pos..self.cursor.pos]).unwrap_or("");
                Token::new(TokenKind::Tag(interner.intern(tag_text)), line, col, span_len)
            }
            b'\\' => Token::new(TokenKind::Difference, line, col, 1),
            _ => {
                let ch = c as char;
                Token::new(TokenKind::Unknown(ch), line, col, 1)
            }
        }
    }

    fn identifier(&mut self, line: usize, col: usize, interner: &mut Interner) -> Token {
        let start_byte_pos = self.cursor.pos - 1;
        let start_char_pos = self.cursor.char_pos - 1;
        while self.cursor.peek().is_ascii_alphanumeric() || self.cursor.peek() == b'_' || self.cursor.peek() >= 128 {
            self.cursor.advance();
        }
        let source = self.cursor.source;
        let text_bytes = &source[start_byte_pos..self.cursor.pos];
        let text = std::str::from_utf8(text_bytes).unwrap_or("");
        
        let kind = keyword::lookup(text, interner);
        Token::new(kind, line, col, self.cursor.char_pos - start_char_pos)
    }
}
