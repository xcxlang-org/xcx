use super::hir::{HirFunc, HirStmt, HirStmtKind, HirExpr, HirExprKind};
use crate::intern::StringId;

pub fn should_inline(callee: &HirFunc, caller: &HirFunc, depth: usize) -> bool {
    if callee.is_fiber || caller.is_fiber {
        return false;
    }
    if depth >= 3 {
        return false;
    }
    if callee.name == caller.name {
        return false;
    }

    let mut has_recursion = false;
    check_ref_recursion(callee, callee.name, &mut has_recursion);
    if has_recursion {
        return false;
    }

    for stmt in &callee.body {
        if is_return_in_loop(stmt, false) {
            return false;
        }
    }

    let cost = calculate_func_cost(callee);
    if cost >= 20 {
        return false;
    }

    true
}

fn calculate_func_cost(func: &HirFunc) -> usize {
    let mut cost = 0;
    for stmt in &func.body {
        cost += calculate_stmt_cost(stmt);
    }
    cost
}

fn calculate_stmt_cost(stmt: &HirStmt) -> usize {
    let base = 1;
    let extra = match &stmt.kind {
        HirStmtKind::If { then_branch, else_ifs, else_branch, .. } => {
            let mut sum = calculate_block_cost(then_branch);
            for (_, branch) in else_ifs {
                sum += calculate_block_cost(branch);
            }
            if let Some(branch) = else_branch {
                sum += calculate_block_cost(branch);
            }
            sum
        }
        HirStmtKind::While { body, .. } => calculate_block_cost(body),
        HirStmtKind::For { body, .. } => calculate_block_cost(body),
        HirStmtKind::InlineBlock { stmts, .. } => calculate_block_cost(stmts),
        _ => 0,
    };
    base + extra
}

fn calculate_block_cost(stmts: &[HirStmt]) -> usize {
    stmts.iter().map(calculate_stmt_cost).sum()
}



fn check_ref_recursion(func: &HirFunc, target_name: StringId, has_recursion: &mut bool) {
    for stmt in &func.body {
        check_stmt_recursion(stmt, target_name, has_recursion);
        if *has_recursion {
            return;
        }
    }
}

