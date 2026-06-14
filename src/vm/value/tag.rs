pub use super::nan_boxing::{
    TAG_FLOAT, TAG_INT, TAG_BOOL, TAG_DATE, TAG_STR, TAG_ARR, TAG_SET,
    TAG_MAP, TAG_TBL, TAG_FUNC, TAG_ROW, TAG_JSON, TAG_FIB, TAG_DB,
    TAG_CLOSURE, TAG_ARENA, TAG_FIRST_PTR,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Float, Int, Bool, Date, String, Array, Set, Map, Table, Function, Row, Json, Fiber, Database, Unknown
}

impl Tag {
    pub fn name(&self) -> &'static str {
        match self {
            Tag::Float    => "float",
            Tag::Int      => "int",
            Tag::Bool     => "bool",
            Tag::Date     => "date",
            Tag::String   => "string",
            Tag::Array    => "array",
            Tag::Set      => "set",
            Tag::Map      => "map",
            Tag::Table    => "table",
            Tag::Function => "function",
            Tag::Row      => "row",
            Tag::Json     => "json",
            Tag::Fiber    => "fiber",
            Tag::Database => "database",
            Tag::Unknown  => "unknown",
        }
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
