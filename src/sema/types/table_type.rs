use crate::intern::StringId;
use super::ty::Type;
use crate::frontend::ast::ColumnAttribute;

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnType {
    pub name: StringId,
    pub ty: Type,
    pub is_auto: bool,
    pub is_pk: bool,
    pub is_optional: bool,
    pub has_default: bool,
    pub is_unique: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableType {
    pub columns: Vec<ColumnType>,
}

impl From<Vec<crate::frontend::ast::ColumnDef>> for TableType {
    fn from(cols: Vec<crate::frontend::ast::ColumnDef>) -> Self {
        Self {
            columns: cols.into_iter().map(|c| {
                let name = c.name;
                let is_auto = c.is_auto();
                let is_pk = c.is_pk();
                let is_unique = c.is_unique();
                let is_optional = c.attributes.iter().any(|a| matches!(a, ColumnAttribute::Optional));
                let has_default = c.attributes.iter().any(|a| matches!(a, ColumnAttribute::Default(_)));
                let ty = c.ty;
                ColumnType { name, ty, is_auto, is_pk, is_optional, has_default, is_unique }
            }).collect()
        }
    }
}

impl TableType {
    pub fn new(columns: Vec<ColumnType>) -> Self {
        Self { columns }
    }
    
    pub fn empty() -> Self {
        Self { columns: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ColumnType> {
        self.columns.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, ColumnType> {
        self.columns.iter_mut()
    }

    pub fn push(&mut self, col: ColumnType) {
        self.columns.push(col);
    }
}

impl<'a> IntoIterator for &'a TableType {
    type Item = &'a ColumnType;
    type IntoIter = std::slice::Iter<'a, ColumnType>;
    fn into_iter(self) -> Self::IntoIter {
        self.columns.iter()
    }
}

impl IntoIterator for TableType {
    type Item = ColumnType;
    type IntoIter = std::vec::IntoIter<ColumnType>;
    fn into_iter(self) -> Self::IntoIter {
        self.columns.into_iter()
    }
}