fn check_stmt_recursion(stmt: &HirStmt, target: StringId, has_recursion: &mut bool) {
    match &stmt.kind {
        HirStmtKind::ExprStmt(expr) => check_expr_recursion(expr, target, has_recursion),
        HirStmtKind::VarDecl { value, .. } => {
            if let Some(val) = value {
                check_expr_recursion(val, target, has_recursion);
            }
        }
        HirStmtKind::Assign { value, .. } => check_expr_recursion(value, target, has_recursion),
        HirStmtKind::AssignGlobal { value, .. } => check_expr_recursion(value, target, has_recursion),
        HirStmtKind::Print(expr) => check_expr_recursion(expr, target, has_recursion),
        HirStmtKind::TerminalWrite(expr) => check_expr_recursion(expr, target, has_recursion),
        HirStmtKind::If { condition, then_branch, else_ifs, else_branch } => {
            check_expr_recursion(condition, target, has_recursion);
            for s in then_branch {
                check_stmt_recursion(s, target, has_recursion);
            }
            for (cond, branch) in else_ifs {
                check_expr_recursion(cond, target, has_recursion);
                for s in branch {
                    check_stmt_recursion(s, target, has_recursion);
                }
            }
            if let Some(branch) = else_branch {
                for s in branch {
                    check_stmt_recursion(s, target, has_recursion);
                }
            }
        }
        HirStmtKind::While { condition, body } => {
            check_expr_recursion(condition, target, has_recursion);
            for s in body {
                check_stmt_recursion(s, target, has_recursion);
            }
        }
        HirStmtKind::For { start, end, step, body, .. } => {
            check_expr_recursion(start, target, has_recursion);
            check_expr_recursion(end, target, has_recursion);
            if let Some(s) = step {
                check_expr_recursion(s, target, has_recursion);
            }
            for s in body {
                check_stmt_recursion(s, target, has_recursion);
            }
        }
        HirStmtKind::Return(expr) => {
            if let Some(e) = expr {
                check_expr_recursion(e, target, has_recursion);
            }
        }
        HirStmtKind::FunctionCallStmt { name, args } => {
            if *name == target {
                *has_recursion = true;
                return;
            }
            for arg in args {
                check_expr_recursion(arg.expr(), target, has_recursion);
            }
        }
        HirStmtKind::JsonBind { json, path, .. } => {
            check_expr_recursion(json, target, has_recursion);
            check_expr_recursion(path, target, has_recursion);
        }
        HirStmtKind::JsonBindGlobal { json, path, .. } => {
            check_expr_recursion(json, target, has_recursion);
            check_expr_recursion(path, target, has_recursion);
        }
        HirStmtKind::JsonInject { json, mapping, .. } => {
            check_expr_recursion(json, target, has_recursion);
            check_expr_recursion(mapping, target, has_recursion);
        }
        HirStmtKind::JsonInjectLocal { json, mapping, .. } => {
            check_expr_recursion(json, target, has_recursion);
            check_expr_recursion(mapping, target, has_recursion);
        }
        HirStmtKind::Yield { value, .. } => {
            check_expr_recursion(value, target, has_recursion);
        }
        HirStmtKind::YieldFrom(expr) => {
            check_expr_recursion(expr, target, has_recursion);
        }
        HirStmtKind::NetRequestStmt { method, url, headers, body: req_body, timeout, .. } => {
            check_expr_recursion(method, target, has_recursion);
            check_expr_recursion(url, target, has_recursion);
            if let Some(h) = headers {
                check_expr_recursion(h, target, has_recursion);
            }
            if let Some(b) = req_body {
                check_expr_recursion(b, target, has_recursion);
            }
            if let Some(t) = timeout {
                check_expr_recursion(t, target, has_recursion);
            }
        }
        HirStmtKind::NetRequestStmtGlobal { method, url, headers, body: req_body, timeout, .. } => {
            check_expr_recursion(method, target, has_recursion);
            check_expr_recursion(url, target, has_recursion);
            if let Some(h) = headers {
                check_expr_recursion(h, target, has_recursion);
            }
            if let Some(b) = req_body {
                check_expr_recursion(b, target, has_recursion);
            }
            if let Some(t) = timeout {
                check_expr_recursion(t, target, has_recursion);
            }
        }
        HirStmtKind::Serve { port, host, workers, routes, .. } => {
            check_expr_recursion(port, target, has_recursion);
            if let Some(h) = host {
                check_expr_recursion(h, target, has_recursion);
            }
            if let Some(w) = workers {
                check_expr_recursion(w, target, has_recursion);
            }
            check_expr_recursion(routes, target, has_recursion);
        }
        HirStmtKind::Wait(expr) => check_expr_recursion(expr, target, has_recursion),
        HirStmtKind::InlineBlock { stmts, .. } => {
            for s in stmts {
                check_stmt_recursion(s, target, has_recursion);
            }
        }
        // Bypasses statement kinds enclosing no expression or statement recursion:
        // Break, Continue, YieldVoid, Input, Halt, Include, FiberDecl, DatabaseDecl
        _ => {}
    }
}

