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

/// Literal value of a `@default(v)` column attribute. Only the literal forms
/// are accepted; string defaults keep the interned id and are resolved when
/// the SQL literal is rendered.
#[derive(Debug, Clone, PartialEq)]
pub enum DefaultConst {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(StringId),
}

impl DefaultConst {
    /// Renders the value as a SQLite literal (strings are single-quoted and
    /// escaped); `lookup` resolves interned string ids.
    pub fn to_sql_literal(&self, lookup: impl Fn(StringId) -> String) -> String {
        match self {
            DefaultConst::Int(n) => n.to_string(),
            DefaultConst::Float(v) => v.to_string(),
            DefaultConst::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
            DefaultConst::Str(id) => format!("'{}'", lookup(*id).replace('\'', "''")),
        }
    }
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

    // Returns the constant value of the column's @default attribute, if any.
    pub fn default_const(&self) -> Option<DefaultConst> {
        self.attributes.iter().find_map(|a| match a {
            ColumnAttribute::Default(expr) => match &expr.kind {
                crate::frontend::ast::expr::ExprKind::IntLiteral(n) => Some(DefaultConst::Int(*n)),
                crate::frontend::ast::expr::ExprKind::FloatLiteral(v) => Some(DefaultConst::Float(*v)),
                crate::frontend::ast::expr::ExprKind::BoolLiteral(b) => Some(DefaultConst::Bool(*b)),
                crate::frontend::ast::expr::ExprKind::StringLiteral(id) => Some(DefaultConst::Str(*id)),
                _ => None,
            },
            _ => None,
        })
    }
}
