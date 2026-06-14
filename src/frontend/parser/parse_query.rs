use super::parser::Parser;
use crate::frontend::ast::Expr;

impl<'a> Parser<'a> {
    pub fn parse_query_expr(&mut self) -> Option<Expr> {
        None
    }
}
