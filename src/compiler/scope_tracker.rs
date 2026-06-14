use std::collections::HashMap;
use crate::intern::StringId;
use crate::compiler::compiler::FunctionCompiler;

impl FunctionCompiler {
    pub fn enter_scope(&mut self) -> usize {
        self.scopes.push(HashMap::new());
        self.next_local
    }

    pub fn exit_scope(&mut self, next_local: usize) {
        self.scopes.pop();
        self.next_local = next_local;
    }

    pub fn lookup_local(&mut self, id: &StringId) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(id) {
                return Some(slot);
            }
        }
        
        if let Some(parent) = &self.parent_locals {
            if parent.contains_key(id) {
                if let Some(pos) = self.captures.iter().position(|c| c == id) {
                    return Some(1 + pos);
                } else {
                    let pos = self.captures.len();
                    self.captures.push(*id);
                    let slot = 1 + pos;
                    self.scopes[0].insert(*id, slot);
                    if slot >= self.next_local { self.next_local = slot + 1; }
                    return Some(slot);
                }
            }
        }
        None
    }

    pub fn define_local(&mut self, id: StringId, slot: usize) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(id, slot);
        }
    }

    pub fn convert_to_flat_locals(&self) -> HashMap<StringId, usize> {
        let mut flat = HashMap::new();
        for scope in &self.scopes {
            for (&id, &slot) in scope {
                flat.insert(id, slot);
            }
        }
        flat
    }
}
