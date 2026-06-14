use crate::frontend::ast::{Expr, ExprKind, SetType, SetRange, Argument, Type};
use crate::frontend::lexer::TokenKind;
use crate::error::Span;
use crate::intern::StringId;
use super::parser::Parser;
use super::precedence::Precedence;

impl<'a> Parser<'a> {

    // Handles prefix operators and literals.
    pub(super) fn parse_prefix(&mut self) -> Option<Expr> {
        let span = self.current.span.clone();
        match &self.current.kind {
            TokenKind::Identifier(_) => {
                let id_val = if let Some(id) = self.parse_identifier_as_string_id(false) {
                    id
                } else {
                    return None;
                };
                let id_span = span.clone();
                if self.current.kind == TokenKind::LeftParen {
                    self.advance(); // past '('
                    let args = self.parse_arguments();
                    if self.current.kind == TokenKind::RightParen {
                        self.advance();
                    }
                    Some(Expr {
                        kind: ExprKind::FunctionCall { name: id_val, args },
                        span: id_span,
                    })
                } else {
                    Some(Expr { kind: ExprKind::Identifier(id_val), span })
                }
            }
            TokenKind::TypeI | TokenKind::TypeF | TokenKind::TypeS | TokenKind::TypeB | 
            TokenKind::TypeSetN | TokenKind::TypeSetQ | TokenKind::TypeSetZ | TokenKind::TypeSetS | TokenKind::TypeSetB | TokenKind::TypeSetC |
            TokenKind::Choice | TokenKind::Union | TokenKind::Intersection | TokenKind::Difference | TokenKind::SymDifference |
            TokenKind::And | TokenKind::Or | TokenKind::Terminal | TokenKind::Empty |
            TokenKind::Func | TokenKind::Fiber | TokenKind::Return |
            TokenKind::Break | TokenKind::Continue | TokenKind::As | TokenKind::From | TokenKind::To => {
                let text = match self.current.kind {
                    TokenKind::TypeI => "i",
                    TokenKind::TypeF => "f",
                    TokenKind::TypeS => "s",
                    TokenKind::TypeB => "b",
                    TokenKind::TypeSetN => "N",
                    TokenKind::TypeSetQ => "Q",
                    TokenKind::TypeSetZ => "Z",
                    TokenKind::TypeSetS => "S",
                    TokenKind::TypeSetB => "B",
                    TokenKind::TypeSetC => "C",
                    TokenKind::Choice => "choice",
                    TokenKind::Union => "union",
                    TokenKind::Intersection => "intersection",
                    TokenKind::Difference => "difference",
                    TokenKind::SymDifference => "symmetric_difference",
                    TokenKind::And => "AND",
                    TokenKind::Or => "OR",
                    TokenKind::Not => "NOT",
                    TokenKind::Terminal => "terminal",
                    TokenKind::Empty => "EMPTY",
                    TokenKind::Func => "func",
                    TokenKind::Fiber => "fiber",
                    TokenKind::Return => "return",
                    TokenKind::Break => "break",
                    TokenKind::Continue => "continue",
                    TokenKind::As => "as",
                    TokenKind::From => "from",
                    TokenKind::To => "to",
                    _ => unreachable!(),
                };
                let id = self.interner.intern(text);
                let id_span = span.clone();
                self.advance();
                
                if self.current.kind == TokenKind::LeftParen {
                    self.advance(); // past '('
                    let args = self.parse_arguments();
                    if self.current.kind == TokenKind::RightParen {
                        self.advance();
                    }
                    Some(Expr {
                        kind: ExprKind::FunctionCall { name: id, args },
                        span: id_span,
                    })
                } else {
                    Some(Expr { kind: ExprKind::Identifier(id), span })
                }
            }
            TokenKind::Alert | TokenKind::Error | TokenKind::Fatal | TokenKind::Columns | TokenKind::Rows | TokenKind::Schema | TokenKind::Data => {
                let text = match self.current.kind {
                    TokenKind::Alert => "alert",
                    TokenKind::Error => "error",
                    TokenKind::Fatal => "fatal",
                    TokenKind::Columns => "columns",
                    TokenKind::Rows => "rows",
                    TokenKind::Schema => "schema",
                    TokenKind::Data => "data",
                    _ => unreachable!(),
                };
                let id = self.interner.intern(text);
                self.advance();
                Some(Expr { kind: ExprKind::Identifier(id), span })
            }
            TokenKind::IntLiteral(val) => {
                let v = *val;
                self.advance();
                Some(Expr { kind: ExprKind::IntLiteral(v), span })
            }
            TokenKind::FloatLiteral(val) => {
                let v = *val;
                self.advance();
                Some(Expr { kind: ExprKind::FloatLiteral(v), span })
            }
            TokenKind::StringLiteral(id) => {
                let id = *id;
                self.advance();
                Some(Expr { kind: ExprKind::StringLiteral(id), span })
            }
            TokenKind::RawBlock(id) => {
                let id = *id;
                self.advance();
                Some(Expr { kind: ExprKind::RawBlock(id), span })
            }
            TokenKind::True => {
                self.advance();
                Some(Expr { kind: ExprKind::BoolLiteral(true), span })
            }
            TokenKind::False => {
                self.advance();
                Some(Expr { kind: ExprKind::BoolLiteral(false), span })
            }
            TokenKind::Minus | TokenKind::Bang | TokenKind::Not => {
                let op = self.current.kind.clone();
                self.advance();
                let right = self.parse_expression(Precedence::Prefix)?;
                Some(Expr {
                    kind: ExprKind::Unary {
                        op,
                        right: Box::new(right),
                    },
                    span,
                })
            }
            TokenKind::LeftParen => {
                self.advance(); // past '('
                let mut exprs = Vec::new();
                if self.current.kind != TokenKind::RightParen {
                    loop {
                        if let Some(e) = self.parse_expression(Precedence::Lowest) {
                            exprs.push(e);
                        } else {
                            return None;
                        }
                        if self.current.kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                if self.current.kind != TokenKind::RightParen { return None; }
                self.advance(); // past ')'
                
                if exprs.len() == 1 {
                    Some(exprs.remove(0))
                } else {
                    Some(Expr { kind: ExprKind::Tuple(exprs), span: span.clone() })
                }
            }
            TokenKind::Set => {
                self.advance(); // past 'set'
                if self.current.kind != TokenKind::Colon {
                    return Some(Expr { kind: ExprKind::Identifier(self.interner.intern("set")), span });
                }
                self.advance(); // past ':'
                let st = match self.current.kind {
                    TokenKind::TypeSetN => SetType::N,
                    TokenKind::TypeSetQ => SetType::Q,
                    TokenKind::TypeSetZ => SetType::Z,
                    TokenKind::TypeSetS => SetType::S,
                    TokenKind::TypeSetB => SetType::B,
                    TokenKind::TypeSetC => SetType::C,
                    _ => return None,
                };
                self.advance(); // past TypeSetX
                if self.current.kind == TokenKind::LeftBrace {
                    self.advance(); // past '{'
                    self.parse_set_literal_content(st, span)
                } else {
                    let name = match st {
                        SetType::N => "set:N",
                        SetType::Q => "set:Q",
                        SetType::Z => "set:Z",
                        SetType::S => "set:S",
                        SetType::B => "set:B",
                        SetType::C => "set:C",
                    };
                    Some(Expr { kind: ExprKind::Identifier(self.interner.intern(name)), span })
                }
            }
            TokenKind::LeftBrace => {
                let lit_span = span.clone();
                if self.peek.kind == TokenKind::Schema {
                    return self.parse_map_literal();
                }
                
                self.advance(); // past '{'
                let elements = self.parse_array_or_set_literal_elements(TokenKind::RightBrace);
                if self.current.kind == TokenKind::RightBrace {
                    self.advance(); // past '}'
                }
                Some(Expr { kind: ExprKind::ArrayOrSetLiteral { elements }, span: lit_span })
            }
            TokenKind::LeftBracket => {
                let lit_span = span.clone();
                self.advance(); // past '['
                
                if self.current.kind == TokenKind::RightBracket {
                    return Some(Expr {
                        kind: ExprKind::ArrayLiteral { elements: Vec::new() },
                        span: lit_span,
                    });
                }

                let first_expr = self.parse_expression(Precedence::Lowest)?;
                
                if self.current.kind == TokenKind::DoubleColon {
                    self.advance(); // past '::'
                    let first_val = self.parse_expression(Precedence::Lowest)?;
                    
                    let mut elements = vec![(first_expr, first_val)];
                    
                    if self.current.kind == TokenKind::Comma {
                        self.advance();
                    }
                    
                    while self.current.kind != TokenKind::RightBracket && self.current.kind != TokenKind::EOF {
                        let k = self.parse_expression(Precedence::Lowest)?;
                        if self.current.kind != TokenKind::DoubleColon {
                             self.error("Expected '::' in map literal.");
                             return None;
                        }
                        self.advance();
                        let v = self.parse_expression(Precedence::Lowest)?;
                        elements.push((k, v));
                        if self.current.kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    
                    if self.current.kind == TokenKind::RightBracket {
                        self.advance(); // past ']'
                    }
                    
                    Some(Expr {
                        kind: ExprKind::MapLiteral {
                            key_type: Box::new(Type::String), 
                            value_type: Box::new(Type::Unknown),
                            elements,
                        },
                        span: lit_span,
                    })
                } else {
                    let mut elements = vec![first_expr];
                    if self.current.kind == TokenKind::Comma {
                        self.advance();
                    }
                    elements.extend(self.parse_array_or_set_literal_elements(TokenKind::RightBracket));
                    if self.current.kind == TokenKind::RightBracket {
                        self.advance(); // past ']'
                    }
                    Some(Expr {
                        kind: ExprKind::ArrayLiteral { elements },
                        span: lit_span,
                    })
                }
            }
            TokenKind::Random => {
                self.advance(); // past 'random'
                if self.current.kind != TokenKind::Dot { return None; }
                self.advance(); // past '.'
                
                let method: Option<String> = match self.current.kind {
                    TokenKind::Identifier(id) => Some(self.interner.lookup(id).to_string()),
                    TokenKind::Choice => Some("choice".to_string()),
                    TokenKind::TypeI => Some("int".to_string()),
                    TokenKind::TypeF => Some("float".to_string()),
                    _ => None,
                };
                
                if let Some(m) = method {
                    match m.as_str() {
                        "int" => {
                            self.advance(); // past 'int'
                            if self.current.kind != TokenKind::LeftParen { return None; }
                            self.advance(); // past '('
                            
                            let min = self.parse_expression(Precedence::Lowest)?;
                            if self.current.kind != TokenKind::Comma { return None; }
                            self.advance();
                            let max = self.parse_expression(Precedence::Lowest)?;
                            
                            let mut step = None;
                            if self.current.kind == TokenKind::AtStep {
                                self.advance();
                                let s = self.parse_expression(Precedence::Lowest)?;
                                step = Some(Box::new(s));
                            }
                            
                            if self.current.kind != TokenKind::RightParen { return None; }
                            self.advance();
                            
                            return Some(Expr {
                                kind: ExprKind::RandomInt {
                                    min: Box::new(min),
                                    max: Box::new(max),
                                    step,
                                },
                                span,
                            });
                        }
                        "float" => {
                            self.advance(); // past 'float'
                            if self.current.kind != TokenKind::LeftParen { return None; }
                            self.advance(); // past '('
                            
                            let min = self.parse_expression(Precedence::Lowest)?;
                            if self.current.kind != TokenKind::Comma { return None; }
                            self.advance();
                            let max = self.parse_expression(Precedence::Lowest)?;
                            
                            let mut step = None;
                            if self.current.kind == TokenKind::AtStep {
                                self.advance();
                                let s = self.parse_expression(Precedence::Lowest)?;
                                step = Some(Box::new(s));
                            }
                            
                            if self.current.kind != TokenKind::RightParen { return None; }
                            self.advance();
                            
                            return Some(Expr {
                                kind: ExprKind::RandomFloat {
                                    min: Box::new(min),
                                    max: Box::new(max),
                                    step,
                                },
                                span,
                            });
                        }
                        "choice" => {
                            self.advance(); // past 'choice'
                            if self.current.kind != TokenKind::From { return None; }
                            self.advance(); // past 'from'
                            
                            let set_expr = self.parse_expression(Precedence::Lowest)?;
                            
                            return Some(Expr {
                                kind: ExprKind::RandomChoice {
                                    set: Box::new(set_expr),
                                },
                                span,
                            });
                        }
                        _ => {}
                    }
                }
                
                // Fallback for module call random.something()
                let _ = self.parse_module_call(TokenKind::Random, span);
                None
            }
            TokenKind::Date => {
                let span = self.current.span.clone();
                if self.peek.kind == TokenKind::LeftParen {
                    self.advance(); // past 'date'
                    self.advance(); // past '('
                    let date_string = if let TokenKind::StringLiteral(id) = self.current.kind { id } else { return None; };
                    self.advance();
                    let format = if self.current.kind == TokenKind::Comma {
                        self.advance();
                        if let TokenKind::StringLiteral(fmt_id) = self.current.kind { self.advance(); Some(fmt_id) } else { return None; }
                    } else { None };
                    if self.current.kind != TokenKind::RightParen { return None; }
                    self.advance();
                    Some(Expr { kind: ExprKind::DateLiteral { date_string, format }, span })
                } else {
                    // Could be date.now()
                    self.parse_module_call(TokenKind::Date, span)
                }
            }
            TokenKind::Net | TokenKind::Json | TokenKind::Crypto | TokenKind::Store | TokenKind::Env | TokenKind::Halt | TokenKind::Perf => {
                let module = self.current.kind.clone();
                self.parse_module_call(module, span)
            }
            TokenKind::Map => self.parse_map_literal(),
            TokenKind::Table => self.parse_table_literal(),
            TokenKind::Database => self.parse_database_literal(),
            TokenKind::Yield => {
                self.advance(); // past 'yield'
                let expr = self.parse_expression(Precedence::Prefix)?;
                Some(Expr {
                    kind: ExprKind::Yield(Box::new(expr)),
                    span,
                })
            }
            TokenKind::Unknown(c) if *c == '@' => {
                self.error("Unexpected '@' - unrecognized prefix command.");
                return None;
            }
            TokenKind::Tag(id) => {
                let id = *id;
                self.advance();
                Some(Expr {
                    kind: ExprKind::Tag(id),
                    span,
                })
            }
            TokenKind::Dot => {
                if self.peek.kind == TokenKind::Terminal {
                    return self.parse_terminal_expression();
                }
                None
            }
            _ => None,
        }
    }

    // Handles named and positional arguments.
    pub(super) fn parse_arguments(&mut self) -> Vec<Argument> {
        let mut args = Vec::new();
        while self.current.kind != TokenKind::RightParen && self.current.kind != TokenKind::EOF {
            if matches!(self.current.kind, TokenKind::Identifier(_)) && self.peek.kind == TokenKind::Equal {
                let name = if let TokenKind::Identifier(id) = self.current.kind { id } else { unreachable!() };
                self.advance(); // past identifier
                self.advance(); // past '='
                if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                    args.push(Argument::Named(name, expr));
                } else {
                    break;
                }
            } else {
                if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                    args.push(Argument::Positional(expr));
                } else {
                    break;
                }
            }
            
            if self.current.kind == TokenKind::Comma {
                self.advance();
            }
        }
        args
    }

    // Handles infix operators based on precedence.
    pub(super) fn parse_infix(&mut self, left: Expr) -> Option<Expr> {
        let op = self.current.kind.clone();
        if op == TokenKind::Dot {
             return self.parse_dot_infix(left);
        }
        if op == TokenKind::LeftBracket {
             return self.parse_index_infix(left);
        }
        if op == TokenKind::Arrow {
             return self.parse_lambda_infix(left);
        }
        if op == TokenKind::As {
            let span = left.span.clone();
            self.advance(); // past 'as'
            
            let name = if self.current.kind == TokenKind::LeftParen {
                self.advance(); // past '('
                let id = if let TokenKind::Identifier(id) = self.current.kind {
                    id
                } else {
                    self.error("Expected identifier in as(...)");
                    return None;
                };
                self.advance(); // past identifier
                if self.current.kind != TokenKind::RightParen {
                    self.error("Expected ')' after identifier in as(...)");
                    return None;
                }
                self.advance(); // past ')'
                id
            } else if let TokenKind::Identifier(_) = self.current.kind {
                let id = self.parse_identifier_as_string_id(false).unwrap();
                id
            } else {
                self.error("Expected identifier after 'as'");
                return None;
            };
            
            return Some(Expr {
                kind: ExprKind::As { expr: Box::new(left), name },
                span,
            });
        }

        let span = left.span.clone();
        let precedence = self.current_precedence();
        
        let next_precedence = if op == TokenKind::Caret {
            match precedence {
                Precedence::Power => Precedence::Sum,
                p => p,
            }
        } else {
            precedence
        };

        self.advance();
        let right = self.parse_expression(next_precedence)?;
        Some(Expr {
            kind: ExprKind::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            },
            span,
        })
    }

