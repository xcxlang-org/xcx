use crate::frontend::lexer::TokenKind;
use crate::frontend::ast::{Stmt, StmtKind, Expr, ExprKind, Type, HaltLevel, Argument};
use super::parser::Parser;
use super::precedence::Precedence;

impl<'a> Parser<'a> {
    // Main entry point for parsing any single XCX statement.
    pub(super) fn parse_statement_internal(&mut self) -> Option<Stmt> {
        match self.current.kind {
            TokenKind::Const | TokenKind::TypeI | TokenKind::TypeF | TokenKind::TypeS | TokenKind::TypeB | TokenKind::Array | TokenKind::Set | TokenKind::Map | TokenKind::Date | TokenKind::Table | TokenKind::Json | TokenKind::Database | TokenKind::Crypto | TokenKind::Env | TokenKind::Store | TokenKind::Random => {
                if self.peek.kind == TokenKind::Equal {
                    self.parse_assignment()
                } else if self.peek.kind == TokenKind::Dot || self.peek.kind == TokenKind::LeftParen || self.peek.kind == TokenKind::LeftBracket {
                    self.parse_expr_stmt()
                } else if self.peek.kind == TokenKind::Colon {
                    if self.current.kind == TokenKind::Database {
                        self.parse_database_decl()
                    } else {
                        self.parse_var_decl()
                    }
                } else {
                    self.parse_var_decl()
                }
            }
            TokenKind::Identifier(id) if self.interner.lookup(id) == "var" => {
                self.parse_var_decl()
            }
            TokenKind::GreaterBang => {
                self.parse_print_stmt()
            }
            TokenKind::GreaterQuestion => {
                self.parse_input_stmt()
            }
            TokenKind::Halt => {
                self.parse_halt_stmt()
            }
            TokenKind::If => {
                self.parse_if_statement()
            }
            TokenKind::While => {
                self.parse_while_statement()
            }
            TokenKind::For => {
                self.parse_for_statement()
            }
            TokenKind::Break => {
                self.parse_break_statement()
            }
            TokenKind::Continue => {
                self.parse_continue_statement()
            }
            TokenKind::Dot => {
                let span = self.current.span.clone();
                if let Some(expr) = self.parse_terminal_expression() {
                    self.expect_semicolon();
                    return Some(Stmt {
                        kind: StmtKind::ExprStmt(Box::new(expr)),
                        span,
                    });
                }
                self.parse_expr_stmt()
            }
            TokenKind::Func => {
                self.parse_func_def()
            }
            TokenKind::Return => {
                self.parse_return_stmt()
            }
            TokenKind::Include => {
                self.parse_include_stmt()
            }
            TokenKind::Fiber => {
                self.parse_fiber_statement()
            }
            TokenKind::Yield => {
                self.parse_yield_stmt()
            }
            TokenKind::AtWait => {
                self.parse_wait_stmt()
            }
            TokenKind::Serve => {
                self.parse_serve_stmt()
            }
            TokenKind::Net => {
                if self.peek.kind == TokenKind::Dot {
                   // This could be net.request or just net.get (...)
                   // We'll peek further or let parse_net_stmt decide
                   self.parse_net_stmt()
                } else {
                   self.parse_expr_stmt()
                }
            }
            TokenKind::End => {
                None
            }
            TokenKind::Identifier(_) | TokenKind::Union | TokenKind::Intersection | TokenKind::Difference | TokenKind::SymDifference => {
                let peek_kind = self.peek.kind.clone();
                if peek_kind == TokenKind::Equal {
                    self.parse_assignment()
                } else if peek_kind == TokenKind::LeftParen {
                    self.parse_func_call_stmt()
                } else if matches!(peek_kind, TokenKind::Colon) {
                    self.parse_var_decl()
                } else {
                    self.parse_var_decl().or_else(|| self.parse_expr_stmt())
                }
            }
            _ => {
                self.parse_expr_stmt()
            }
        }
    }

