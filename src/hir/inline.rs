use super::hir::{
    HirExpr, HirExprKind, HirStmt, HirStmtKind, HirArgument, HirRange,
    HirLocal, HirFunc, HirLocalDef, HirParam,
};

pub fn clone_expr(expr: &HirExpr, offset: u32) -> HirExpr {
    let kind = match &expr.kind {
        HirExprKind::IntLiteral(val) => HirExprKind::IntLiteral(*val),
        HirExprKind::FloatLiteral(val) => HirExprKind::FloatLiteral(*val),
        HirExprKind::StringLiteral(val) => HirExprKind::StringLiteral(*val),
        HirExprKind::BoolLiteral(val) => HirExprKind::BoolLiteral(*val),
        HirExprKind::Local(local) => HirExprKind::Local(local + offset),
        HirExprKind::Global(val) => HirExprKind::Global(*val),
        HirExprKind::RawBlock(val) => HirExprKind::RawBlock(*val),
        HirExprKind::ArrayLiteral { elements } => HirExprKind::ArrayLiteral {
            elements: elements.iter().map(|e| clone_expr(e, offset)).collect(),
        },
        HirExprKind::Binary { left, op, right } => HirExprKind::Binary {
            left: Box::new(clone_expr(left, offset)),
            op: op.clone(),
            right: Box::new(clone_expr(right, offset)),
        },
        HirExprKind::Unary { op, right } => HirExprKind::Unary {
            op: op.clone(),
            right: Box::new(clone_expr(right, offset)),
        },
        HirExprKind::FunctionCall { name, args } => HirExprKind::FunctionCall {
            name: *name,
            args: args.iter().map(|a| clone_arg(a, offset)).collect(),
        },
        HirExprKind::MethodCall { receiver, method, args, wait_after } => HirExprKind::MethodCall {
            receiver: Box::new(clone_expr(receiver, offset)),
            method: *method,
            args: args.iter().map(|a| clone_arg(a, offset)).collect(),
            wait_after: *wait_after,
        },
        HirExprKind::SetLiteral { set_type, elements, range } => HirExprKind::SetLiteral {
            set_type: set_type.clone(),
            elements: elements.iter().map(|e| clone_expr(e, offset)).collect(),
            range: range.as_ref().map(|r| clone_range(r, offset)),
        },
        HirExprKind::ArrayOrSetLiteral { elements } => HirExprKind::ArrayOrSetLiteral {
            elements: elements.iter().map(|e| clone_expr(e, offset)).collect(),
        },
        HirExprKind::RandomChoice { set } => HirExprKind::RandomChoice {
            set: Box::new(clone_expr(set, offset)),
        },
        HirExprKind::RandomInt { min, max, step } => HirExprKind::RandomInt {
            min: Box::new(clone_expr(min, offset)),
            max: Box::new(clone_expr(max, offset)),
            step: step.as_ref().map(|s| Box::new(clone_expr(s, offset))),
        },
        HirExprKind::RandomFloat { min, max, step } => HirExprKind::RandomFloat {
            min: Box::new(clone_expr(min, offset)),
            max: Box::new(clone_expr(max, offset)),
            step: step.as_ref().map(|s| Box::new(clone_expr(s, offset))),
        },
        HirExprKind::MapLiteral { key_type, value_type, elements } => HirExprKind::MapLiteral {
            key_type: key_type.clone(),
            value_type: value_type.clone(),
            elements: elements
                .iter()
                .map(|(k, v)| (clone_expr(k, offset), clone_expr(v, offset)))
                .collect(),
        },
        HirExprKind::DateLiteral { date_string, format } => HirExprKind::DateLiteral {
            date_string: *date_string,
            format: *format,
        },
        HirExprKind::TableLiteral { columns, rows } => HirExprKind::TableLiteral {
            columns: columns.clone(),
            rows: rows
                .iter()
                .map(|row| row.iter().map(|e| clone_expr(e, offset)).collect())
                .collect(),
        },
        HirExprKind::DatabaseLiteral(elements) => HirExprKind::DatabaseLiteral(
            elements
                .iter()
                .map(|(k, e)| (*k, clone_expr(e, offset)))
                .collect(),
        ),
        HirExprKind::Index { receiver, index } => HirExprKind::Index {
            receiver: Box::new(clone_expr(receiver, offset)),
            index: Box::new(clone_expr(index, offset)),
        },
        HirExprKind::MemberAccess { receiver, member } => HirExprKind::MemberAccess {
            receiver: Box::new(clone_expr(receiver, offset)),
            member: *member,
        },
        HirExprKind::TerminalCommand(cmd, args) => HirExprKind::TerminalCommand(
            *cmd,
            args.iter().map(|e| clone_expr(e, offset)).collect(),
        ),
        HirExprKind::Lambda { params, return_type, body, locals } => HirExprKind::Lambda {
            params: params
                .iter()
                .map(|p| HirParam {
                    ty: p.ty.clone(),
                    local: p.local + offset,
                    name: p.name,
                })
                .collect(),
            return_type: return_type.clone(),
            body: Box::new(clone_expr(body, offset)),
            locals: locals
                .iter()
                .map(|l| HirLocalDef {
                    name: l.name,
                    ty: l.ty.clone(),
                    is_const: l.is_const,
                })
                .collect(),
        },
        HirExprKind::Tuple(elements) => HirExprKind::Tuple(
            elements.iter().map(|e| clone_expr(e, offset)).collect(),
        ),
        HirExprKind::ModuleCall { module, method, args } => HirExprKind::ModuleCall {
            module: module.clone(),
            method: *method,
            args: args.iter().map(|a| clone_arg(a, offset)).collect(),
        },
        HirExprKind::As { expr, name } => HirExprKind::As {
            expr: Box::new(clone_expr(expr, offset)),
            name: *name,
        },
        HirExprKind::Yield(expr) => HirExprKind::Yield(Box::new(clone_expr(expr, offset))),
        HirExprKind::Tag(id) => HirExprKind::Tag(*id),
    };

    HirExpr {
        kind,
        span: expr.span.clone(),
        ty: expr.ty.clone(),
    }
}

