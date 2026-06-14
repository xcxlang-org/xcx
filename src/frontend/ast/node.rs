use crate::error::Span;
use super::stmt::Stmt;

// A generic wrapper for AST nodes that attaches source location metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct AstNode<T> {
    pub data: T,
    pub span: Span,
}

// The root node of a parsed XCX source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}
