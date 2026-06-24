use crate::frontend::ast::{Stmt, StmtKind};
use crate::sema::symbol::symbol_table::SymbolTable;
use crate::sema::error::type_error::TypeError;
use crate::sema::error::error_kind::TypeErrorKind;
use super::checker::Checker;

impl<'a> Checker<'a> {

    pub fn check_stmt(&mut self, stmt: &mut Stmt, symbols: &mut SymbolTable<'_>, errors: &mut Vec<TypeError>) {
        let span = stmt.span.clone();
        match &mut stmt.kind {
            StmtKind::VarDecl { is_const, ty, name, value } => {
                self.check_var_decl(*is_const, ty, name, value, symbols, errors, &span);
            }
            StmtKind::Input(name, ty) => {
                self.check_input(*name, ty, symbols, errors, &span);
            }
            StmtKind::Print(expr) | StmtKind::TerminalWrite(expr) => {
                self.check_io_expr(expr, symbols, errors);
            }
            StmtKind::Halt { message, .. } => {
                self.check_halt(message, symbols, errors);
            }
            StmtKind::FunctionDef { name, params, return_type, body } => {
                self.check_func_def(name, params, return_type, body, symbols, errors);
            }
            StmtKind::Return(expr) => {
                self.check_return(expr, symbols, errors, &span);
            }
            StmtKind::ExprStmt(expr) => {
                self.check_expr_stmt(expr, symbols, errors);
            }
            StmtKind::If { condition, then_branch, else_ifs, else_branch } => {
                self.check_if(condition, then_branch, else_ifs, else_branch, symbols, errors);
            }
            StmtKind::While { condition, body } => {
                self.check_while(condition, body, symbols, errors);
            }
            StmtKind::For { var_name, start, end, step, body, iter_type } => {
                self.check_for(var_name, start, end, step, body, iter_type, symbols, errors, &span);
            }
            StmtKind::Break => {
                if self.loop_depth == 0 {
                    errors.push(TypeError { kind: TypeErrorKind::BreakOutsideLoop, span });
                }
            }
            StmtKind::Continue => {
                if self.loop_depth == 0 {
                    errors.push(TypeError { kind: TypeErrorKind::ContinueOutsideLoop, span });
                }
            }
            StmtKind::Assign { name, value } => {
                self.check_assign(name, value, symbols, errors, &span);
            }
            StmtKind::Include { .. } => {}
            StmtKind::FunctionCallStmt { name, args } => {
                self.check_function_call_stmt(name, args, symbols, errors, &span);
            }
            StmtKind::JsonBind { json, path, target } => {
                self.check_json_bind(json, path, *target, symbols, errors, &span);
            }
            StmtKind::JsonInject { json, mapping, table } => {
                self.check_json_inject(json, mapping, *table, symbols, errors, &span);
            }
            StmtKind::FiberDef { name, params, return_type, body } => {
                self.check_fiber_def(name, params, return_type, body, symbols, errors, &span);
            }
            StmtKind::FiberDecl { inner_type, name, fiber_name, args } => {
                self.check_fiber_decl(inner_type, name, fiber_name, args, symbols, errors, &span);
            }
            StmtKind::Yield { value, target } => {
                self.check_yield_stmt_with_target(value, target.as_ref(), symbols, errors, &span);
            }
            StmtKind::YieldFrom(expr) => {
                self.check_yield_from(expr, symbols, errors, &span);
            }
            StmtKind::YieldVoid => {
                self.check_yield_void(errors, &span);
            }
            StmtKind::DatabaseDecl { name, fields } => {
                self.check_database_decl(name, fields, symbols, errors, &span);
            }
            StmtKind::NetRequestStmt { method, url, headers, body, timeout, target } => {
                self.check_net_request_stmt(method, url, headers, body, timeout, *target, symbols, errors);
            }
            StmtKind::Serve { port, host, workers, routes, .. } => {
                self.check_serve(port, host, workers, routes, symbols, errors);
            }
            StmtKind::Wait(expr) => {
                self.check_wait(expr, symbols, errors);
            }
            StmtKind::MultiVarDecl(stmts) => {
                for s in stmts {
                    self.check_stmt(s, symbols, errors);
                }
            }
        }
    }

}
