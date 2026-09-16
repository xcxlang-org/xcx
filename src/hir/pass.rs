use std::collections::HashMap;
use crate::intern::StringId;
use super::hir::{
    HirFunc, HirStmt, HirStmtKind, HirExpr, HirExprKind, HirLocalDef,
};

pub fn run_inliner_pass(funcs: &mut HashMap<u32, HirFunc>) {
    let mut func_indices = HashMap::new();
    for (&id, func) in funcs.iter() {
        func_indices.insert(func.name, id);
    }

    for depth in 0..3 {
        let mut changed = false;
        let mut updated_funcs = HashMap::new();

        for (&id, func) in funcs.iter() {
            let mut next_local = func.locals.len() as u32;
            let mut new_locals = Vec::new();
            let mut new_body = Vec::new();

            for stmt in &func.body {
                new_body.extend(inline_in_stmt(
                    stmt.clone(),
                    funcs,
                    &func_indices,
                    &mut next_local,
                    func,
                    depth,
                    &mut new_locals,
                    false,
                ));
            }

            if !new_locals.is_empty() {
                changed = true;
                let mut updated_func = func.clone();
                updated_func.body = new_body;
                updated_func.locals.extend(new_locals);
                updated_funcs.insert(id, updated_func);
            }
        }

        for (id, updated_func) in updated_funcs {
            funcs.insert(id, updated_func);
        }

        if !changed {
            break;
        }
    }
}

fn inline_in_block(
    stmts: Vec<HirStmt>,
    funcs: &HashMap<u32, HirFunc>,
    func_indices: &HashMap<StringId, u32>,
    next_local: &mut u32,
    caller: &HirFunc,
    depth: usize,
    new_locals: &mut Vec<HirLocalDef>,
    in_loop: bool,
) -> Vec<HirStmt> {
    let mut result = Vec::new();
    for stmt in stmts {
        result.extend(inline_in_stmt(
            stmt,
            funcs,
            func_indices,
            next_local,
            caller,
            depth,
            new_locals,
            in_loop,
        ));
    }
    result
}

