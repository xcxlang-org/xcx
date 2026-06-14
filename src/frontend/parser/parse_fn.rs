use crate::frontend::lexer::TokenKind;
use crate::frontend::ast::{Stmt, StmtKind, Type};
use super::parser::Parser;

impl<'a> Parser<'a> {
    pub fn parse_func_def(&mut self) -> Option<Stmt> {
        let start_span = self.current.span.clone();
        self.advance(); 
        let mut return_type = None;
        let name = self.parse_identifier_as_string_id(true)?;
        if self.current.kind != TokenKind::LeftParen { return None; }
        self.advance(); 
        let mut params = Vec::new();
        while self.current.kind != TokenKind::RightParen && self.current.kind != TokenKind::EOF {
            if self.current.kind == TokenKind::Arrow {
                self.advance();
                return_type = self.parse_type();
                break;
            }
            let ty = self.parse_type().unwrap_or(Type::Unknown);
            if self.current.kind == TokenKind::Colon {
                self.advance(); 
                let param_name = self.parse_identifier_as_string_id(false)?;
                params.push((ty, param_name));
            } else if let TokenKind::Identifier(id) = self.current.kind {
                self.advance();
                params.push((ty, id));
            } else {
                break;
            }
            if self.current.kind == TokenKind::Comma { self.advance(); }
        }
        if self.current.kind == TokenKind::RightParen { self.advance(); }
        if self.current.kind == TokenKind::Arrow {
            self.error("Return type declaration outside parameter list is prohibited. Specify the return type inside the parameter list (e.g. 'func main(i: x, i: y -> i)').");
            return None;
        }
        let mut body = Vec::new();
        if self.current.kind != TokenKind::LeftBrace { return None; }
        self.advance(); 
        while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
            if let Some(stmt) = self.parse_statement() { body.push(stmt); } else { self.advance(); }
        }
        if self.current.kind == TokenKind::RightBrace { self.advance(); self.expect_semicolon(); }
        Some(Stmt { kind: StmtKind::FunctionDef { name, params, return_type: return_type.map(Box::new), body }, span: start_span })
    }
}