fn clone_arg(arg: &HirArgument, offset: u32) -> HirArgument {
    match arg {
        HirArgument::Positional(e) => HirArgument::Positional(clone_expr(e, offset)),
        HirArgument::Named(name, e) => HirArgument::Named(*name, clone_expr(e, offset)),
    }
}

fn clone_range(range: &HirRange, offset: u32) -> HirRange {
    HirRange {
        start: Box::new(clone_expr(&range.start, offset)),
        end: Box::new(clone_expr(&range.end, offset)),
        step: range.step.as_ref().map(|s| Box::new(clone_expr(s, offset))),
    }
}

pub fn clone_stmt(stmt: &HirStmt, offset: u32, result_local: HirLocal) -> HirStmt {
    let kind = match &stmt.kind {
        HirStmtKind::VarDecl { local, value } => HirStmtKind::VarDecl {
            local: local + offset,
            value: value.as_ref().map(|v| Box::new(clone_expr(v, offset))),
        },
        HirStmtKind::Print(expr) => HirStmtKind::Print(Box::new(clone_expr(expr, offset))),
        HirStmtKind::TerminalWrite(expr) => HirStmtKind::TerminalWrite(Box::new(clone_expr(expr, offset))),
        HirStmtKind::Input(local, ty) => HirStmtKind::Input(local + offset, ty.clone()),
        HirStmtKind::ExprStmt(expr) => HirStmtKind::ExprStmt(Box::new(clone_expr(expr, offset))),
        HirStmtKind::If { condition, then_branch, else_ifs, else_branch } => HirStmtKind::If {
            condition: Box::new(clone_expr(condition, offset)),
            then_branch: then_branch.iter().map(|s| clone_stmt(s, offset, result_local)).collect(),
            else_ifs: else_ifs
                .iter()
                .map(|(cond, branch)| {
                    (
                        Box::new(clone_expr(cond, offset)),
                        branch.iter().map(|s| clone_stmt(s, offset, result_local)).collect(),
                    )
                })
                .collect(),
            else_branch: else_branch
                .as_ref()
                .map(|branch| branch.iter().map(|s| clone_stmt(s, offset, result_local)).collect()),
        },
        HirStmtKind::While { condition, body } => HirStmtKind::While {
            condition: Box::new(clone_expr(condition, offset)),
            body: body.iter().map(|s| clone_stmt(s, offset, result_local)).collect(),
        },
        HirStmtKind::For { local, start, end, step, body, iter_type } => HirStmtKind::For {
            local: local + offset,
            start: Box::new(clone_expr(start, offset)),
            end: Box::new(clone_expr(end, offset)),
            step: step.as_ref().map(|s| Box::new(clone_expr(s, offset))),
            body: body.iter().map(|s| clone_stmt(s, offset, result_local)).collect(),
            iter_type: *iter_type,
        },
        HirStmtKind::Break => HirStmtKind::Break,
        HirStmtKind::Continue => HirStmtKind::Continue,
        HirStmtKind::Assign { local, value } => HirStmtKind::Assign {
            local: local + offset,
            value: Box::new(clone_expr(value, offset)),
        },
        HirStmtKind::AssignGlobal { name, value } => HirStmtKind::AssignGlobal {
            name: *name,
            value: Box::new(clone_expr(value, offset)),
        },
        HirStmtKind::Halt { level, message } => HirStmtKind::Halt {
            level: level.clone(),
            message: Box::new(clone_expr(message, offset)),
        },
        HirStmtKind::Return(expr) => {
            HirStmtKind::Return(expr.as_ref().map(|e| Box::new(clone_expr(e, offset))))
        }
        HirStmtKind::FunctionCallStmt { name, args } => HirStmtKind::FunctionCallStmt {
            name: *name,
            args: args.iter().map(|a| clone_arg(a, offset)).collect(),
        },
        HirStmtKind::Include { path, alias } => HirStmtKind::Include {
            path: *path,
            alias: *alias,
        },
        HirStmtKind::JsonBind { json, path, target } => HirStmtKind::JsonBind {
            json: Box::new(clone_expr(json, offset)),
            path: Box::new(clone_expr(path, offset)),
            target: target + offset,
        },
        HirStmtKind::JsonBindGlobal { json, path, target } => HirStmtKind::JsonBindGlobal {
            json: Box::new(clone_expr(json, offset)),
            path: Box::new(clone_expr(path, offset)),
            target: *target,
        },
        HirStmtKind::JsonInject { json, mapping, table } => HirStmtKind::JsonInject {
            json: Box::new(clone_expr(json, offset)),
            mapping: Box::new(clone_expr(mapping, offset)),
            table: *table,
        },
        HirStmtKind::JsonInjectLocal { json, mapping, table } => HirStmtKind::JsonInjectLocal {
            json: Box::new(clone_expr(json, offset)),
            mapping: Box::new(clone_expr(mapping, offset)),
            table: table + offset,
        },
        HirStmtKind::FiberDecl { inner_type, target, fiber_name, args } => HirStmtKind::FiberDecl {
            inner_type: inner_type.clone(),
            target: target + offset,
            fiber_name: *fiber_name,
            args: args.iter().map(|a| clone_arg(a, offset)).collect(),
        },
        HirStmtKind::Yield { value, target } => HirStmtKind::Yield {
            value: Box::new(clone_expr(value, offset)),
            target: *target,
        },
        HirStmtKind::YieldFrom(expr) => HirStmtKind::YieldFrom(Box::new(clone_expr(expr, offset))),
        HirStmtKind::YieldVoid => HirStmtKind::YieldVoid,
        HirStmtKind::DatabaseDecl { name, fields } => HirStmtKind::DatabaseDecl {
            name: *name,
            fields: fields
                .iter()
                .map(|(k, e)| (*k, Box::new(clone_expr(e, offset))))
                .collect(),
        },
        HirStmtKind::NetRequestStmt { method, url, headers, body, timeout, target } => {
            HirStmtKind::NetRequestStmt {
                method: Box::new(clone_expr(method, offset)),
                url: Box::new(clone_expr(url, offset)),
                headers: headers.as_ref().map(|h| Box::new(clone_expr(h, offset))),
                body: body.as_ref().map(|b| Box::new(clone_expr(b, offset))),
                timeout: timeout.as_ref().map(|t| Box::new(clone_expr(t, offset))),
                target: target + offset,
            }
        }
        HirStmtKind::NetRequestStmtGlobal { method, url, headers, body, timeout, target } => {
            HirStmtKind::NetRequestStmtGlobal {
                method: Box::new(clone_expr(method, offset)),
                url: Box::new(clone_expr(url, offset)),
                headers: headers.as_ref().map(|h| Box::new(clone_expr(h, offset))),
                body: body.as_ref().map(|b| Box::new(clone_expr(b, offset))),
                timeout: timeout.as_ref().map(|t| Box::new(clone_expr(t, offset))),
                target: *target,
            }
        }
        HirStmtKind::Serve { name, port, host, workers, routes } => HirStmtKind::Serve {
            name: *name,
            port: Box::new(clone_expr(port, offset)),
            host: host.as_ref().map(|h| Box::new(clone_expr(h, offset))),
            workers: workers.as_ref().map(|w| Box::new(clone_expr(w, offset))),
            routes: Box::new(clone_expr(routes, offset)),
        },
        HirStmtKind::Wait(expr) => HirStmtKind::Wait(Box::new(clone_expr(expr, offset))),
        HirStmtKind::InlineBlock { stmts, result_local: block_result } => HirStmtKind::InlineBlock {
            stmts: stmts.iter().map(|s| clone_stmt(s, offset, result_local)).collect(),
            result_local: block_result.map(|r| r + offset),
        },
    };

    HirStmt {
        kind,
        span: stmt.span.clone(),
    }
}

