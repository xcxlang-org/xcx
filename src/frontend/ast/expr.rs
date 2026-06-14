use crate::intern::StringId;
use crate::error::Span;
use crate::frontend::lexer::TokenKind;
use super::ty::{Type, SetType};
use super::argument::Argument;

// Represents a range for set literals (e.g., 1,,10 @step 2).
#[derive(Debug, Clone, PartialEq)]
pub struct SetRange {
    pub start: Box<Expr>,
    pub end: Box<Expr>,
    pub step: Option<Box<Expr>>,
}

// Represents a single expression in the XCX AST.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

// Enumerates all types of expressions in the XCX language.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(StringId),
    BoolLiteral(bool),
    Identifier(StringId),
    RawBlock(StringId),
    ArrayLiteral {
        elements: Vec<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: TokenKind,
        right: Box<Expr>,
    },
    Unary {
        op: TokenKind,
        right: Box<Expr>,
    },
    FunctionCall {
        name: StringId,
        args: Vec<Argument>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: StringId,
        args: Vec<Argument>,
        wait_after: bool,
    },
    SetLiteral {
        set_type: SetType,
        elements: Vec<Expr>,
        range: Option<SetRange>,
    },
    ArrayOrSetLiteral {
        elements: Vec<Expr>,
    },
    RandomChoice {
        set: Box<Expr>,
    },
    RandomInt {
        min: Box<Expr>,
        max: Box<Expr>,
        step: Option<Box<Expr>>,
    },
    RandomFloat {
        min: Box<Expr>,
        max: Box<Expr>,
        step: Option<Box<Expr>>,
    },
    MapLiteral {
        key_type: Box<Type>,
        value_type: Box<Type>,
        elements: Vec<(Expr, Expr)>,
    },
    DateLiteral {
        date_string: StringId,
        format: Option<StringId>,
    },
    TableLiteral {
        columns: Vec<super::table::ColumnDef>,
        rows: Vec<Vec<Expr>>,
    },
    DatabaseLiteral(Vec<(StringId, Expr)>),
    Index {
        receiver: Box<Expr>,
        index: Box<Expr>,
    },
    MemberAccess {
        receiver: Box<Expr>,
        member: StringId,
    },
    TerminalCommand(StringId, Vec<Expr>),
    Lambda {
        params: Vec<(Type, StringId)>,
        return_type: Option<Box<Type>>,
        body: Box<Expr>,
    },
    Tuple(Vec<Expr>),
    ModuleCall {
        module: TokenKind,
        method: StringId,
        args: Vec<Argument>,
    },
    As {
        expr: Box<Expr>,
        name: StringId,
    },
    Yield(Box<Expr>),
    Tag(StringId),
}