    // Handles member access and method calls.
    fn parse_dot_infix(&mut self, receiver: Expr) -> Option<Expr> {
        let span = receiver.span.clone();
        self.advance(); // past '.'

        if self.current.kind == TokenKind::LeftBracket {
            self.advance(); // past '['
            let index = self.parse_expression(Precedence::Lowest)?;
            if self.current.kind != TokenKind::RightBracket { return None; }
            self.advance(); // past ']'
            return Some(Expr {
                kind: ExprKind::Index {
                    receiver: Box::new(receiver),
                    index: Box::new(index),
                },
                span,
            });
        }
        
        let member = if let Some(id) = self.parse_identifier_as_string_id(false) {
            id
        } else {
            return None;
        };
        
        let member_str = self.interner.lookup(member).to_string();
        if member_str == "choice" && self.current.kind == TokenKind::From {
            self.advance(); // past 'from'
            let set = self.parse_expression(Precedence::Lowest)?;
            return Some(Expr {
                kind: ExprKind::RandomChoice { set: Box::new(set) },
                span,
            });
        }
        
        if self.current.kind == TokenKind::LeftParen {
            self.advance(); // past '('
            let args = self.parse_arguments();
            if self.current.kind == TokenKind::RightParen {
                self.advance();
            }
            
            let mut wait_after = false;
            if self.current.kind == TokenKind::AtWait {
                self.advance(); // past '@wait'
                wait_after = true;
            }
            
            Some(Expr {
                kind: ExprKind::MethodCall {
                    receiver: Box::new(receiver),
                    method: member,
                    args,
                    wait_after,
                },
                span,
            })
        } else {
            Some(Expr {
                kind: ExprKind::MemberAccess {
                    receiver: Box::new(receiver),
                    member,
                },
                span,
            })
        }
    }

