use crate::frontend::ast::{Stmt, StmtKind, Type};
use crate::intern::StringId;
use crate::frontend::ast::argument::Argument;
use super::hir::{
    HirStmt, HirStmtKind, HirLocal, HirParam, HirLocalDef, HirArgument,
};
use super::lower_expr::lower_expr;
use std::collections::HashMap;

pub struct HirFuncBuilder {
    pub name: StringId,
    pub params: Vec<HirParam>,
    pub return_type: Option<Type>,
    pub body: Vec<HirStmt>,
    pub locals: Vec<HirLocalDef>,
    pub is_fiber: bool,
    pub scopes: Vec<HashMap<StringId, HirLocal>>,
    pub next_local: u32,
}

impl HirFuncBuilder {
    pub fn new(name: StringId, is_fiber: bool) -> Self {
        Self {
            name,
            params: Vec::new(),
            return_type: None,
            body: Vec::new(),
            locals: Vec::new(),
            is_fiber,
            scopes: vec![HashMap::new()],
            next_local: 0,
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn lookup_local(&self, name: &StringId) -> Option<HirLocal> {
        for scope in self.scopes.iter().rev() {
            if let Some(&local) = scope.get(name) {
                return Some(local);
            }
        }
        None
    }

    pub fn define_local(&mut self, name: StringId, ty: Type, is_const: bool) -> HirLocal {
        let local = self.next_local;
        self.next_local += 1;
        self.locals.push(HirLocalDef { name, ty, is_const });
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, local);
        }
        local
    }
}

fn lower_arg(
    arg: &Argument,
    builder: &HirFuncBuilder,
    func_indices: &HashMap<StringId, usize>,
    globals: &HashMap<StringId, usize>,
) -> HirArgument {
    let mut resolved_locals = HashMap::new();
    for scope in &builder.scopes {
        for (&k, &v) in scope {
            resolved_locals.insert(k, v);
        }
    }
    match arg {
        Argument::Positional(expr) => {
            HirArgument::Positional(lower_expr(expr, &resolved_locals, func_indices, globals))
        }
        Argument::Named(name, expr) => {
            HirArgument::Named(*name, lower_expr(expr, &resolved_locals, func_indices, globals))
        }
    }
}

