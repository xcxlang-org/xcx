use crate::intern::StringId;
use super::expr::Expr;
use crate::sema::types::Type;

// Attributes that can be applied to a table column.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnAttribute {
    Auto,
    PrimaryKey,
    Unique,
    Optional,
    Default(Expr),
    ForeignKey(StringId, StringId), // Table, Column
}

// Definition of a single column in a relational table.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: StringId,
    pub ty: Type,
    pub attributes: Vec<ColumnAttribute>,
}

impl ColumnDef {
    // Returns true if the column has the @auto attribute.
    pub fn is_auto(&self) -> bool {
        self.attributes.iter().any(|a| matches!(a, ColumnAttribute::Auto))
    }

    // Returns true if the column is part of the primary key.
    pub fn is_pk(&self) -> bool {
        self.attributes.iter().any(|a| matches!(a, ColumnAttribute::PrimaryKey))
    }

    // Returns true if the column has the @unique attribute.
    pub fn is_unique(&self) -> bool {
        self.attributes.iter().any(|a| matches!(a, ColumnAttribute::Unique))
    }
}