    // Handles array-style indexing.
    fn parse_index_infix(&mut self, receiver: Expr) -> Option<Expr> {
        let span = receiver.span.clone();
        self.advance(); // past '['
        let index = self.parse_expression(Precedence::Lowest)?;
        
        if self.current.kind != TokenKind::RightBracket { return None; }
        self.advance(); // past ']'
        Some(Expr {
            kind: ExprKind::Index {
                receiver: Box::new(receiver),
                index: Box::new(index),
            },
            span,
        })
    }

    // Collects elements for array or set literals.
    fn parse_array_or_set_literal_elements(&mut self, end_kind: TokenKind) -> Vec<Expr> {
        let mut elements = Vec::new();
        while self.current.kind != end_kind && self.current.kind != TokenKind::EOF {
            if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                elements.push(expr);
            } else {
                break;
            }
            if self.current.kind == TokenKind::Comma {
                self.advance();
            }
        }
        elements
    }

    // Handles the content of set literals, including ranges.
    fn parse_set_literal_content(&mut self, st: SetType, lit_span: Span) -> Option<Expr> {
        let mut elements = Vec::new();
        let mut is_range = false;
        let mut range = None;
        
        if self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
            let first_expr = self.parse_expression(Precedence::Lowest)?;
            
            if self.current.kind == TokenKind::DoubleComma {
                is_range = true;
                self.advance();
                let end_expr = self.parse_expression(Precedence::Lowest)?;
                
                let mut step_expr = None;
                if self.current.kind == TokenKind::AtStep {
                    self.advance();
                    let s_expr = self.parse_expression(Precedence::Lowest)?;
                    
                    step_expr = Some(Box::new(s_expr));
                }
                
                range = Some(SetRange {
                    start: Box::new(first_expr),
                    end: Box::new(end_expr),
                    step: step_expr,
                });
            } else {
                elements.push(first_expr);
                if self.current.kind == TokenKind::Comma {
                    self.advance();
                }
                while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
                    if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                        elements.push(expr);
                    }
                    if self.current.kind == TokenKind::Comma {
                        self.advance();
                    }
                }
            }
        }
        
        if is_range {
            Some(Expr {
                kind: ExprKind::SetLiteral {
                    set_type: st,
                    elements: Vec::new(),
                    range,
                },
                span: lit_span,
            })
        } else {
            Some(Expr {
                kind: ExprKind::SetLiteral {
                    set_type: st,
                    elements,
                    range: None,
                },
                span: lit_span,
            })
        }
    }

    // Handles map literals with key-value pairs.
    pub(super) fn parse_map_literal(&mut self) -> Option<Expr> {
        let span = self.current.span.clone();
        if self.current.kind == TokenKind::Map {
            self.advance();
        }
        if self.current.kind != TokenKind::LeftBrace { return None; }
        self.advance(); // past '{'
        
        if self.current.kind != TokenKind::Schema { return None; }
        self.advance();
        if self.current.kind != TokenKind::Equal { return None; }
        self.advance();
        if self.current.kind != TokenKind::LeftBracket { return None; }
        self.advance();
        
        let k_ty = match self.current.kind {
            TokenKind::TypeI => Type::Int,
            TokenKind::TypeF => Type::Float,
            TokenKind::TypeS => Type::String,
            TokenKind::TypeB => Type::Bool,
            _ => return None,
        };
        self.advance();
        if self.current.kind != TokenKind::Bridge { return None; }
        self.advance();
        let v_ty = match self.current.kind {
            TokenKind::TypeI => Type::Int,
            TokenKind::TypeF => Type::Float,
            TokenKind::TypeS => Type::String,
            TokenKind::TypeB => Type::Bool,
            _ => return None,
        };
        self.advance();
        if self.current.kind != TokenKind::RightBracket { return None; }
        self.advance();
        
        if self.current.kind != TokenKind::Data { return None; }
        self.advance();
        if self.current.kind != TokenKind::Equal { return None; }
        self.advance();
        if self.current.kind != TokenKind::LeftBracket { return None; }
        self.advance();
        
        let mut elements = Vec::new();
        if self.current.kind == TokenKind::Empty {
            self.advance();
            if self.current.kind != TokenKind::RightBracket { return None; }
            self.advance();
        } else {
            while self.current.kind != TokenKind::RightBracket && self.current.kind != TokenKind::EOF {
                let key_expr = self.parse_expression(Precedence::Concatenation)?;
                
                if self.current.kind != TokenKind::DoubleColon { return None; }
                self.advance();
                
                let val_expr = self.parse_expression(Precedence::Lowest)?;
                elements.push((key_expr, val_expr));
                
                if self.current.kind == TokenKind::Comma {
                    self.advance();
                }
            }
            if self.current.kind == TokenKind::RightBracket {
                self.advance();
            }
        }
        
        if self.current.kind != TokenKind::RightBrace { return None; }
        
        Some(Expr {
            kind: ExprKind::MapLiteral {
                key_type: Box::new(k_ty),
                value_type: Box::new(v_ty),
                elements,
            },
            span,
        })
    }

    // Handles table literals with schema and rows.
    pub(super) fn parse_table_literal(&mut self) -> Option<Expr> {
        let span = self.current.span.clone();
        if self.current.kind == TokenKind::Table {
            self.advance();
        }
        if self.current.kind != TokenKind::LeftBrace { return None; }
        self.advance(); // past '{'

        if self.current.kind != TokenKind::Columns { return None; }
        self.advance();
        if self.current.kind != TokenKind::Colon && self.current.kind != TokenKind::Equal { return None; }
        self.advance();
        if self.current.kind != TokenKind::LeftBracket { return None; }
        self.advance();

        let mut columns = Vec::new();
        while self.current.kind != TokenKind::RightBracket && self.current.kind != TokenKind::EOF {
            let col_name = if let Some(id) = self.parse_identifier_as_string_id(false) {
                id
            } else {
                return None;
            };
            if self.current.kind != TokenKind::DoubleColon { return None; }
            self.advance();

            let col_ty = self.parse_type()?;
            
            let mut attributes = Vec::new();
            while matches!(self.current.kind, TokenKind::AtAuto | TokenKind::AtPk | TokenKind::AtUnique | TokenKind::AtOptional | TokenKind::AtDefault | TokenKind::AtFk) {
                match self.current.kind {
                    TokenKind::AtAuto => { attributes.push(crate::frontend::ast::table::ColumnAttribute::Auto); self.advance(); }
                    TokenKind::AtPk => { attributes.push(crate::frontend::ast::table::ColumnAttribute::PrimaryKey); self.advance(); }
                    TokenKind::AtUnique => { attributes.push(crate::frontend::ast::table::ColumnAttribute::Unique); self.advance(); }
                    TokenKind::AtOptional => { attributes.push(crate::frontend::ast::table::ColumnAttribute::Optional); self.advance(); }
                    TokenKind::AtDefault => {
                        self.advance(); // past '@default'
                        if self.current.kind != TokenKind::LeftParen { break; }
                        self.advance(); // past '('
                        let val = self.parse_expression(Precedence::Lowest)?;
                        if self.current.kind == TokenKind::RightParen { self.advance(); }
                        attributes.push(crate::frontend::ast::table::ColumnAttribute::Default(val));
                    }
                    TokenKind::AtFk => {
                        self.advance(); // past '@fk'
                        if self.current.kind != TokenKind::LeftParen { break; }
                        self.advance(); // past '('
                        let tbl = if let Some(id) = self.parse_identifier_as_string_id(false) { id } else { break; };
                        if self.current.kind != TokenKind::Dot { break; }
                        self.advance(); // past '.'
                        let col = if let Some(id) = self.parse_identifier_as_string_id(false) { id } else { break; };
                        if self.current.kind == TokenKind::RightParen { self.advance(); }
                        attributes.push(crate::frontend::ast::table::ColumnAttribute::ForeignKey(tbl, col));
                    }
                    _ => break,
                }
            }

            columns.push(crate::frontend::ast::table::ColumnDef {
                name: col_name,
                ty: col_ty,
                attributes,
            });

            if self.current.kind == TokenKind::Comma {
                self.advance();
            }
        }
        if self.current.kind == TokenKind::RightBracket {
            self.advance();
        }
        
        if self.current.kind == TokenKind::Comma {
            self.advance();
        }

        if self.current.kind != TokenKind::Rows { return None; }
        self.advance();
        if self.current.kind != TokenKind::Colon && self.current.kind != TokenKind::Equal { return None; }
        self.advance();
        if self.current.kind != TokenKind::LeftBracket { return None; }
        self.advance();

        let mut rows = Vec::new();
        if self.current.kind == TokenKind::Empty {
            self.advance();
        } else {
        while self.current.kind != TokenKind::RightBracket && self.current.kind != TokenKind::EOF {
            let mut row_vals = Vec::new();
            if self.current.kind == TokenKind::LeftParen || self.current.kind == TokenKind::LeftBracket {
                let close_kind = if self.current.kind == TokenKind::LeftParen { TokenKind::RightParen } else { TokenKind::RightBracket };
                self.advance();
                
                while self.current.kind != close_kind && self.current.kind != TokenKind::EOF {
                    if let Some(val) = self.parse_expression(Precedence::Lowest) {
                        row_vals.push(val);
                    }
                    if self.current.kind == TokenKind::Comma {
                        self.advance();
                    }
                }
                if self.current.kind == close_kind {
                    self.advance();
                }
            }
            
            rows.push(row_vals);
            
            if self.current.kind == TokenKind::Comma {
                self.advance();
            }
        }
        }
        
        if self.current.kind == TokenKind::RightBracket {
            self.advance();
        }

        Some(Expr {
            kind: ExprKind::TableLiteral { columns, rows },
            span,
        })
    }

    // Handles database literals.
    pub(super) fn parse_database_literal(&mut self) -> Option<Expr> {
        let span = self.current.span.clone();
        if self.current.kind == TokenKind::Database {
            self.advance();
        }
        if self.current.kind != TokenKind::LeftBrace { return None; }
        self.advance(); // past '{'

        let mut fields = Vec::new();
        while self.current.kind != TokenKind::RightBrace && self.current.kind != TokenKind::EOF {
             let field = if let Some(id) = self.parse_identifier_as_string_id(false) { id } else { break; };
             if self.current.kind != TokenKind::Equal && self.current.kind != TokenKind::Colon { break; }
             self.advance();
             let val = self.parse_expression(Precedence::Lowest)?;
             fields.push((field, val));

             if self.current.kind == TokenKind::Comma {
                 self.advance();
             }
        }
        
        Some(Expr {
            kind: ExprKind::DatabaseLiteral(fields),
            span,
        })
    }

    // Handles lambda (anonymous function) expressions.
    fn parse_lambda_infix(&mut self, left: Expr) -> Option<Expr> {
        let span = left.span.clone();
        self.advance(); // past '->'
        
        let params: Vec<(Type, StringId)> = match left.kind {
            ExprKind::Identifier(id) => {
                vec![(Type::Unknown, id)]
            }
            ExprKind::Tuple(exprs) => {
                let mut ids = Vec::new();
                for e in exprs {
                    if let ExprKind::Identifier(id) = e.kind {
                        ids.push((Type::Unknown, id));
                    } else {
                        return None;
                    }
                }
                ids
            }
            _ => return None,
        };
    
        let body = self.parse_expression(Precedence::Lowest)?;
    
        Some(Expr {
            kind: ExprKind::Lambda {
                params,
                return_type: None,
                body: Box::new(body),
            },
            span,
        })
    }

    // Handles .terminal command expressions.
    pub(super) fn parse_terminal_expression(&mut self) -> Option<Expr> {
        let span = self.current.span.clone();
        self.advance(); // past '.'
        self.advance(); // past 'terminal'
        
        if self.current.kind == TokenKind::Bang {
            self.advance(); // past '!'
        }

        if let Some(cmd_id) = self.parse_identifier_as_string_id(false) {
            let cmd_str = self.interner.lookup(cmd_id).to_string();

            let cmd_id_u = cmd_id;
            
            if cmd_str == "write" {
                if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                    return Some(Expr {
                        kind: ExprKind::TerminalCommand(cmd_id_u, vec![expr]),
                        span,
                    });
                }
            } else {
                let mut args = Vec::new();
                if self.current.kind != TokenKind::Semicolon && self.current.kind != TokenKind::EOF && self.current.kind != TokenKind::RightParen && self.current.kind != TokenKind::RightBracket {
                    while self.current.kind != TokenKind::Semicolon && self.current.kind != TokenKind::EOF && self.current.kind != TokenKind::RightParen && self.current.kind != TokenKind::RightBracket {
                        if let Some(expr) = self.parse_expression(Precedence::Lowest) {
                            args.push(expr);
                        }
                        if self.current.kind == TokenKind::Comma {
                            self.advance(); 
                        } else if self.current.kind != TokenKind::Semicolon && self.current.kind != TokenKind::EOF && self.current.kind != TokenKind::RightParen && self.current.kind != TokenKind::RightBracket {
                            // Logic here might need to be careful not to consume semicolon if it was already advanced by parse_expression
                        } else {
                            break; 
                        }
                    }
                }

                return Some(Expr {
                    kind: ExprKind::TerminalCommand(cmd_id_u, args),
                    span,
                });
            }
        }
        None
    }

    fn parse_module_call(&mut self, module: TokenKind, span: Span) -> Option<Expr> {
        self.advance(); // past module keyword
        if self.current.kind != TokenKind::Dot { 
            self.error(&format!("Expected '.' after '{:?}'.", module));
            return None; 
        }
        self.advance(); // past '.'
        
        let method_id = if let TokenKind::Identifier(id) = self.current.kind { id } else {
            self.error("Expected method name after module prefix.");
            return None;
        };
        self.advance(); // past method name

        let mut args = Vec::new();
        if self.current.kind == TokenKind::LeftParen {
            self.advance(); // past '('
            while self.current.kind != TokenKind::RightParen && self.current.kind != TokenKind::EOF {
                if let Some(arg_expr) = self.parse_expression(Precedence::Lowest) {
                    args.push(crate::frontend::ast::Argument::Positional(arg_expr));
                } else {
                    return None;
                }
                if self.current.kind == TokenKind::Comma {
                    self.advance();
                } else if self.current.kind != TokenKind::RightParen {
                    self.error("Expected ',' or ')' in module call.");
                    return None;
                }
            }
            if self.current.kind != TokenKind::RightParen {
                self.error("Expected ')' at end of module call.");
                return None;
            }
            self.advance(); // past ')'
        }

        Some(Expr {
            kind: ExprKind::ModuleCall {
                module,
                method: method_id,
                args,
            },
            span,
        })
    }
}