    fn parse_wait_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); // past '@wait'

        let has_parens = self.current.kind == TokenKind::LeftParen;
        if has_parens {
            self.advance(); // past '('
        }
        
        let ms_expr = self.parse_expression(Precedence::Lowest)?;
        
        if has_parens {
            if self.current.kind != TokenKind::RightParen {
                self.error("Expected ')' to close '@wait(...)'.");
                return None;
            }
            self.advance(); // past ')'
        }
        self.expect_semicolon();
        
        Some(Stmt { kind: StmtKind::Wait(Box::new(ms_expr)), span })
    }

    fn parse_net_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); // past 'net'

        if self.current.kind != TokenKind::Dot {
            self.error("Expected '.' after 'net'.");
            return None;
        }
        self.advance(); // past '.'

        let method_name_id = if let Some(id) = self.parse_identifier_as_string_id(false) {
            id
        } else {
            self.error("Expected method name after 'net.'.");
            return None;
        };
        let method_name = self.interner.lookup(method_name_id).to_string();

        match method_name.as_str() {
            "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
                let method_id = self.interner.intern(&method_name);

                if self.current.kind != TokenKind::LeftParen {
                    self.error(&format!("Expected '(' after 'net.{}'.", method_name));
                    return None;
                }
                self.advance(); // past '('

                let url = self.parse_expression(Precedence::Lowest)?;
                

                let mut body = None;
                if self.current.kind == TokenKind::Comma {
                    self.advance(); // past ','
                    body = Some(Box::new(self.parse_expression(Precedence::Lowest)?));
                    
                }

                if self.current.kind != TokenKind::RightParen {
                    self.error(&format!("Expected ')' after 'net.{}' arguments.", method_name));
                    return None;
                }
                self.advance(); // past ')'

                if self.current.kind == TokenKind::As {
                    self.advance(); // past 'as'
                    let target = if let TokenKind::Identifier(t_id) = self.current.kind {
                        t_id
                    } else {
                        self.error("Expected identifier after 'as'.");
                        return None;
                    };
                    self.advance(); // past target
                    self.expect_semicolon();
                    return Some(Stmt {
                        kind: StmtKind::NetRequestStmt {
                            method: Box::new(Expr {
                                kind: ExprKind::StringLiteral(method_id),
                                span: span.clone(),
                            }),
                            url: Box::new(url),
                            headers: None,
                            body,
                            timeout: None,
                            target,
                        },
                        span,
                    });
                }

                self.expect_semicolon();
                let mut args_vec = Vec::new();
                args_vec.push(Argument::Positional(url));
                if let Some(b) = body {
                    args_vec.push(Argument::Positional(*b));
                }
                Some(Stmt {
                    kind: StmtKind::ExprStmt(Box::new(Expr {
                        kind: ExprKind::ModuleCall {
                            module: TokenKind::Net,
                            method: method_id,
                            args: args_vec,
                        },
                        span: span.clone(),
                    })),
                    span,
                })
            }

            "request" => {
                if self.current.kind != TokenKind::LeftBrace {
                    self.error("Expected '{' after 'net.request'.");
                    return None;
                }
                self.advance(); // past '{'

                let mut r_method = None;
                let mut r_url = None;
                let mut r_headers = None;
                let mut r_body = None;
                let mut r_timeout = None;

                while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
                    let field = if let Some(f_id) = self.parse_identifier_as_string_id(false) {
                        self.interner.lookup(f_id).to_string()
                    } else {
                        break;
                    };

                    if self.current.kind != TokenKind::Equal {
                        self.error(&format!("Expected '=' after '{}' in net.request.", field));
                        break;
                    }
                    self.advance(); // past '='

                    let val = self.parse_expression(Precedence::Lowest)?;
                    

                    match field.as_str() {
                        "method"  => r_method  = Some(Box::new(val)),
                        "url"     => r_url     = Some(Box::new(val)),
                        "headers" => r_headers = Some(Box::new(val)),
                        "body"    => r_body    = Some(Box::new(val)),
                        "timeout" => r_timeout = Some(Box::new(val)),
                        _ => {}
                    }

                    if self.current.kind == TokenKind::Comma {
                        self.advance();
                    }
                }

                if self.current.kind == TokenKind::RightBrace {
                    self.advance();
                }

                if self.current.kind == TokenKind::As {
                    self.advance(); // past 'as'
                    let target = if let TokenKind::Identifier(t_id) = self.current.kind {
                        t_id
                    } else {
                        self.error("Expected identifier after 'as'.");
                        return None;
                    };
                    self.advance(); // past target
                    self.expect_semicolon();
                    return Some(Stmt {
                        kind: StmtKind::NetRequestStmt {
                            method: r_method.unwrap_or_else(|| Box::new(Expr {
                                kind: ExprKind::StringLiteral(self.interner.intern("GET")),
                                span: span.clone(),
                            })),
                            url: r_url.unwrap_or_else(|| Box::new(Expr {
                                kind: ExprKind::StringLiteral(self.interner.intern("")),
                                span: span.clone(),
                            })),
                            headers: r_headers,
                            body: r_body,
                            timeout: r_timeout,
                            target,
                        },
                        span,
                    });
                }
                None
            }

            "respond" => {
                if self.current.kind != TokenKind::LeftParen {
                    self.error("Expected '(' after 'net.respond'.");
                    return None;
                }
                self.advance(); // past '('

                let status = self.parse_expression(Precedence::Lowest)?;
                

                if self.current.kind != TokenKind::Comma {
                    self.error("Expected ',' after status in 'net.respond'.");
                    return None;
                }
                self.advance(); // past ','

                let body = self.parse_expression(Precedence::Lowest)?;
                

                let mut headers = None;
                if self.current.kind == TokenKind::Comma {
                    self.advance(); // past ','
                    headers = Some(Box::new(self.parse_expression(Precedence::Lowest)?));
                    
                }

                if self.current.kind != TokenKind::RightParen {
                    self.error("Expected ')' after 'net.respond' arguments.");
                    return None;
                }
                self.advance(); // past ')'
                self.expect_semicolon();

                let mut args_vec = Vec::new();
                args_vec.push(Argument::Positional(status));
                args_vec.push(Argument::Positional(body));
                if let Some(h) = headers {
                    args_vec.push(Argument::Positional(*h));
                }

                Some(Stmt {
                    kind: StmtKind::ExprStmt(Box::new(Expr {
                        kind: ExprKind::ModuleCall {
                            module: TokenKind::Net,
                            method: self.interner.intern("respond"),
                            args: args_vec,
                        },
                        span: span.clone(),
                    })),
                    span,
                })
            }

            _ => {
                self.error(&format!("Unknown method 'net.{}'", method_name));
                None
            }
        }
    }



    fn parse_assignment(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        let name = self.parse_identifier_as_string_id(true)?;
        
        if self.current.kind != TokenKind::Equal { return None; }
        self.advance(); 
        
        let value = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_semicolon();
        
        Some(Stmt {
            kind: StmtKind::Assign { name, value: Box::new(value) },
            span,
        })
    }


    fn parse_expr_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        let expr = self.parse_expression(Precedence::Lowest)?;
        
        let kind = if let ExprKind::MethodCall { receiver, method, args, wait_after: _ } = expr.kind {
            let bind_id = self.interner.intern("bind");
            let inject_id = self.interner.intern("inject");
            
            if method == bind_id && args.len() == 2 {
                if let ExprKind::Identifier(target) = &args[1].expr().kind {
                    StmtKind::JsonBind {
                        json: receiver,
                        path: Box::new(args[0].expr().clone()),
                        target: *target,
                    }
                } else {
                    StmtKind::ExprStmt(Box::new(Expr {
                        kind: ExprKind::MethodCall { receiver, method, args, wait_after: false },
                        span: expr.span,
                    }))
                }
            } else if method == inject_id && args.len() == 2 {
                 if let ExprKind::Identifier(table_id) = &args[1].expr().kind {
                    StmtKind::JsonInject {
                        json: receiver,
                        mapping: Box::new(args[0].expr().clone()),
                        table: *table_id,
                    }
                } else if let ExprKind::StringLiteral(table_name) = &args[1].expr().kind {
                    StmtKind::JsonInject {
                        json: receiver,
                        mapping: Box::new(args[0].expr().clone()),
                        table: *table_name,
                    }
                } else {
                    StmtKind::ExprStmt(Box::new(Expr {
                        kind: ExprKind::MethodCall { receiver, method, args, wait_after: false },
                        span: expr.span,
                    }))
                }
            } else {
                StmtKind::ExprStmt(Box::new(Expr {
                    kind: ExprKind::MethodCall { receiver, method, args, wait_after: false },
                    span: expr.span,
                }))
            }
        } else {
            StmtKind::ExprStmt(Box::new(expr))
        };

        self.expect_semicolon();
        Some(Stmt { kind, span })
    }


    fn parse_input_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); 
        let name = self.parse_identifier_as_string_id(true)?;
        let ty = self.parse_type().unwrap_or(Type::String);
        self.expect_semicolon();
        Some(Stmt { kind: StmtKind::Input(name, Box::new(ty)), span })
    }

    fn parse_print_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); 
        let expr = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_semicolon();
        Some(Stmt { kind: StmtKind::Print(Box::new(expr)), span })
    }

    fn parse_halt_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.current.span.clone();
        self.advance(); 
        if self.current.kind != TokenKind::Dot { return None; }
        self.advance(); 
        let level = match self.current.kind {
            TokenKind::Alert => HaltLevel::Alert,
            TokenKind::Error => HaltLevel::Error,
            TokenKind::Fatal => HaltLevel::Fatal,
            _ => return None,
        };
        self.advance(); 
        if self.current.kind != TokenKind::GreaterBang { return None; }
        self.advance(); 
        let message = self.parse_expression(Precedence::Lowest)?;
        
        self.expect_semicolon();
        Some(Stmt { kind: StmtKind::Halt { level, message: Box::new(message) }, span: start_span })
    }
    fn parse_yield_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); // past 'yield'
        
        if self.current.kind == TokenKind::From {
            self.advance(); 
            let expr = self.parse_expression(Precedence::Lowest)?;
            self.expect_semicolon();
            return Some(Stmt { kind: StmtKind::YieldFrom(Box::new(expr)), span });
        }
        
        if self.current.kind == TokenKind::Semicolon {
            self.advance();
            return Some(Stmt { kind: StmtKind::YieldVoid, span });
        }
        
        let value = self.parse_expression(Precedence::Lowest)?;
        
        let mut target = None;
        if self.current.kind == TokenKind::As {
            self.advance(); // past 'as'
            if let TokenKind::Identifier(id) = self.current.kind {
                target = Some(id);
                self.advance();
            } else {
                self.error("Expected identifier after 'as' in yield statement.");
                return None;
            }
        }
        
        self.expect_semicolon();
        Some(Stmt { 
            kind: StmtKind::Yield { 
                value: Box::new(value), 
                target 
            }, 
            span 
        })
    }

    fn parse_return_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); 
        if self.current.kind == TokenKind::Semicolon {
            self.advance();
            return Some(Stmt { kind: StmtKind::Return(None), span });
        }
        let value = self.parse_expression(Precedence::Lowest);
        
        self.expect_semicolon();
        Some(Stmt { kind: StmtKind::Return(value.map(Box::new)), span })
    }

    fn parse_func_call_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        let name = self.parse_identifier_as_string_id(true)?;
        if self.current.kind != TokenKind::LeftParen { return None; }
        self.advance(); 
        let args = self.parse_arguments();
        if self.current.kind == TokenKind::RightParen { self.advance(); }
        self.expect_semicolon();
        Some(Stmt { kind: StmtKind::FunctionCallStmt { name, args }, span })
    }

}
