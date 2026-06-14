use crate::frontend::lexer::TokenKind;
use crate::frontend::ast::{Stmt, StmtKind, ForIterType};
use super::parser::Parser;
use super::precedence::Precedence;

impl<'a> Parser<'a> {
    pub fn parse_if_statement(&mut self) -> Option<Stmt> {
        let start_span = self.current.span.clone();
        self.advance(); // past 'if'
        
        if self.current.kind != TokenKind::LeftParen {
            self.error("Condition must be in '(...)'.");
            return None;
        }
        self.advance(); // past '('
        
        let condition = self.parse_expression(Precedence::Lowest)?;
        
        if self.current.kind != TokenKind::RightParen {
            self.error("Missing ')' after condition.");
            return None;
        }
        self.advance(); // past ')'
        
        if self.current.kind == TokenKind::Then {
            self.advance(); // past 'then'
        }
        if self.current.kind == TokenKind::Semicolon {
            self.advance();
        }
        
        let mut then_branch = Vec::new();
        while !matches!(self.current.kind, TokenKind::ElseIf | TokenKind::Else | TokenKind::End | TokenKind::EOF) {
            if let Some(stmt) = self.parse_statement() {
                then_branch.push(stmt);
            } else {
                self.advance();
            }
        }

        let mut else_ifs = Vec::new();
        while self.current.kind == TokenKind::ElseIf {
            self.advance(); // past 'elseif'
            if self.current.kind != TokenKind::LeftParen { return None; }
            self.advance(); // past '('
            let cond = self.parse_expression(Precedence::Lowest)?;
            if self.current.kind != TokenKind::RightParen { return None; }
            self.advance();
            if self.current.kind == TokenKind::Then { self.advance(); }
            self.expect_semicolon();
            
            let mut branch = Vec::new();
            while !matches!(self.current.kind, TokenKind::ElseIf | TokenKind::Else | TokenKind::End | TokenKind::EOF) {
                if let Some(stmt) = self.parse_statement() {
                    branch.push(stmt);
                } else {
                    self.advance();
                }
            }
            else_ifs.push((cond, branch));
        }

        let mut else_branch = None;
        if self.current.kind == TokenKind::Else {
            self.advance(); // past 'else'
            if self.current.kind == TokenKind::Semicolon { self.advance(); }
            let mut branch = Vec::new();
            while self.current.kind != TokenKind::End && self.current.kind != TokenKind::EOF {
                if let Some(stmt) = self.parse_statement() {
                    branch.push(stmt);
                } else {
                    self.advance();
                }
            }
            else_branch = Some(branch);
        }

        if self.current.kind == TokenKind::End {
            self.advance();
            self.expect_semicolon();
        }
        
        Some(Stmt {
            kind: StmtKind::If { condition: Box::new(condition), then_branch, else_ifs: else_ifs.into_iter().map(|(c, b)| (Box::new(c), b)).collect(), else_branch },
            span: start_span,
        })
    }

    pub fn parse_while_statement(&mut self) -> Option<Stmt> {
        let start_span = self.current.span.clone();
        self.advance(); // past 'while'
        
        if self.current.kind != TokenKind::LeftParen { return None; }
        self.advance(); // past '('
        let condition = self.parse_expression(Precedence::Lowest)?;
        
        if self.current.kind != TokenKind::RightParen { return None; }
        self.advance();
        if self.current.kind == TokenKind::Do { self.advance(); }
        self.expect_semicolon();

        let mut body = Vec::new();
        while self.current.kind != TokenKind::End && self.current.kind != TokenKind::EOF {
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            } else {
                self.advance();
            }
        }

        if self.current.kind == TokenKind::End {
            self.advance();
            self.expect_semicolon();
        }

        Some(Stmt {
            kind: StmtKind::While { condition: Box::new(condition), body },
            span: start_span,
        })
    }

    pub fn parse_for_statement(&mut self) -> Option<Stmt> {
        let start_span = self.current.span.clone();
        self.advance(); // past 'for'

        let var_name = self.parse_identifier_as_string_id(true)?;

        if self.current.kind != TokenKind::In { return None; }
        self.advance();

        let start = self.parse_expression(Precedence::Lowest)?;
        
        let (end, step, iter_type) = if self.current.kind == TokenKind::To || self.current.kind == TokenKind::DoubleColon {
            self.advance();
            let end = self.parse_expression(Precedence::Lowest)?;
            
            let mut step = None;
            if self.current.kind == TokenKind::AtStep {
                self.advance();
                step = Some(self.parse_expression(Precedence::Lowest)?);
            }
            (end, step, ForIterType::Range)
        } else {
            let mut step = None;
            if self.current.kind == TokenKind::AtStep {
                self.advance();
                step = Some(self.parse_expression(Precedence::Lowest)?);
            }
            (crate::frontend::ast::Expr { kind: crate::frontend::ast::ExprKind::IntLiteral(0), span: start_span.clone() }, step, ForIterType::Array)
        };

        if self.current.kind == TokenKind::Do { self.advance(); }
        self.expect_semicolon();

        let mut body = Vec::new();
        while self.current.kind != TokenKind::End && self.current.kind != TokenKind::EOF {
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            } else {
                self.advance();
            }
        }

        if self.current.kind == TokenKind::End {
            self.advance();
            self.expect_semicolon();
        }

        Some(Stmt {
            kind: StmtKind::For { 
                var_name, 
                start: Box::new(start), 
                end: Box::new(end), 
                step: step.map(Box::new), 
                body, 
                iter_type 
            },
            span: start_span,
        })
    }

    pub fn parse_break_statement(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); 
        self.expect_semicolon();
        Some(Stmt { kind: StmtKind::Break, span })
    }

    pub fn parse_continue_statement(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); 
        self.expect_semicolon();
        Some(Stmt { kind: StmtKind::Continue, span })
    }
}
