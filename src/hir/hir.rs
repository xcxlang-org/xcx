use crate::intern::StringId;
use crate::error::Span;
use crate::frontend::lexer::TokenKind;
use crate::sema::types::{Type, SetType};
use crate::frontend::ast::stmt::HaltLevel;
use crate::frontend::ast::table::ColumnDef;

pub type HirLocal = u32;

pub const LAMBDA_LOCAL_OFFSET: u32 = 1_000_000;

#[derive(Debug, Clone, PartialEq)]
pub enum HirBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    And,
    Or,
    Has,
    SetUnion,
    SetIntersection,
    SetDifference,
    SetSymDifference,
    IntConcat,
    MapConcat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirUnOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirArgument {
    Positional(HirExpr),
    Named(StringId, HirExpr),
}

impl HirArgument {
    pub fn expr(&self) -> &HirExpr {
        match self {
            HirArgument::Positional(e) => e,
            HirArgument::Named(_, e) => e,
        }
    }

    pub fn expr_mut(&mut self) -> &mut HirExpr {
        match self {
            HirArgument::Positional(e) => e,
            HirArgument::Named(_, e) => e,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirParam {
    pub ty: Type,
    pub local: HirLocal,
    pub name: StringId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirLocalDef {
    pub name: StringId,
    pub ty: Type,
    pub is_const: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirRange {
    pub start: Box<HirExpr>,
    pub end: Box<HirExpr>,
    pub step: Option<Box<HirExpr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirExpr {
    pub kind: HirExprKind,
    pub span: Span,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirExprKind {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(StringId),
    BoolLiteral(bool),
    Local(HirLocal),
    Global(StringId),
    RawBlock(StringId),
    ArrayLiteral {
        elements: Vec<HirExpr>,
    },
    Binary {
        left: Box<HirExpr>,
        op: HirBinOp,
        right: Box<HirExpr>,
    },
    Unary {
        op: HirUnOp,
        right: Box<HirExpr>,
    },
    FunctionCall {
        name: StringId,
        args: Vec<HirArgument>,
    },
    MethodCall {
        receiver: Box<HirExpr>,
        method: StringId,
        args: Vec<HirArgument>,
        wait_after: bool,
    },
    SetLiteral {
        set_type: SetType,
        elements: Vec<HirExpr>,
        range: Option<HirRange>,
    },
    ArrayOrSetLiteral {
        elements: Vec<HirExpr>,
    },
    RandomChoice {
        set: Box<HirExpr>,
    },
    RandomInt {
        min: Box<HirExpr>,
        max: Box<HirExpr>,
        step: Option<Box<HirExpr>>,
    },
    RandomFloat {
        min: Box<HirExpr>,
        max: Box<HirExpr>,
        step: Option<Box<HirExpr>>,
    },
    MapLiteral {
        key_type: Box<Type>,
        value_type: Box<Type>,
        elements: Vec<(HirExpr, HirExpr)>,
    },
    DateLiteral {
        date_string: StringId,
        format: Option<StringId>,
    },
    TableLiteral {
        columns: Vec<ColumnDef>,
        rows: Vec<Vec<HirExpr>>,
    },
    DatabaseLiteral(Vec<(StringId, HirExpr)>),
    Index {
        receiver: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    MemberAccess {
        receiver: Box<HirExpr>,
        member: StringId,
    },
    TerminalCommand(StringId, Vec<HirExpr>),
    Lambda {
        params: Vec<HirParam>,
        return_type: Option<Box<Type>>,
        body: Box<HirExpr>,
        locals: Vec<HirLocalDef>,
    },
    Tuple(Vec<HirExpr>),
    ModuleCall {
        module: TokenKind,
        method: StringId,
        args: Vec<HirArgument>,
    },
    As {
        expr: Box<HirExpr>,
        name: StringId,
    },
    Yield(Box<HirExpr>),
    Tag(StringId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirStmt {
    pub kind: HirStmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HirStmtKind {
    VarDecl {
        local: HirLocal,
        value: Option<Box<HirExpr>>,
    },
    Print(Box<HirExpr>),
    TerminalWrite(Box<HirExpr>),
    Input(HirLocal, Box<Type>),
    ExprStmt(Box<HirExpr>),
    If {
        condition: Box<HirExpr>,
        then_branch: Vec<HirStmt>,
        else_ifs: Vec<(Box<HirExpr>, Vec<HirStmt>)>,
        else_branch: Option<Vec<HirStmt>>,
    },
    While {
        condition: Box<HirExpr>,
        body: Vec<HirStmt>,
    },
    For {
        local: HirLocal,
        start: Box<HirExpr>,
        end: Box<HirExpr>,
        step: Option<Box<HirExpr>>,
        body: Vec<HirStmt>,
        iter_type: crate::frontend::ast::stmt::ForIterType,
    },
    Break,
    Continue,
    Assign {
        local: HirLocal,
        value: Box<HirExpr>,
    },
    AssignGlobal {
        name: StringId,
        value: Box<HirExpr>,
    },
    Halt {
        level: HaltLevel,
        message: Box<HirExpr>,
    },
    Return(Option<Box<HirExpr>>),
    FunctionCallStmt {
        name: StringId,
        args: Vec<HirArgument>,
    },
    Include {
        path: StringId,
        alias: Option<StringId>,
    },
    JsonBind {
        json: Box<HirExpr>,
        path: Box<HirExpr>,
        target: HirLocal,
    },
    JsonBindGlobal {
        json: Box<HirExpr>,
        path: Box<HirExpr>,
        target: StringId,
    },
    JsonInject {
        json: Box<HirExpr>,
        mapping: Box<HirExpr>,
        table: StringId,
    },
    JsonInjectLocal {
        json: Box<HirExpr>,
        mapping: Box<HirExpr>,
        table: HirLocal,
    },
    FiberDecl {
        inner_type: Option<Box<Type>>,
        target: HirLocal,
        fiber_name: StringId,
        args: Vec<HirArgument>,
    },
    Yield {
        value: Box<HirExpr>,
        target: Option<StringId>,
    },
    YieldFrom(Box<HirExpr>),
    YieldVoid,
    DatabaseDecl {
        name: StringId,
        fields: Vec<(StringId, Box<HirExpr>)>,
    },
    NetRequestStmt {
        method: Box<HirExpr>,
        url: Box<HirExpr>,
        headers: Option<Box<HirExpr>>,
        body: Option<Box<HirExpr>>,
        timeout: Option<Box<HirExpr>>,
        target: HirLocal,
    },
    NetRequestStmtGlobal {
        method: Box<HirExpr>,
        url: Box<HirExpr>,
        headers: Option<Box<HirExpr>>,
        body: Option<Box<HirExpr>>,
        timeout: Option<Box<HirExpr>>,
        target: StringId,
    },
    Serve {
        name: StringId,
        port: Box<HirExpr>,
        host: Option<Box<HirExpr>>,
        workers: Option<Box<HirExpr>>,
        routes: Box<HirExpr>,
    },
    Wait(Box<HirExpr>),
    InlineBlock {
        stmts: Vec<HirStmt>,
        result_local: Option<HirLocal>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirFunc {
    pub name: StringId,
    pub params: Vec<HirParam>,
    pub return_type: Option<Type>,
    pub body: Vec<HirStmt>,
    pub locals: Vec<HirLocalDef>,
    pub is_fiber: bool,
}
