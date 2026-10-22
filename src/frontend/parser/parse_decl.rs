use crate::frontend::lexer::TokenKind;
use crate::frontend::ast::{Stmt, StmtKind, Expr, ExprKind, Type, SetRange};
use crate::error::Span;
use super::parser::Parser;
use super::precedence::Precedence;

impl<'a> Parser<'a> {
    pub fn parse_var_decl(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        let mut is_const = false;
        if self.current.kind == TokenKind::Const {
            is_const = true;
            self.advance();
        }

        let is_var_inferred = if let TokenKind::Identifier(id) = self.current.kind {
            self.interner.lookup(id) == "var"
        } else {
            false
        };

        let mut ty = if is_var_inferred {
            self.advance(); 
            if self.current.kind == TokenKind::Colon { self.advance(); }
            Type::Unknown
        } else {
            let ty_opt = self.parse_type();
            if ty_opt.is_none() {
                if is_const { self.error("Type required for const."); }
                return None; 
            }
            ty_opt.unwrap()
        };

        let is_map = matches!(ty, Type::Map(_, _));
        let is_table = matches!(ty, Type::Table(_));
        let is_database = matches!(ty, Type::Database);

        if self.current.kind == TokenKind::Colon { self.advance(); }

        if is_map || is_table || is_database {
            let name = self.parse_identifier_as_string_id(true)?;
            let value = if is_map {
                if self.current.kind == TokenKind::Equal { self.advance(); }
                if self.current.kind == TokenKind::LeftBrace || self.current.kind == TokenKind::Map {
                    let map_lit = self.parse_map_literal()?;
                    if let ExprKind::MapLiteral { ref key_type, ref value_type, .. } = map_lit.kind {
                        ty = Type::Map(key_type.clone(), value_type.clone());
                    }
                    self.advance();
                    if self.current.kind == TokenKind::Semicolon { self.advance(); }
                    Some(map_lit)
                } else {
                    let val = self.parse_expression(Precedence::Lowest)?;
                    self.expect_semicolon();
                    Some(val)
                }
            } else if is_table {
                if self.current.kind == TokenKind::Equal { self.advance(); }
                if self.current.kind == TokenKind::LeftBrace || self.current.kind == TokenKind::Table {
                    let table_lit = self.parse_table_literal()?;
                    if let ExprKind::TableLiteral { ref columns, .. } = table_lit.kind {
                        ty = Type::Table(columns.clone().into());
                    }

                    self.advance();
                    if self.current.kind == TokenKind::Semicolon { self.advance(); }
                    Some(table_lit)
                } else {
                    let val = self.parse_expression(Precedence::Lowest)?;
                    self.expect_semicolon();
                    Some(val)
                }
            } else {
                if self.current.kind == TokenKind::Equal { self.advance(); }
                if self.current.kind == TokenKind::LeftBrace || self.current.kind == TokenKind::Database {
                    let db_lit = self.parse_database_literal()?;
                    self.advance();
                    if self.current.kind == TokenKind::Semicolon { self.advance(); }
                    Some(db_lit)
                } else {
                    let val = self.parse_expression(Precedence::Lowest)?;
                    self.expect_semicolon();
                    Some(val)
                }
            };

            return Some(Stmt {
                kind: StmtKind::VarDecl { is_const, ty: Box::new(ty), name, value: value.map(Box::new) },
                span,
            });
        }

        let mut decls = Vec::new();
        // A declaration whose value is a `{ ... }` literal terminates at the
        // closing brace, mirroring `func`/`fiber`/data-literal blocks: the
        // trailing `;` is optional for those and required for flat
        // declarations (`i: x = 5;`), where it is the only statement
        // terminator.
        let mut ends_with_brace_literal;
        loop {
            let decl_span = self.current.span.clone();
            let name = self.parse_identifier_as_string_id(true)?;
            ends_with_brace_literal = false;
            let value = if self.current.kind == TokenKind::Equal || matches!(self.current.kind, TokenKind::RawBlock(_)) {
                if self.current.kind == TokenKind::Equal { self.advance(); }
                // The RawBlock self-parses at runtime (LoadConst + JsonParse for
                // parseable content; unparseable content keeps plain-string
                // behaviour per json_http.md). Wrapping it in an extra
                // json.parse(…) re-parses the already-parsed value — scalars
                // lose their json-ness and JSON strings hit R305.
                Some(self.parse_expression(Precedence::Lowest)?)
            } else if self.current.kind == TokenKind::LeftBrace {
                ends_with_brace_literal = true;
                let lit_span = self.current.span.clone();
                self.advance(); 
                if let Type::Set(st) = &ty {
                    let st = st.clone();
                    let mut elements = Vec::new();
                    let mut range = None;
                    if self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
                        let first_expr = self.parse_expression(Precedence::Lowest)?;
                        if self.current.kind == TokenKind::DoubleComma {
                            self.advance();
                            let end_expr = self.parse_expression(Precedence::Lowest)?;
                            let mut step_expr = None;
                            if self.current.kind == TokenKind::AtStep {
                                self.advance();
                                let s_expr = self.parse_expression(Precedence::Lowest)?;
                                step_expr = Some(Box::new(s_expr));
                            }
                            range = Some(SetRange { start: Box::new(first_expr), end: Box::new(end_expr), step: step_expr });
                        } else {
                            elements.push(first_expr);
                            if self.current.kind == TokenKind::Comma { self.advance(); }
                            while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
                                if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                                    elements.push(expr);
                                }
                                if self.current.kind == TokenKind::Comma { self.advance(); }
                            }
                        }
                    }
                    if self.current.kind == TokenKind::RightBrace { self.advance(); }
                    Some(Expr { kind: ExprKind::SetLiteral { set_type: st, elements, range }, span: lit_span })
                } else {
                    let mut elements = Vec::new();
                    while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
                        if let Some(expr) = self.parse_expression(Precedence::Lowest) { elements.push(expr); }
                        if self.current.kind == TokenKind::Comma { self.advance(); }
                    }
                    if self.current.kind == TokenKind::RightBrace { self.advance(); }
                    Some(Expr { kind: ExprKind::ArrayLiteral { elements }, span: lit_span })
                }
            } else {
                None
            };

            decls.push(Stmt {
                kind: StmtKind::VarDecl {
                    is_const,
                    ty: Box::new(ty.clone()),
                    name,
                    value: value.map(Box::new),
                },
                span: decl_span,
            });

            if self.current.kind == TokenKind::Comma {
                self.advance();
            } else {
                break;
            }
        }

        if ends_with_brace_literal {
            if self.current.kind == TokenKind::Semicolon { self.advance(); }
        } else {
            self.expect_semicolon();
        }

        if decls.len() == 1 {
            Some(decls.pop().unwrap())
        } else {
            Some(Stmt {
                kind: StmtKind::MultiVarDecl(decls),
                span,
            })
        }
    }

    pub fn parse_include_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); // past 'include'

        let path = if let TokenKind::StringLiteral(id) = self.current.kind {
            id
        } else {
            self.error("Expected file path string after 'include'.");
            return None;
        };
        self.advance(); // past path string

        let mut alias = None;
        if self.current.kind == TokenKind::As {
            self.advance(); // past 'as'
            if let TokenKind::Identifier(id) = self.current.kind {
                alias = Some(id);
                self.advance();
            } else {
                self.error("Expected identifier after 'as' in 'include'.");
                return None;
            }
        }

        self.expect_semicolon();

        Some(Stmt {
            kind: StmtKind::Include { path, alias },
            span,
        })
    }

    pub fn parse_serve_stmt(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); // past 'serve'
        self.parse_serve_stmt_headless(span)
    }

    pub fn parse_serve_stmt_headless(&mut self, span: Span) -> Option<Stmt> {
        if self.current.kind != TokenKind::Colon { 
            self.error("Expected ':' after 'serve'.");
            return None; 
        }
        self.advance(); // past ':'
        let name = self.parse_identifier_as_string_id(true)?;
        if self.current.kind != TokenKind::LeftBrace { 
            self.error("Expected '{' for serve configuration.");
            return None; 
        }
        self.advance(); // past '{'
        let mut port = None;
        let mut host = None;
        let mut workers = None;
        let mut routes = None;
        while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
             let field_id = self.parse_identifier_as_string_id(false)?;
             let field = self.interner.lookup(field_id).to_string();
             // parse_identifier_as_string_id already advanced past the field name
             if self.current.kind != TokenKind::Equal { 
                 self.error(&format!("Expected '=' after field '{}' in serve.", field));
                 break; 
             }
             self.advance(); // past '='
             let val = self.parse_expression(Precedence::Lowest)?;
             
             match field.as_str() {
                 "port"    => port    = Some(Box::new(val)),
                 "host"    => host    = Some(Box::new(val)),
                 "workers" => workers = Some(Box::new(val)),
                 "routes"  => routes  = Some(Box::new(val)),
                 _ => {}
             }
             if self.current.kind == TokenKind::Comma { self.advance(); }
        }
        if self.current.kind != TokenKind::RightBrace { 
            self.error("Expected '}' at end of serve configuration.");
            return None; 
        }
        self.advance(); // past '}'
        if self.current.kind == TokenKind::Semicolon { self.advance(); }
        if routes.is_none() {
            self.error("Property 'routes' is required in serve configuration.");
            return None;
        }
        Some(Stmt {
            kind: StmtKind::Serve {
                name,
                port: port.unwrap_or_else(|| Box::new(Expr { kind: ExprKind::IntLiteral(8080), span: span.clone() })),
                host,
                workers,
                routes: routes.unwrap(),
            },
            span,
        })
    }

    pub fn parse_database_decl(&mut self) -> Option<Stmt> {
        let span = self.current.span.clone();
        self.advance(); // past 'database'
        
        if self.current.kind != TokenKind::Colon {
             self.error("Expected ':' after 'database'.");
             return None;
        }
        self.advance(); // past ':'
        
        let name = self.parse_identifier_as_string_id(true)?;
        
        if self.current.kind != TokenKind::LeftBrace {
            self.error("Expected '{' for database declaration.");
            return None;
        }
        self.advance(); // past '{'
        
        let mut fields = Vec::new();
        while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
             let field_id = self.parse_identifier_as_string_id(false)?;
             if self.current.kind != TokenKind::Equal && self.current.kind != TokenKind::Colon {
                 self.error("Expected '=' or ':' in database declaration.");
                 break;
             }
             self.advance();
             let val = self.parse_expression(Precedence::Lowest)?;
             fields.push((field_id, Box::new(val)));
             
             if self.current.kind == TokenKind::Comma {
                 self.advance();
             }
        }
        
        if self.current.kind != TokenKind::RightBrace {
            self.error("Expected '}' at end of database declaration.");
            return None;
        }
        self.advance(); // past '}'
        if self.current.kind == TokenKind::Semicolon { self.advance(); }

        Some(Stmt {
            kind: StmtKind::DatabaseDecl { name, fields },
            span,
        })
    }
}
