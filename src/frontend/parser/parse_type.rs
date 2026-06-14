use crate::frontend::lexer::TokenKind;
use crate::frontend::ast::{Type, SetType};
use super::parser::Parser;

impl<'a> Parser<'a> {
    pub fn parse_type(&mut self) -> Option<Type> {
        let mut is_array = false;
        if self.current.kind == TokenKind::Array {
            is_array = true;
            self.advance();
            if self.current.kind != TokenKind::Colon { return None; }
            self.advance();
        }
        let ty = match self.current.kind {
            TokenKind::TypeI => { self.advance(); Type::Int },
            TokenKind::TypeF => { self.advance(); Type::Float },
            TokenKind::TypeS => { self.advance(); Type::String },
            TokenKind::TypeB => { self.advance(); Type::Bool },
            TokenKind::Date => { self.advance(); Type::Date },
            TokenKind::Json => { self.advance(); Type::Json },
            TokenKind::Set => {
                self.advance();
                if self.current.kind == TokenKind::Colon {
                    self.advance();
                    let st = match self.current.kind {
                        TokenKind::TypeSetN => SetType::N,
                        TokenKind::TypeSetQ => SetType::Q,
                        TokenKind::TypeSetZ => SetType::Z,
                        TokenKind::TypeSetS => SetType::S,
                        TokenKind::TypeSetB => SetType::B,
                        TokenKind::TypeSetC => SetType::C,
                        _ => return None,
                    };
                    self.advance();
                    Type::Set(st)
                } else {
                    Type::Set(SetType::N)
                }
            }
            TokenKind::Map => {
                self.advance(); 
                if self.current.kind == TokenKind::Colon && self.is_type_intro(&self.peek.kind) {
                    self.advance(); 
                    let k_ty = self.parse_type()?;
                    if self.current.kind == TokenKind::Bridge {
                        self.advance(); 
                        let v_ty = self.parse_type()?;
                        Type::Map(Box::new(k_ty), Box::new(v_ty))
                    } else {
                        Type::Map(Box::new(k_ty), Box::new(Type::Int))
                    }
                } else {
                    Type::Map(Box::new(Type::Int), Box::new(Type::Int))
                }
            }
            TokenKind::Table => { self.advance(); Type::Table(crate::sema::types::TableType::empty()) }
            TokenKind::Database => { self.advance(); Type::Database }
            TokenKind::Fiber => {
                self.advance();
                let inner = if self.current.kind == TokenKind::Colon {
                    self.advance();
                    self.parse_type().map(Box::new)
                } else { None };
                Type::Fiber(inner)
            }
            _ => return None,
        };
        if is_array { Some(Type::Array(Box::new(ty))) } else { Some(ty) }
    }

    pub(super) fn is_type_intro(&self, kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::TypeI | TokenKind::TypeF | TokenKind::TypeS | TokenKind::TypeB | 
                 TokenKind::Date | TokenKind::Json | TokenKind::Set | TokenKind::Map | 
                 TokenKind::Table | TokenKind::Database | TokenKind::Fiber | TokenKind::Array)
    }
}
