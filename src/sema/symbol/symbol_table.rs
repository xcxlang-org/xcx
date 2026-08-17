use crate::sema::types::Type;
use super::symbol::{Symbol, SymbolKind};
use super::scope::Scope;

#[derive(Clone)]
/// Hierarchical symbol table for tracking variable types and constant status.
/// Supports nested scopes and parent-linkage for cross-context resolution
/// during semantic analysis and code generation.
pub struct SymbolTable<'a> {
    parent: Option<&'a SymbolTable<'a>>,
    scopes: Vec<Scope>,
}

impl<'a> SymbolTable<'a> {
    pub fn new() -> Self {
        Self {
            parent: None,
            scopes: vec![Scope::new()],
        }
    }

    pub fn new_with_parent(parent: &'a SymbolTable<'a>) -> Self {
        Self {
            parent: Some(parent),
            scopes: vec![Scope::new()],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn has(&self, name: &str) -> bool {
        let name = name.trim();
        let mut curr = Some(self);
        while let Some(st) = curr {
            for scope in st.scopes.iter().rev() {
                if scope.contains(name) {
                    return true;
                }
            }
            curr = st.parent;
        }
        false
    }

    pub fn has_in_current_scope(&self, name: &str) -> bool {
        let name = name.trim();
        self.scopes.last().map(|s| s.contains(name)).unwrap_or(false)
    }

    pub fn define(&mut self, name: String, ty: Type, is_const: bool) {
        let name = name.trim().to_string();
        let kind = if is_const { SymbolKind::Constant } else { SymbolKind::Variable };
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, Symbol::new(ty, kind));
        }
    }

    pub fn lookup(&self, name: &str) -> Option<Type> {
        let name = name.trim();
        let mut curr = Some(self);
        while let Some(st) = curr {
            for scope in st.scopes.iter().rev() {
                if let Some(symbol) = scope.lookup(name) {
                    return Some(symbol.ty.clone());
                }
            }
            curr = st.parent;
        }
        None
    }

    pub fn is_const(&self, name: &str) -> bool {
        let name = name.trim();
        let mut curr = Some(self);
        while let Some(st) = curr {
            for scope in st.scopes.iter().rev() {
                if let Some(symbol) = scope.lookup(name) {
                    return symbol.is_const();
                }
            }
            curr = st.parent;
        }
        false
    }
}
