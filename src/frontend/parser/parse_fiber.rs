use crate::frontend::lexer::TokenKind;
use crate::frontend::ast::{Stmt, StmtKind, Type};
use crate::intern::StringId;
use crate::error::Span;
use super::parser::Parser;

impl<'a> Parser<'a> {
    pub fn parse_fiber_statement(&mut self) -> Option<Stmt> {
        if self.peek.kind == TokenKind::Colon { self.parse_fiber_decl() } else { self.parse_fiber_def() }
    }

    pub fn parse_fiber_def(&mut self) -> Option<Stmt> {
        let start_span = self.current.span.clone();
        self.advance(); 
        let name = self.parse_identifier_as_string_id(true)?;
        self.finish_fiber_def(start_span, name, None)
    }

    pub fn finish_fiber_def(&mut self, start_span: Span, name: StringId, return_type: Option<Type>) -> Option<Stmt> {
        if self.current.kind != TokenKind::LeftParen { return None; }
        self.advance(); 
        let mut params = Vec::new();
        let mut return_type = return_type;
        while self.current.kind != TokenKind::RightParen && self.current.kind != TokenKind::EOF {
            if self.current.kind == TokenKind::Arrow { self.advance(); return_type = self.parse_type(); break; }
            let ty = self.parse_type()?;
            if self.current.kind != TokenKind::Colon { break; }
            self.advance(); 
            let param_name = self.parse_identifier_as_string_id(false)?;
            params.push((ty, param_name));
            if self.current.kind == TokenKind::Comma { self.advance(); }
        }
        if self.current.kind == TokenKind::RightParen { self.advance(); }
        if self.current.kind != TokenKind::LeftBrace { return None; }
        self.advance(); 
        let mut body = Vec::new();
        while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
            if let Some(stmt) = self.parse_statement() { body.push(stmt); } else { self.advance(); }
        }
        if self.current.kind == TokenKind::RightBrace { self.advance(); self.expect_semicolon(); }
        Some(Stmt { kind: StmtKind::FiberDef { name, params, return_type: return_type.map(Box::new), body }, span: start_span })
    }

    pub fn parse_fiber_decl(&mut self) -> Option<Stmt> {
        let start_span = self.current.span.clone();
        self.advance(); 
        self.advance(); 
        let inner_type = if let TokenKind::Identifier(_) = self.current.kind {
            if self.peek.kind == TokenKind::Equal { None } else {
                let ty = self.parse_type();
                if self.current.kind == TokenKind::Colon { self.advance(); }
                ty
            }
        } else {
            let ty = self.parse_type();
            if self.current.kind == TokenKind::Colon { self.advance(); }
            ty
        };
        let name = self.parse_identifier_as_string_id(true)?;
        if self.current.kind == TokenKind::LeftParen { return self.finish_fiber_def(start_span, name, inner_type); }
        if self.current.kind != TokenKind::Equal { return None; }
        self.advance(); 
        let fiber_name = self.parse_identifier_as_string_id(true)?;
        if self.current.kind != TokenKind::LeftParen { return None; }
        self.advance(); 
        let args = self.parse_arguments();
        if self.current.kind == TokenKind::RightParen { self.advance(); }
        self.expect_semicolon();
        Some(Stmt { kind: StmtKind::FiberDecl { inner_type: inner_type.map(Box::new), name, fiber_name, args }, span: start_span })
    }
}