pub fn inline_call_site(
    callee: &HirFunc,
    args: &[HirArgument],
    caller_next_local: u32,
) -> (Vec<HirStmt>, HirLocal) {
    let mut param_stmts = Vec::new();
    let result_local = caller_next_local + callee.locals.len() as u32;

    for (i, param) in callee.params.iter().enumerate() {
        let arg_expr = args[i].expr();
        let cloned_arg = clone_expr(arg_expr, 0);
        let local_def = param.local + caller_next_local;
        param_stmts.push(HirStmt {
            kind: HirStmtKind::VarDecl {
                local: local_def,
                value: Some(Box::new(cloned_arg)),
            },
            span: arg_expr.span.clone(),
        });
    }

    let mut body_stmts = Vec::new();
    for stmt in &callee.body {
        let cloned = clone_stmt(stmt, caller_next_local, result_local);
        body_stmts.push(cloned);
    }

    let mut inline_block_stmts = param_stmts;
    inline_block_stmts.extend(body_stmts);

    let inline_stmt = HirStmt {
        kind: HirStmtKind::InlineBlock {
            stmts: inline_block_stmts,
            result_local: Some(result_local),
        },
        span: callee.body.first().map(|s| s.span.clone()).unwrap_or_default(),
    };

    (vec![inline_stmt], result_local)
}
