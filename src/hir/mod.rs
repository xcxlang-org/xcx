pub mod hir;
pub mod lower;
pub mod lower_expr;
pub mod lower_stmt;
pub mod inline;
pub mod inline_policy;
pub mod pass;
pub mod compile_hir;
pub mod compile_expr;
pub mod compile_expr_special;

pub use hir::{
    HirLocal, HirBinOp, HirUnOp, HirArgument, HirParam, HirLocalDef, HirRange,
    HirExpr, HirExprKind, HirStmt, HirStmtKind, HirFunc,
};
pub use lower::{lower_func, lower_program};
pub use pass::run_inliner_pass;
pub use compile_hir::compile_hir_to_chunk;