pub fn lower_stmt(
    stmt: &Stmt,
    builder: &mut HirFuncBuilder,
    func_indices: &HashMap<StringId, usize>,
    globals: &HashMap<StringId, usize>,
) -> Vec<HirStmt> {
    let mut resolved_locals = HashMap::new();
    for scope in &builder.scopes {
        for (&k, &v) in scope {
            resolved_locals.insert(k, v);
        }
    }

    let kind = match &stmt.kind {
        StmtKind::VarDecl { is_const, ty, name, value } => {
            let local = builder.define_local(*name, *ty.clone(), *is_const);
            let val_hir = value.as_ref().map(|v| {
                Box::new(lower_expr(v, &resolved_locals, func_indices, globals))
            });
            HirStmtKind::VarDecl { local, value: val_hir }
        }
        StmtKind::MultiVarDecl(stmts) => {
            let mut result = Vec::new();
            for s in stmts {
                result.extend(lower_stmt(s, builder, func_indices, globals));
            }
            return result;
        }
        StmtKind::Print(expr) => {
            HirStmtKind::Print(Box::new(lower_expr(expr, &resolved_locals, func_indices, globals)))
        }
        StmtKind::TerminalWrite(expr) => {
            HirStmtKind::TerminalWrite(Box::new(lower_expr(expr, &resolved_locals, func_indices, globals)))
        }
        StmtKind::Input(name, ty) => {
            let local = builder.lookup_local(name).unwrap_or_else(|| {
                builder.define_local(*name, *ty.clone(), false)
            });
            HirStmtKind::Input(local, ty.clone())
        }
        StmtKind::ExprStmt(expr) => {
            HirStmtKind::ExprStmt(Box::new(lower_expr(expr, &resolved_locals, func_indices, globals)))
        }
        StmtKind::If { condition, then_branch, else_ifs, else_branch } => {
            builder.enter_scope();
            let mut then_hir = Vec::new();
            for s in then_branch {
                then_hir.extend(lower_stmt(s, builder, func_indices, globals));
            }
            builder.exit_scope();

            let mut else_ifs_hir = Vec::new();
            for (cond, branch) in else_ifs {
                builder.enter_scope();
                let cond_hir = Box::new(lower_expr(cond, &resolved_locals, func_indices, globals));
                let mut branch_hir = Vec::new();
                for s in branch {
                    branch_hir.extend(lower_stmt(s, builder, func_indices, globals));
                }
                else_ifs_hir.push((cond_hir, branch_hir));
                builder.exit_scope();
            }

            let else_hir = else_branch.as_ref().map(|branch| {
                builder.enter_scope();
                let mut branch_hir = Vec::new();
                for s in branch {
                    branch_hir.extend(lower_stmt(s, builder, func_indices, globals));
                }
                builder.exit_scope();
                branch_hir
            });

            HirStmtKind::If {
                condition: Box::new(lower_expr(condition, &resolved_locals, func_indices, globals)),
                then_branch: then_hir,
                else_ifs: else_ifs_hir,
                else_branch: else_hir,
            }
        }
        StmtKind::While { condition, body } => {
            builder.enter_scope();
            let mut body_hir = Vec::new();
            for s in body {
                body_hir.extend(lower_stmt(s, builder, func_indices, globals));
            }
            builder.exit_scope();

            HirStmtKind::While {
                condition: Box::new(lower_expr(condition, &resolved_locals, func_indices, globals)),
                body: body_hir,
            }
        }
        StmtKind::For { var_name, start, end, step, body, iter_type } => {
            builder.enter_scope();
            let loop_local = builder.define_local(*var_name, Type::Int, false);
            let mut body_hir = Vec::new();
            for s in body {
                body_hir.extend(lower_stmt(s, builder, func_indices, globals));
            }
            builder.exit_scope();

            HirStmtKind::For {
                local: loop_local,
                start: Box::new(lower_expr(start, &resolved_locals, func_indices, globals)),
                end: Box::new(lower_expr(end, &resolved_locals, func_indices, globals)),
                step: step.as_ref().map(|s| Box::new(lower_expr(s, &resolved_locals, func_indices, globals))),
                body: body_hir,
                iter_type: *iter_type,
            }
        }
        StmtKind::Break => HirStmtKind::Break,
        StmtKind::Continue => HirStmtKind::Continue,
        StmtKind::Assign { name, value } => {
            let val_hir = lower_expr(value, &resolved_locals, func_indices, globals);
            if let Some(local) = builder.lookup_local(name) {
                HirStmtKind::Assign { local, value: Box::new(val_hir) }
            } else {
                HirStmtKind::AssignGlobal { name: *name, value: Box::new(val_hir) }
            }
        }
        StmtKind::Halt { level, message } => HirStmtKind::Halt {
            level: level.clone(),
            message: Box::new(lower_expr(message, &resolved_locals, func_indices, globals)),
        },
        StmtKind::Return(expr) => {
            let val_hir = expr.as_ref().map(|e| {
                Box::new(lower_expr(e, &resolved_locals, func_indices, globals))
            });
            HirStmtKind::Return(val_hir)
        }
        StmtKind::FunctionCallStmt { name, args } => HirStmtKind::FunctionCallStmt {
            name: *name,
            args: args.iter().map(|a| lower_arg(a, builder, func_indices, globals)).collect(),
        },
        StmtKind::Include { path, alias } => HirStmtKind::Include {
            path: *path,
            alias: *alias,
        },
        StmtKind::JsonBind { json, path, target } => {
            let json_hir = lower_expr(json, &resolved_locals, func_indices, globals);
            let path_hir = lower_expr(path, &resolved_locals, func_indices, globals);
            if let Some(local) = builder.lookup_local(target) {
                HirStmtKind::JsonBind {
                    json: Box::new(json_hir),
                    path: Box::new(path_hir),
                    target: local,
                }
            } else {
                HirStmtKind::JsonBindGlobal {
                    json: Box::new(json_hir),
                    path: Box::new(path_hir),
                    target: *target,
                }
            }
        }
        StmtKind::JsonInject { json, mapping, table } => {
            let json_hir = lower_expr(json, &resolved_locals, func_indices, globals);
            let mapping_hir = lower_expr(mapping, &resolved_locals, func_indices, globals);
            if let Some(local) = builder.lookup_local(table) {
                HirStmtKind::JsonInjectLocal {
                    json: Box::new(json_hir),
                    mapping: Box::new(mapping_hir),
                    table: local,
                }
            } else {
                HirStmtKind::JsonInject {
                    json: Box::new(json_hir),
                    mapping: Box::new(mapping_hir),
                    table: *table,
                }
            }
        }
        StmtKind::Yield { value, target } => HirStmtKind::Yield {
            value: Box::new(lower_expr(value, &resolved_locals, func_indices, globals)),
            target: *target,
        },
        StmtKind::YieldFrom(expr) => {
            HirStmtKind::YieldFrom(Box::new(lower_expr(expr, &resolved_locals, func_indices, globals)))
        }
        StmtKind::YieldVoid => HirStmtKind::YieldVoid,
        StmtKind::DatabaseDecl { name, fields } => {
            let fields_hir = fields
                .iter()
                .map(|(k, e)| (*k, Box::new(lower_expr(e, &resolved_locals, func_indices, globals))))
                .collect();
            HirStmtKind::DatabaseDecl {
                name: *name,
                fields: fields_hir,
            }
        }
        StmtKind::NetRequestStmt { method, url, headers, body, timeout, target } => {
            let method_hir = lower_expr(method, &resolved_locals, func_indices, globals);
            let url_hir = lower_expr(url, &resolved_locals, func_indices, globals);
            let headers_hir = headers.as_ref().map(|h| Box::new(lower_expr(h, &resolved_locals, func_indices, globals)));
            let body_hir = body.as_ref().map(|b| Box::new(lower_expr(b, &resolved_locals, func_indices, globals)));
            let timeout_hir = timeout.as_ref().map(|t| Box::new(lower_expr(t, &resolved_locals, func_indices, globals)));

            if let Some(local) = builder.lookup_local(target) {
                HirStmtKind::NetRequestStmt {
                    method: Box::new(method_hir),
                    url: Box::new(url_hir),
                    headers: headers_hir,
                    body: body_hir,
                    timeout: timeout_hir,
                    target: local,
                }
            } else {
                HirStmtKind::NetRequestStmtGlobal {
                    method: Box::new(method_hir),
                    url: Box::new(url_hir),
                    headers: headers_hir,
                    body: body_hir,
                    timeout: timeout_hir,
                    target: *target,
                }
            }
        }
        StmtKind::Serve { name, port, host, workers, routes } => {
            let port_hir = lower_expr(port, &resolved_locals, func_indices, globals);
            let host_hir = host.as_ref().map(|h| Box::new(lower_expr(h, &resolved_locals, func_indices, globals)));
            let workers_hir = workers.as_ref().map(|w| Box::new(lower_expr(w, &resolved_locals, func_indices, globals)));
            let routes_hir = lower_expr(routes, &resolved_locals, func_indices, globals);

            HirStmtKind::Serve {
                name: *name,
                port: Box::new(port_hir),
                host: host_hir,
                workers: workers_hir,
                routes: Box::new(routes_hir),
            }
        }
        StmtKind::Wait(expr) => {
            HirStmtKind::Wait(Box::new(lower_expr(expr, &resolved_locals, func_indices, globals)))
        }
        StmtKind::FiberDecl { inner_type, name, fiber_name, args } => {
            let local = builder.define_local(*name, Type::Fiber((*inner_type).clone()), false);
            let args_hir = args.iter().map(|a| lower_arg(a, builder, func_indices, globals)).collect();
            HirStmtKind::FiberDecl {
                inner_type: inner_type.clone(),
                target: local,
                fiber_name: *fiber_name,
                args: args_hir,
            }
        }
        StmtKind::FunctionDef { .. } | StmtKind::FiberDef { .. } => {
            return Vec::new();
        }
    };

    vec![HirStmt {
        kind,
        span: stmt.span.clone(),
    }]
}
