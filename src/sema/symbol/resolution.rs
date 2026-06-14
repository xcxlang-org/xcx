use crate::frontend::ast::{Stmt, StmtKind, Type};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::check::checker::{Checker, FunctionSignature};

impl<'a> Checker<'a> {
    pub(crate) fn pre_scan_stmts(&mut self, stmts: &[Stmt], symbols: &mut SymbolTable<'_>) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::FunctionDef { name, params, return_type, .. } => {
                    let name_str = self.interner.lookup(*name).trim().to_string();
                    let param_types = params.iter().map(|(ty, _)| ty.clone()).collect();
                    let sig = FunctionSignature {
                        params: param_types,
                        return_type: return_type.as_ref().map(|t| *t.clone()),
                        is_fiber: false,
                    };
                    self.functions.insert(name_str.clone(), sig);
                    symbols.define(name_str, Type::Unknown, false);
                }
                StmtKind::FiberDef { name, params, return_type, .. } => {
                    let name_str = self.interner.lookup(*name).trim().to_string();
                    let param_types = params.iter().map(|(ty, _)| ty.clone()).collect();
                    let sig = FunctionSignature {
                        params: param_types,
                        return_type: return_type.as_ref().map(|t| *t.clone()),
                        is_fiber: true,
                    };
                    self.functions.insert(name_str.clone(), sig);
                    let var_type = Type::Fiber(return_type.clone());
                    symbols.define(name_str, var_type, false);
                }
                _ => {}
            }
        }
    }
}
