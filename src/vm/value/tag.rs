pub const TAG_FLOAT:   u64 = 0;
pub const TAG_INT:     u64 = 1;
pub const TAG_BOOL:    u64 = 2;
pub const TAG_DATE:    u64 = 3;
pub const TAG_STR:     u64 = 4;
pub const TAG_ARR:     u64 = 5;
pub const TAG_SET:     u64 = 6;
pub const TAG_MAP:     u64 = 7;
pub const TAG_TBL:     u64 = 8;
pub const TAG_FUNC:    u64 = 9;
pub const TAG_ROW:     u64 = 10;
pub const TAG_JSON:    u64 = 11;
pub const TAG_FIB:     u64 = 12;
pub const TAG_DB:      u64 = 13;
pub const TAG_CLOSURE: u64 = 14;
pub const TAG_ARENA:   u64 = 15;
pub const TAG_FUNC_PTR: u64 = 16;
pub const TAG_BOOL_ARR: u64 = 17;

pub const TAG_FIRST_PTR: u64 = TAG_STR;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Float, Int, Bool, Date, String, Array, Set, Map, Table, Function, Row, Json, Fiber, Database, BoolArray, Unknown
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
            Tag::BoolArray => "bool_array",
            Tag::Unknown  => "unknown",
        }
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