fn check_expr_recursion(expr: &HirExpr, target: StringId, has_recursion: &mut bool) {
    if *has_recursion {
        return;
    }
    match &expr.kind {
        HirExprKind::FunctionCall { name, args } => {
            if *name == target {
                *has_recursion = true;
                return;
            }
            for arg in args {
                check_expr_recursion(arg.expr(), target, has_recursion);
            }
        }
        HirExprKind::MethodCall { receiver, args, .. } => {
            check_expr_recursion(receiver, target, has_recursion);
            for arg in args {
                check_expr_recursion(arg.expr(), target, has_recursion);
            }
        }
        HirExprKind::Binary { left, right, .. } => {
            check_expr_recursion(left, target, has_recursion);
            check_expr_recursion(right, target, has_recursion);
        }
        HirExprKind::Unary { right, .. } => {
            check_expr_recursion(right, target, has_recursion);
        }
        HirExprKind::ArrayLiteral { elements } => {
            for e in elements {
                check_expr_recursion(e, target, has_recursion);
            }
        }
        HirExprKind::SetLiteral { elements, range, .. } => {
            for e in elements {
                check_expr_recursion(e, target, has_recursion);
            }
            if let Some(r) = range {
                check_expr_recursion(&r.start, target, has_recursion);
                check_expr_recursion(&r.end, target, has_recursion);
                if let Some(s) = &r.step {
                    check_expr_recursion(s, target, has_recursion);
                }
            }
        }
        HirExprKind::ArrayOrSetLiteral { elements } => {
            for e in elements {
                check_expr_recursion(e, target, has_recursion);
            }
        }
        HirExprKind::RandomChoice { set } => check_expr_recursion(set, target, has_recursion),
        HirExprKind::RandomInt { min, max, step } => {
            check_expr_recursion(min, target, has_recursion);
            check_expr_recursion(max, target, has_recursion);
            if let Some(s) = step {
                check_expr_recursion(s, target, has_recursion);
            }
        }
        HirExprKind::RandomFloat { min, max, step } => {
            check_expr_recursion(min, target, has_recursion);
            check_expr_recursion(max, target, has_recursion);
            if let Some(s) = step {
                check_expr_recursion(s, target, has_recursion);
            }
        }
        HirExprKind::MapLiteral { elements, .. } => {
            for (k, v) in elements {
                check_expr_recursion(k, target, has_recursion);
                check_expr_recursion(v, target, has_recursion);
            }
        }
        HirExprKind::TableLiteral { rows, .. } => {
            for row in rows {
                for e in row {
                    check_expr_recursion(e, target, has_recursion);
                }
            }
        }
        HirExprKind::DatabaseLiteral(elements) => {
            for (_, e) in elements {
                check_expr_recursion(e, target, has_recursion);
            }
        }
        HirExprKind::Index { receiver, index } => {
            check_expr_recursion(receiver, target, has_recursion);
            check_expr_recursion(index, target, has_recursion);
        }
        HirExprKind::MemberAccess { receiver, .. } => {
            check_expr_recursion(receiver, target, has_recursion);
        }
        HirExprKind::TerminalCommand(_, args) => {
            for e in args {
                check_expr_recursion(e, target, has_recursion);
            }
        }
        HirExprKind::Lambda { body, .. } => {
            check_expr_recursion(body, target, has_recursion);
        }
        HirExprKind::Tuple(elements) => {
            for e in elements {
                check_expr_recursion(e, target, has_recursion);
            }
        }
        HirExprKind::ModuleCall { args, .. } => {
            for arg in args {
                check_expr_recursion(arg.expr(), target, has_recursion);
            }
        }
        HirExprKind::As { expr, .. } => check_expr_recursion(expr, target, has_recursion),
        HirExprKind::Yield(expr) => check_expr_recursion(expr, target, has_recursion),
        // Bypasses leaf node expressions that cannot contain nested expressions:
        // IntLiteral, FloatLiteral, StringLiteral, BoolLiteral, Local, Global, RawBlock, DateLiteral, Tag
        _ => {}
    }
}

fn is_return_in_loop(stmt: &HirStmt, in_loop: bool) -> bool {
    match &stmt.kind {
        HirStmtKind::Return(_) => in_loop,
        HirStmtKind::If { then_branch, else_ifs, else_branch, .. } => {
            then_branch.iter().any(|s| is_return_in_loop(s, in_loop))
                || else_ifs.iter().any(|(_, branch)| branch.iter().any(|s| is_return_in_loop(s, in_loop)))
                || else_branch.as_ref().map_or(false, |branch| branch.iter().any(|s| is_return_in_loop(s, in_loop)))
        }
        HirStmtKind::While { body, .. } => {
            body.iter().any(|s| is_return_in_loop(s, true))
        }
        HirStmtKind::For { body, .. } => {
            body.iter().any(|s| is_return_in_loop(s, true))
        }
        HirStmtKind::InlineBlock { stmts, .. } => {
            stmts.iter().any(|s| is_return_in_loop(s, in_loop))
        }
        _ => false,
    }
}