fn inline_in_stmt(
    mut stmt: HirStmt,
    funcs: &HashMap<u32, HirFunc>,
    func_indices: &HashMap<StringId, u32>,
    next_local: &mut u32,
    caller: &HirFunc,
    depth: usize,
    new_locals: &mut Vec<HirLocalDef>,
    in_loop: bool,
) -> Vec<HirStmt> {
    let mut prepended = Vec::new();

    match &mut stmt.kind {
        HirStmtKind::ExprStmt(expr) => {
            prepended = extract_calls_from_expr(expr, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
        }
        HirStmtKind::VarDecl { value, .. } => {
            if let Some(val) = value {
                prepended = extract_calls_from_expr(val, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
            }
        }
        HirStmtKind::Assign { value, .. } => {
            prepended = extract_calls_from_expr(value, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
        }
        HirStmtKind::AssignGlobal { value, .. } => {
            prepended = extract_calls_from_expr(value, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
        }
        HirStmtKind::Print(expr) => {
            prepended = extract_calls_from_expr(expr, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
        }
        HirStmtKind::TerminalWrite(expr) => {
            prepended = extract_calls_from_expr(expr, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
        }
        HirStmtKind::Return(expr) => {
            if let Some(e) = expr {
                prepended = extract_calls_from_expr(e, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
            }
        }
        HirStmtKind::Wait(expr) => {
            prepended = extract_calls_from_expr(expr, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
        }
        HirStmtKind::Yield { value, .. } => {
            prepended = extract_calls_from_expr(value, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
        }
        HirStmtKind::YieldFrom(expr) => {
            prepended = extract_calls_from_expr(expr, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
        }
        HirStmtKind::FunctionCallStmt { name, args } => {
            for arg in args.iter_mut() {
                prepended.extend(extract_calls_from_expr(
                    arg.expr_mut(),
                    funcs,
                    func_indices,
                    next_local,
                    caller,
                    depth,
                    new_locals,
                    in_loop,
                ));
            }
            if let Some(&callee_id) = func_indices.get(name) {
                if let Some(callee) = funcs.get(&callee_id) {
                    if super::inline_policy::should_inline(callee, caller, depth, in_loop) {
                        let (inline_stmts, result_local) = super::inline::inline_call_site(callee, args, *next_local);
                        for local in &callee.locals {
                            new_locals.push(local.clone());
                        }
                        new_locals.push(HirLocalDef {
                            name: callee.name,
                            ty: callee.return_type.clone().unwrap_or(crate::sema::types::Type::Int),
                            is_const: false,
                        });
                        *next_local = result_local + 1;
                        prepended.extend(inline_stmts);
                        return prepended;
                    }
                }
            }
        }
        HirStmtKind::If { condition, then_branch, else_ifs, else_branch } => {
            prepended = extract_calls_from_expr(condition, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
            *then_branch = inline_in_block(
                std::mem::take(then_branch),
                funcs,
                func_indices,
                next_local,
                caller,
                depth,
                new_locals,
                in_loop,
            );
            for (cond, branch) in else_ifs {
                let cond_prepended = extract_calls_from_expr(cond, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
                *branch = inline_in_block(
                    std::mem::take(branch),
                    funcs,
                    func_indices,
                    next_local,
                    caller,
                    depth,
                    new_locals,
                    in_loop,
                );
                prepended.extend(cond_prepended);
            }
            if let Some(branch) = else_branch {
                *branch = inline_in_block(
                    std::mem::take(branch),
                    funcs,
                    func_indices,
                    next_local,
                    caller,
                    depth,
                    new_locals,
                    in_loop,
                );
            }
        }
        HirStmtKind::While { condition, body } => {
            prepended = extract_calls_from_expr(condition, funcs, func_indices, next_local, caller, depth, new_locals, in_loop);
            *body = inline_in_block(
                std::mem::take(body),
                funcs,
                func_indices,
                next_local,
                caller,
                depth,
                new_locals,
                true,
            );
        }
        HirStmtKind::For { start, end, step, body, .. } => {
            prepended.extend(extract_calls_from_expr(start, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            prepended.extend(extract_calls_from_expr(end, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            if let Some(s) = step {
                prepended.extend(extract_calls_from_expr(s, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
            *body = inline_in_block(
                std::mem::take(body),
                funcs,
                func_indices,
                next_local,
                caller,
                depth,
                new_locals,
                true,
            );
        }
        HirStmtKind::InlineBlock { stmts, .. } => {
            *stmts = inline_in_block(
                std::mem::take(stmts),
                funcs,
                func_indices,
                next_local,
                caller,
                depth,
                new_locals,
                in_loop,
            );
        }
        // Bypasses statements containing no function calls to inline:
        // Break, Continue, YieldVoid, Input, Halt, Include, FiberDecl, DatabaseDecl
        _ => {}
    }

    if prepended.is_empty() {
        vec![stmt]
    } else {
        let mut block_stmts = prepended;
        block_stmts.push(stmt);
        block_stmts
    }
}

fn extract_calls_from_expr(
    expr: &mut HirExpr,
    funcs: &HashMap<u32, HirFunc>,
    func_indices: &HashMap<StringId, u32>,
    next_local: &mut u32,
    caller: &HirFunc,
    depth: usize,
    new_locals: &mut Vec<HirLocalDef>,
    in_loop: bool,
) -> Vec<HirStmt> {
    let mut stmts = Vec::new();
    match &mut expr.kind {
        HirExprKind::FunctionCall { name, args } => {
            for arg in args.iter_mut() {
                stmts.extend(extract_calls_from_expr(
                    arg.expr_mut(),
                    funcs,
                    func_indices,
                    next_local,
                    caller,
                    depth,
                    new_locals,
                    in_loop,
                ));
            }
            if let Some(&callee_id) = func_indices.get(name) {
                if let Some(callee) = funcs.get(&callee_id) {
                    if super::inline_policy::should_inline(callee, caller, depth, in_loop) {
                        let (inline_stmts, result_local) = super::inline::inline_call_site(callee, args, *next_local);

                        for local in &callee.locals {
                            new_locals.push(local.clone());
                        }
                        new_locals.push(HirLocalDef {
                            name: callee.name,
                            ty: callee.return_type.clone().unwrap_or(crate::sema::types::Type::Int),
                            is_const: false,
                        });

                        *next_local = result_local + 1;
                        expr.kind = HirExprKind::Local(result_local);
                        stmts.extend(inline_stmts);
                    }
                }
            }
        }
        HirExprKind::Binary { left, right, .. } => {
            stmts.extend(extract_calls_from_expr(left, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            stmts.extend(extract_calls_from_expr(right, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
        }
        HirExprKind::Unary { right, .. } => {
            stmts.extend(extract_calls_from_expr(right, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
        }
        HirExprKind::ArrayLiteral { elements } => {
            for e in elements {
                stmts.extend(extract_calls_from_expr(e, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
        }
        HirExprKind::MethodCall { receiver, args, .. } => {
            stmts.extend(extract_calls_from_expr(receiver, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            for arg in args {
                stmts.extend(extract_calls_from_expr(
                    arg.expr_mut(),
                    funcs,
                    func_indices,
                    next_local,
                    caller,
                    depth,
                    new_locals,
                    in_loop,
                ));
            }
        }
        HirExprKind::Index { receiver, index } => {
            stmts.extend(extract_calls_from_expr(receiver, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            stmts.extend(extract_calls_from_expr(index, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
        }
        HirExprKind::MemberAccess { receiver, .. } => {
            stmts.extend(extract_calls_from_expr(receiver, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
        }
        HirExprKind::SetLiteral { elements, range, .. } => {
            for e in elements {
                stmts.extend(extract_calls_from_expr(e, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
            if let Some(r) = range {
                stmts.extend(extract_calls_from_expr(&mut r.start, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
                stmts.extend(extract_calls_from_expr(&mut r.end, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
                if let Some(s) = &mut r.step {
                    stmts.extend(extract_calls_from_expr(s, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
                }
            }
        }
        HirExprKind::ArrayOrSetLiteral { elements } => {
            for e in elements {
                stmts.extend(extract_calls_from_expr(e, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
        }
        HirExprKind::RandomChoice { set } => {
            stmts.extend(extract_calls_from_expr(set, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
        }
        HirExprKind::RandomInt { min, max, step } => {
            stmts.extend(extract_calls_from_expr(min, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            stmts.extend(extract_calls_from_expr(max, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            if let Some(s) = step {
                stmts.extend(extract_calls_from_expr(s, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
        }
        HirExprKind::RandomFloat { min, max, step } => {
            stmts.extend(extract_calls_from_expr(min, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            stmts.extend(extract_calls_from_expr(max, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            if let Some(s) = step {
                stmts.extend(extract_calls_from_expr(s, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
        }
        HirExprKind::MapLiteral { elements, .. } => {
            for (k, v) in elements {
                stmts.extend(extract_calls_from_expr(k, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
                stmts.extend(extract_calls_from_expr(v, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
        }
        HirExprKind::TableLiteral { rows, .. } => {
            for row in rows {
                for e in row {
                    stmts.extend(extract_calls_from_expr(e, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
                }
            }
        }
        HirExprKind::DatabaseLiteral(elements) => {
            for (_, e) in elements {
                stmts.extend(extract_calls_from_expr(e, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
        }
        HirExprKind::TerminalCommand(_, args) => {
            for e in args {
                stmts.extend(extract_calls_from_expr(e, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
        }
        HirExprKind::Lambda { body, .. } => {
            stmts.extend(extract_calls_from_expr(body, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
        }
        HirExprKind::Tuple(elements) => {
            for e in elements {
                stmts.extend(extract_calls_from_expr(e, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
            }
        }
        HirExprKind::ModuleCall { args, .. } => {
            for arg in args {
                stmts.extend(extract_calls_from_expr(
                    arg.expr_mut(),
                    funcs,
                    func_indices,
                    next_local,
                    caller,
                    depth,
                    new_locals,
                    in_loop,
                ));
            }
        }
        HirExprKind::As { expr, .. } => {
            stmts.extend(extract_calls_from_expr(expr, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
        }
        HirExprKind::Yield(expr) => {
            stmts.extend(extract_calls_from_expr(expr, funcs, func_indices, next_local, caller, depth, new_locals, in_loop));
        }
        // Bypasses leaf node expressions that cannot contain nested expressions or function calls:
        // IntLiteral, FloatLiteral, StringLiteral, BoolLiteral, Local, Global, RawBlock, DateLiteral, Tag
        _ => {}
    }
    stmts
}
