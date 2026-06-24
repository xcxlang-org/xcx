use crate::intern::StringId;
use crate::error::Span;
use super::ty::Type;
use super::expr::Expr;
use super::argument::Argument;

// Levels for the structured `halt` system.
#[derive(Debug, Clone, PartialEq)]
pub enum HaltLevel {
    Alert,
    Error,
    Fatal,
}

// Iteration mode for `for` loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForIterType {
    Range,
    Array,
    Set,
    Fiber,
}

// Represents a single statement in the XCX AST.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

// Enumerates all types of statements in the XCX language.
#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    VarDecl {
        is_const: bool,
        ty: Box<Type>,
        name: StringId,
        value: Option<Box<Expr>>,
    },
    Print(Box<Expr>),
    TerminalWrite(Box<Expr>),
    Input(StringId, Box<Type>),
    ExprStmt(Box<Expr>),
    If {
        condition: Box<Expr>,
        then_branch: Vec<Stmt>,
        else_ifs: Vec<(Box<Expr>, Vec<Stmt>)>,
        else_branch: Option<Vec<Stmt>>,
    },
    While {
        condition: Box<Expr>,
        body: Vec<Stmt>,
    },
    For {
        var_name: StringId,
        start: Box<Expr>,
        end: Box<Expr>,
        step: Option<Box<Expr>>,
        body: Vec<Stmt>,
        iter_type: ForIterType,
    },
    Break,
    Continue,
    Assign {
        name: StringId,
        value: Box<Expr>,
    },
    Halt {
        level: HaltLevel,
        message: Box<Expr>,
    },
    FunctionDef {
        name: StringId,
        params: Vec<(Type, StringId)>,
        return_type: Option<Box<Type>>,
        body: Vec<Stmt>,
    },
    Return(Option<Box<Expr>>),
    FunctionCallStmt {
        name: StringId,
        args: Vec<Argument>,
    },
    Include {
        path: StringId,
        alias: Option<StringId>,
    },
    JsonBind {
        json: Box<Expr>,
        path: Box<Expr>,
        target: StringId,
    },
    JsonInject {
        json: Box<Expr>,
        mapping: Box<Expr>,
        table: StringId,
    },
    FiberDef {
        name: StringId,
        params: Vec<(Type, StringId)>,
        return_type: Option<Box<Type>>,   
        body: Vec<Stmt>,
    },
    FiberDecl {
        inner_type: Option<Box<Type>>,    
        name: StringId,              
        fiber_name: StringId,        
        args: Vec<Argument>,
    },
    Yield {
        value: Box<Expr>,
        target: Option<StringId>,
    },
    YieldFrom(Box<Expr>),
    YieldVoid,
    DatabaseDecl {
        name: StringId,
        fields: Vec<(StringId, Box<Expr>)>,
    },
    NetRequestStmt {
        method: Box<Expr>,
        url: Box<Expr>,
        headers: Option<Box<Expr>>,
        body: Option<Box<Expr>>,
        timeout: Option<Box<Expr>>,
        target: StringId,
    },
    Serve {
        name: StringId,
        port: Box<Expr>,
        host: Option<Box<Expr>>,
        workers: Option<Box<Expr>>,
        routes: Box<Expr>,
    },
    Wait(Box<Expr>),
    MultiVarDecl(Vec<Stmt>),
}
