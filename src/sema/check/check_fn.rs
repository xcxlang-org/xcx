use crate::frontend::ast::{Stmt, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::intern::StringId;
use super::checker::Checker;

impl<'a> Checker<'a> {
    pub(crate) fn check_func_def(
        &mut self,
        name: &StringId,
        params: &mut Vec<(Type, StringId)>,
        return_type: &mut Option<Box<Type>>,
        body: &mut Vec<Stmt>,
        symbols: &mut SymbolTable<'_>,
        errors: &mut Vec<TypeError>,
    ) {
        let name_str = self.interner.lookup(*name).trim().to_string();
        if !symbols.has(&name_str) {
            symbols.define(name_str.clone(), Type::Unknown, false);
        }

        let mut func_symbols = SymbolTable::new_with_parent(symbols);
        let prev_ctx = self.fiber_context.take();
        let prev_is_fib = self.is_fiber_context;
        
        self.fiber_context = Some(return_type.as_ref().map(|t| *t.clone()));
        self.is_fiber_context = false;
        
        func_symbols.enter_scope();
        func_symbols.define(name_str, Type::Unknown, false);

        for (ty, param_name) in params {
            let p_name_str = self.interner.lookup(*param_name).trim().to_string();
            func_symbols.define(p_name_str, ty.clone(), false);
        }
        for s in body {
            self.check_stmt(s, &mut func_symbols, errors);
        }
        self.fiber_context = prev_ctx;
        self.is_fiber_context = prev_is_fib;
    }
}
