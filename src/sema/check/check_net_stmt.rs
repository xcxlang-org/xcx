use crate::frontend::ast::{Expr, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::intern::StringId;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_net_request_stmt(
        &mut self,
        method: &mut Expr,
        url: &mut Expr,
        headers: &mut Option<Box<Expr>>,
        body: &mut Option<Box<Expr>>,
        timeout: &mut Option<Box<Expr>>,
        target: StringId,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        self.check_expr(method, symbols, errors);
        self.check_expr(url, symbols, errors);
        if let Some(h) = headers { self.check_expr(h, symbols, errors); }
        if let Some(b) = body { self.check_expr(b, symbols, errors); }
        if let Some(t) = timeout { self.check_expr(t, symbols, errors); }
        let name_str = self.interner.lookup(target).trim().to_string();
        symbols.define(name_str, Type::Json, false);
    }

    pub(crate) fn check_serve(
        &mut self,
        port: &mut Expr,
        host: &mut Option<Box<Expr>>,
        workers: &mut Option<Box<Expr>>,
        routes: &mut Expr,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        self.check_expr(port, symbols, errors);
        if let Some(h) = host { self.check_expr(h, symbols, errors); }
        if let Some(w) = workers { self.check_expr(w, symbols, errors); }
        self.check_routes_expr(routes, symbols, errors);
    }
}
