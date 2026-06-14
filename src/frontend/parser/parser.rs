use crate::frontend::lexer::{Lexer, Token, TokenKind};
use crate::intern::{Interner, StringId};
use crate::frontend::ast::{Stmt, Program};

// The recursive descent and Pratt parser for XCX.
pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    pub interner: Interner,
    pub source: &'a str,
    pub current: Token,
    pub peek: Token,
    pub previous: Token,
    pub has_error: bool,
    pub depth: usize,
}

impl<'a> Parser<'a> {
    // Creates a new parser for the given source text.
    pub fn new(source: &'a str) -> Self {
        let lexer = Lexer::new(source);
        Self::new_with_interner(source, lexer, Interner::new())
    }

    // Creates a new parser using an existing interner and lexer.
    pub fn new_with_interner(source: &'a str, mut lexer: Lexer<'a>, mut interner: Interner) -> Self {
        let current = lexer.next_token(&mut interner);
        let peek = lexer.next_token(&mut interner);
        let previous = current.clone(); 
        Self {
            lexer,
            interner,
            source,
            current,
            peek,
            previous,
            has_error: false,
            depth: 0,
        }
    }

    // Calculates the precedence of the current token. (Moved to pratt.rs)
    // Calculates the precedence of the next token. (Moved to pratt.rs)

    // Consumes current interner state.
    pub fn into_interner(self) -> Interner {
        self.interner
    }


    // Parses a complete XCX program into an AST.
    pub fn parse_program(&mut self) -> Program {
        let mut stmts = Vec::new();
        while self.current.kind != TokenKind::EOF {
            if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
            } else {
                self.synchronize();
            }
        }
        Program { stmts }
    }

    // Helper to parse identifiers that can contain dots (e.g. math.sin).
    pub fn parse_identifier_as_string_id(&mut self, allow_dots: bool) -> Option<StringId> {
        let kind = self.current.kind.clone();
        let mut text = match kind {
            TokenKind::Identifier(id) => self.interner.lookup(id).to_string(),
            TokenKind::TypeI => "i".to_string(),
            TokenKind::TypeF => "f".to_string(),
            TokenKind::TypeS => "s".to_string(),
            TokenKind::TypeB => "b".to_string(),
            TokenKind::Choice => "choice".to_string(),
            TokenKind::Union => "union".to_string(),
            TokenKind::Intersection => "intersection".to_string(),
            TokenKind::Difference => "difference".to_string(),
            TokenKind::SymDifference => "symmetric_difference".to_string(),
            TokenKind::Alert => "alert".to_string(),
            TokenKind::Error => "error".to_string(),
            TokenKind::Fatal => "fatal".to_string(),
            TokenKind::Terminal => "terminal".to_string(),
            TokenKind::Store => "store".to_string(),
            TokenKind::Date => "date".to_string(),
            TokenKind::Json => "json".to_string(),
            TokenKind::Net => "net".to_string(),
            TokenKind::Random => "random".to_string(),
            TokenKind::Halt => "halt".to_string(),
            TokenKind::Columns => "columns".to_string(),
            TokenKind::Rows => "rows".to_string(),
            TokenKind::Schema => "schema".to_string(),
            TokenKind::Data => "data".to_string(),
            TokenKind::Empty => "EMPTY".to_string(),
            TokenKind::TypeSetN => "N".to_string(),
            TokenKind::TypeSetQ => "Q".to_string(),
            TokenKind::TypeSetZ => "Z".to_string(),
            TokenKind::TypeSetS => "S".to_string(),
            TokenKind::TypeSetB => "B".to_string(),
            TokenKind::TypeSetC => "C".to_string(),
            TokenKind::Set => "set".to_string(),
            TokenKind::Map => "map".to_string(),
            TokenKind::Table => "table".to_string(),
            TokenKind::Fiber => "fiber".to_string(),
            TokenKind::Serve => "serve".to_string(),
            TokenKind::Yield => "yield".to_string(),
            TokenKind::Return => "return".to_string(),
            TokenKind::Func => "func".to_string(),
            TokenKind::Array => "array".to_string(),
            TokenKind::Include => "include".to_string(),
            TokenKind::As => "as".to_string(),
            TokenKind::From => "from".to_string(),
            TokenKind::To => "to".to_string(),
            TokenKind::Has => "has".to_string(),
            _ => return None,
        };
        self.advance();

        if allow_dots {
            while self.current.kind == TokenKind::Dot {
                self.advance(); // past '.'
                if let Some(part_id) = self.parse_identifier_as_string_id(false) {
                    text.push('.');
                    text.push_str(self.interner.lookup(part_id));
                } else {
                    break;
                }
            }
        }

        Some(self.interner.intern(&text))
    }

    // Main entry point for parsing any single XCX statement.
    pub fn parse_statement(&mut self) -> Option<Stmt> {
        self.depth += 1;
        if self.depth > 500 {
            panic!("Parser recursion depth limit exceeded (current token: {:?})", self.current.kind);
        }
        let res = self.parse_statement_internal();
        self.depth -= 1;
        res
    }
}
