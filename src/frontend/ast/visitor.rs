use super::stmt::{Stmt, StmtKind};
use super::expr::{Expr, ExprKind};

// Trait for traversing the XCX AST.
// Provides default no-op implementations for every node type.
pub trait AstVisitor {
    fn visit_program(&mut self, program: &super::node::Program) {
        for stmt in &program.stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::VarDecl { name: _, ty: _, value, is_const: _ } => {
                if let Some(v) = value { self.visit_expr(v); }
            }
            StmtKind::Print(e) | StmtKind::TerminalWrite(e) | StmtKind::ExprStmt(e) => {
                self.visit_expr(e);
            }
            StmtKind::Input(_, _) => {}
            StmtKind::If { condition, then_branch, else_ifs, else_branch } => {
                self.visit_expr(condition);
                for s in then_branch { self.visit_stmt(s); }
                for (c, b) in else_ifs {
                    self.visit_expr(c);
                    for s in b { self.visit_stmt(s); }
                }
                if let Some(b) = else_branch {
                    for s in b { self.visit_stmt(s); }
                }
            }
            StmtKind::While { condition, body } => {
                self.visit_expr(condition);
                for s in body { self.visit_stmt(s); }
            }
            StmtKind::For { var_name: _, start, end, step, body, iter_type: _ } => {
                self.visit_expr(start);
                self.visit_expr(end);
                if let Some(s) = step { self.visit_expr(s); }
                for s in body { self.visit_stmt(s); }
            }
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::Assign { name: _, value } => {
                self.visit_expr(value);
            }
            StmtKind::Halt { level: _, message } => {
                self.visit_expr(message);
            }
            StmtKind::FunctionDef { name: _, params: _, return_type: _, body } => {
                for s in body { self.visit_stmt(s); }
            }
            StmtKind::Return(e) => {
                if let Some(v) = e { self.visit_expr(v); }
            }
            StmtKind::FunctionCallStmt { name: _, args } => {
                for a in args { self.visit_expr(a.expr()); }
            }
            StmtKind::Include { .. } => {}
            StmtKind::JsonBind { json, path, target: _ } => {
                self.visit_expr(json);
                self.visit_expr(path);
            }
            StmtKind::JsonInject { json, mapping, table: _ } => {
                self.visit_expr(json);
                self.visit_expr(mapping);
            }
            StmtKind::FiberDef { name: _, params: _, return_type: _, body } => {
                for s in body { self.visit_stmt(s); }
            }
            StmtKind::FiberDecl { inner_type: _, name: _, fiber_name: _, args } => {
                for a in args { self.visit_expr(a.expr()); }
            }
            StmtKind::Yield { value: e, .. } | StmtKind::YieldFrom(e) => {
                self.visit_expr(e);
            }
            StmtKind::YieldVoid => {}
            StmtKind::NetRequestStmt { method, url, headers, body, timeout, target: _ } => {
                self.visit_expr(method);
                self.visit_expr(url);
                if let Some(h) = headers { self.visit_expr(h); }
                if let Some(b) = body { self.visit_expr(b); }
                if let Some(t) = timeout { self.visit_expr(t); }
            }
            StmtKind::Serve { name: _, port, host, workers, routes } => {
                self.visit_expr(port);
                if let Some(h) = host { self.visit_expr(h); }
                if let Some(w) = workers { self.visit_expr(w); }
                self.visit_expr(routes);
            }
            StmtKind::Wait(e) => {
                self.visit_expr(e);
            }
            StmtKind::DatabaseDecl { .. } => {}
            StmtKind::MultiVarDecl(stmts) => {
                for s in stmts { self.visit_stmt(s); }
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::IntLiteral(_) | ExprKind::FloatLiteral(_) | ExprKind::StringLiteral(_) | ExprKind::BoolLiteral(_) | ExprKind::Identifier(_) | ExprKind::RawBlock(_) => {}
            ExprKind::ArrayLiteral { elements } | ExprKind::Tuple(elements) | ExprKind::ArrayOrSetLiteral { elements } => {
                for e in elements { self.visit_expr(e); }
            }
            ExprKind::Binary { left, op: _, right } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            ExprKind::Unary { op: _, right } => {
                self.visit_expr(right);
            }
            ExprKind::FunctionCall { name: _, args } => {
                for a in args { self.visit_expr(a.expr()); }
            }
            ExprKind::MethodCall { receiver, method: _, args, wait_after: _ } => {
                self.visit_expr(receiver);
                for a in args { self.visit_expr(a.expr()); }
            }
            ExprKind::SetLiteral { set_type: _, elements, range } => {
                for e in elements { self.visit_expr(e); }
                if let Some(r) = range {
                    self.visit_expr(&r.start);
                    self.visit_expr(&r.end);
                    if let Some(s) = &r.step { self.visit_expr(s); }
                }
            }
            ExprKind::RandomChoice { set } => {
                self.visit_expr(set);
            }
            ExprKind::RandomInt { min, max, step } | ExprKind::RandomFloat { min, max, step } => {
                self.visit_expr(min);
                self.visit_expr(max);
                if let Some(s) = step { self.visit_expr(s); }
            }
            ExprKind::MapLiteral { key_type: _, value_type: _, elements } => {
                for (k, v) in elements {
                    self.visit_expr(k);
                    self.visit_expr(v);
                }
            }
            ExprKind::DateLiteral { .. } => {}
            ExprKind::TableLiteral { columns: _, rows } => {
                for row in rows {
                    for cell in row {
                        self.visit_expr(cell);
                    }
                }
            }
            ExprKind::DatabaseLiteral(elements) => {
                for (_, e) in elements {
                    self.visit_expr(e);
                }
            }
            ExprKind::Index { receiver, index } => {
                self.visit_expr(receiver);
                self.visit_expr(index);
            }
            ExprKind::MemberAccess { receiver, member: _ } => {
                self.visit_expr(receiver);
            }
            ExprKind::TerminalCommand(_, args) => {
                for a in args { self.visit_expr(a); }
            }
            ExprKind::Lambda { params: _, return_type: _, body } => {
                self.visit_expr(body);
            }
            ExprKind::ModuleCall { module: _, method: _, args } => {
                for a in args { self.visit_expr(a.expr()); }
            }
            ExprKind::As { expr, name: _ } => {
                self.visit_expr(expr);
            }
            ExprKind::Yield(e) => {
                self.visit_expr(e);
            }
            ExprKind::Tag(_) => {}
        }
    }
}
