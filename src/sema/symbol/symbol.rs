use crate::sema::types::Type;

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Constant,
    Function,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub ty: Type,
    pub kind: SymbolKind,
}

impl Symbol {
    pub fn new(ty: Type, kind: SymbolKind) -> Self {
        Self { ty, kind }
    }

    pub fn is_const(&self) -> bool {
        matches!(self.kind, SymbolKind::Constant)
    }
}
