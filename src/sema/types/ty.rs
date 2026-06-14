use crate::intern::StringId;

#[derive(Debug, Clone, PartialEq)]
pub enum SetType {
    N, // Natural
    Q, // Rational
    Z, // Integers
    S, // Strings
    B, // Booleans
    C, // Chars
}

impl std::fmt::Display for SetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetType::N => write!(f, "N"),
            SetType::Q => write!(f, "Q"),
            SetType::Z => write!(f, "Z"),
            SetType::S => write!(f, "S"),
            SetType::B => write!(f, "B"),
            SetType::C => write!(f, "C"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatabaseOpKind {
    Remove,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Array(Box<Type>),
    Set(SetType),
    Map(Box<Type>, Box<Type>),
    Date,
    Table(super::table_type::TableType),
    Database,
    DatabaseOperation(DatabaseOpKind, super::table_type::TableType),


    Json,
    Builtin(StringId),
    Fiber(Option<Box<Type>>),
    Unknown,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::String => write!(f, "str"),
            Type::Bool => write!(f, "bool"),
            Type::Array(inner) => write!(f, "array:{}", inner),
            Type::Set(st) => write!(f, "set:{}", st),
            Type::Map(k, v) => write!(f, "map:{}<->{}", k, v),
            Type::Date => write!(f, "date"),
            Type::Table(_) => write!(f, "table"),
            Type::Database => write!(f, "database"),
            Type::DatabaseOperation(kind, _) => write!(f, "database_op:{:?}", kind),
            Type::Json => write!(f, "json"),
            Type::Builtin(_) => write!(f, "builtin"),
            Type::Fiber(inner) => {
                if let Some(t) = inner {
                    write!(f, "fiber:{}", t)
                } else {
                    write!(f, "fiber")
                }
            }
            Type::Unknown => write!(f, "unknown"),
        }
    }
}
