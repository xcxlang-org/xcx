use std::collections::HashMap;
use crate::error::Span;
use crate::vm::opcode::{OpCode, Chunk, MethodKind, TypeTag};
use crate::vm::value::Value;
use crate::compiler::compiler::{FunctionCompiler, CompileContext, LoopFrame};
use crate::hir::hir::{HirFunc, HirStmt, HirStmtKind, HirLocalDef};
use crate::sema::types::Type;
use super::compile_expr::compile_expr;

pub fn compile_hir_to_chunk(
    func: &HirFunc,
    is_fiber: bool,
    ctx: &mut CompileContext,
    name: String,
    param_count: usize,
) -> Chunk {
    let mut compiler = FunctionCompiler::new(false, None);
    compiler.next_local = func.locals.len();
    compiler.max_locals_used = func.locals.len();

    let mut local_map = HashMap::new();
    for (i, def) in func.locals.iter().enumerate() {
        local_map.insert(def.name, i);
        compiler.local_regs.insert(i);
    }
    compiler.scopes = vec![local_map];

    super::compile_expr::init_capture_map(&compiler);

    for stmt in &func.body {
        compile_stmt(&mut compiler, stmt, ctx, &func.locals);
    }

    if !compiler.bytecode.last().map_or(false, |op| {
        matches!(op, OpCode::Return { .. } | OpCode::ReturnVoid)
    }) {
        compiler.emit(OpCode::ReturnVoid, &Span::default());
    }

    let has_loops = crate::vm::opcode::calculate_has_loops(&compiler.bytecode);
    let max_locals = compiler.max_locals_used.max(compiler.next_local);

    Chunk::new(
        compiler.bytecode,
        compiler.spans,
        is_fiber,
        max_locals.into(),
        has_loops,
        name,
        param_count,
    )
}

fn compile_stmt(
    compiler: &mut FunctionCompiler,
    stmt: &HirStmt,
    ctx: &mut CompileContext,
    func_locals: &[HirLocalDef],
) {
    let entry_next_local = compiler.next_local;
    match &stmt.kind {
        HirStmtKind::VarDecl { local, value } => {
            compiler.local_regs.insert(*local as usize);
            if let Some(val) = value {
                let src = compile_expr(compiler, val, ctx);
                let dst = *local as u8;
                if src != dst {
                    compiler.emit(OpCode::Move { dst, src }, &stmt.span);
                }
                compiler.pop_reg();
            } else {
                let local_def = &func_locals[*local as usize];
                let dst = *local as u8;
                match &local_def.ty {
                    Type::Array(elem_ty) => {
                        if **elem_ty == Type::Bool {
                            compiler.emit(OpCode::BoolArrayInit { dst }, &stmt.span);
                        } else {
                            compiler.emit(OpCode::ArrayInit { dst, base: dst, count: 0 }, &stmt.span);
                        }
                    }
                    Type::Set(_) => {
                        compiler.emit(OpCode::SetInit { dst, base: dst, count: 0 }, &stmt.span);
                    }
                    Type::Map(_, _) => {
                        compiler.emit(OpCode::MapInit { dst, base: dst, count: 0 }, &stmt.span);
                    }
                    Type::Table(cols) => {
                        let vm_cols = cols.iter().map(|c| crate::vm::object::VMColumn {
                            name: ctx.interner.lookup(c.name).to_string(),
                            ty: c.ty.clone(),
                            is_auto: c.is_auto,
                            is_pk: c.is_pk,
                            is_unique: c.is_unique,
                        }).collect();
                        let skeleton_idx = ctx.add_constant(Value::from_table(std::sync::Arc::new(parking_lot::RwLock::new(
                            crate::vm::object::TableObj { 
                                table_name: String::new(), 
                                columns: vm_cols, 
                                rows: Vec::new(), 
                                sql_binding: None, 
                                sql_where: None, 
                                pending_op: None 
                            }
                        ))));
                        compiler.emit(OpCode::TableInit { dst, skeleton_idx, base: dst, row_count: 0, col_count: cols.len() as u32 }, &stmt.span);
                    }
                    _ => {
                        let def = compiler.get_default_value(&local_def.ty, ctx);
                        let idx = ctx.add_constant(def);
                        compiler.emit(OpCode::LoadConst { dst, idx }, &stmt.span);
                    }
                }
            }
        }
        HirStmtKind::Print(expr) => {
            let src = compile_expr(compiler, expr, ctx);
            compiler.emit(OpCode::Print { src }, &stmt.span);
            compiler.pop_reg();
        }
        HirStmtKind::TerminalWrite(expr) => {
            let src = compile_expr(compiler, expr, ctx);
            let dst = compiler.push_reg();
            compiler.emit(OpCode::TerminalWrite { dst, src }, &stmt.span);
            compiler.pop_reg();
            compiler.pop_reg();
        }
        HirStmtKind::Input(local, ty) => {
            compiler.local_regs.insert(*local as usize);
            let dst = *local as u8;
            let type_tag = match **ty {
                Type::Int => TypeTag::Int,
                Type::Float => TypeTag::Float,
                Type::String => TypeTag::String,
                Type::Bool => TypeTag::Bool,
                Type::Array(_) |
                Type::Set(_) |
                Type::Map(_, _) |
                Type::Date |
                Type::Table(_) |
                Type::Database |
                Type::DatabaseOperation(_, _) |
                Type::Json |
                Type::Builtin(_) |
                Type::Fiber(_) |
                Type::Unknown => TypeTag::Unknown,
            };
            compiler.emit(OpCode::Input { dst, ty: type_tag }, &stmt.span);
        }
        HirStmtKind::ExprStmt(expr) => {
            let mut optimized = false;
            if let crate::hir::hir::HirExprKind::MethodCall { receiver, method, args, wait_after } = &expr.kind {
                let method_name = ctx.interner.lookup(*method);
                if method_name == "set" && args.len() == 2 && !*wait_after {
                    let key_expr = match &args[0] {
                        crate::hir::hir::HirArgument::Positional(e) => e,
                        crate::hir::hir::HirArgument::Named(_, e) => e,
                    };
                    let val_expr = match &args[1] {
                        crate::hir::hir::HirArgument::Positional(e) => e,
                        crate::hir::hir::HirArgument::Named(_, e) => e,
                    };
                    if let crate::hir::hir::HirExprKind::StringLiteral(key_id) = &key_expr.kind {
                        if let crate::hir::hir::HirExprKind::Binary { left, op: crate::hir::hir::HirBinOp::Add, right } = &val_expr.kind {
                            if let crate::hir::hir::HirExprKind::MethodCall { receiver: get_recv, method: get_method, args: get_args, wait_after: false } = &left.kind {
                                if ctx.interner.lookup(*get_method) == "get" && get_args.len() == 1 {
                                    let get_key_expr = match &get_args[0] {
                                        crate::hir::hir::HirArgument::Positional(e) => e,
                                        crate::hir::hir::HirArgument::Named(_, e) => e,
                                    };
                                    if let crate::hir::hir::HirExprKind::StringLiteral(get_key_id) = &get_key_expr.kind {
                                        if get_key_id == key_id {
                                            let same_receiver = match (&receiver.kind, &get_recv.kind) {
                                                (crate::hir::hir::HirExprKind::Local(l1), crate::hir::hir::HirExprKind::Local(l2)) => l1 == l2,
                                                (crate::hir::hir::HirExprKind::Global(g1), crate::hir::hir::HirExprKind::Global(g2)) => g1 == g2,
                                                _ => false,
                                            };
                                            if same_receiver && receiver.ty == Type::Json {
                                                let saved_next_local = compiler.next_local;
                                                let container_reg = compile_expr(compiler, receiver, ctx);
                                                let rhs_reg = compile_expr(compiler, right, ctx);
                                                let key_str = ctx.interner.lookup(*key_id).to_string();
                                                let name_idx = ctx.add_constant(Value::from_string(std::sync::Arc::new(
                                                    crate::vm::object::StringObj::new(key_str.into_bytes())
                                                )));
                                                compiler.emit(OpCode::StrAppendMember {
                                                    container: container_reg,
                                                    name_idx,
                                                    src: rhs_reg,
                                                }, &stmt.span);
                                                compiler.next_local = saved_next_local;
                                                compiler.sync_max_locals();
                                                optimized = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if method_name == "update" && args.len() == 2 && !*wait_after {
                    let idx_expr = match &args[0] {
                        crate::hir::hir::HirArgument::Positional(e) => e,
                        crate::hir::hir::HirArgument::Named(_, e) => e,
                    };
                    let val_expr = match &args[1] {
                        crate::hir::hir::HirArgument::Positional(e) => e,
                        crate::hir::hir::HirArgument::Named(_, e) => e,
                    };
                    if let crate::hir::hir::HirExprKind::Binary { left, op: crate::hir::hir::HirBinOp::Add, right } = &val_expr.kind {
                        if let crate::hir::hir::HirExprKind::MethodCall { receiver: get_recv, method: get_method, args: get_args, wait_after: false } = &left.kind {
                            if ctx.interner.lookup(*get_method) == "get" && get_args.len() == 1 {
                                let get_idx_expr = match &get_args[0] {
                                    crate::hir::hir::HirArgument::Positional(e) => e,
                                    crate::hir::hir::HirArgument::Named(_, e) => e,
                                };
                                let idx_match = match (&idx_expr.kind, &get_idx_expr.kind) {
                                    (crate::hir::hir::HirExprKind::Local(l1), crate::hir::hir::HirExprKind::Local(l2)) => l1 == l2,
                                    (crate::hir::hir::HirExprKind::Global(g1), crate::hir::hir::HirExprKind::Global(g2)) => g1 == g2,
                                    (crate::hir::hir::HirExprKind::IntLiteral(n1), crate::hir::hir::HirExprKind::IntLiteral(n2)) => n1 == n2,
                                    _ => false,
                                };
                                if idx_match {
                                    let same_receiver = match (&receiver.kind, &get_recv.kind) {
                                        (crate::hir::hir::HirExprKind::Local(l1), crate::hir::hir::HirExprKind::Local(l2)) => l1 == l2,
                                        (crate::hir::hir::HirExprKind::Global(g1), crate::hir::hir::HirExprKind::Global(g2)) => g1 == g2,
                                        _ => false,
                                    };
                                    let is_string_array = match &receiver.ty {
                                        Type::Array(inner) => matches!(**inner, Type::String),
                                        _ => false,
                                    };
                                    if same_receiver && is_string_array {
                                        let saved_next_local = compiler.next_local;
                                        let container_reg = compile_expr(compiler, receiver, ctx);
                                        let index_reg = compile_expr(compiler, idx_expr, ctx);
                                        let rhs_reg = compile_expr(compiler, right, ctx);
                                        compiler.emit(OpCode::StrAppendElement {
                                            container: container_reg,
                                            index: index_reg,
                                            src: rhs_reg,
                                        }, &stmt.span);
                                        compiler.next_local = saved_next_local;
                                        compiler.sync_max_locals();
                                        optimized = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !optimized {
                compile_expr(compiler, expr, ctx);
                compiler.pop_reg();
            }
        }
        HirStmtKind::Assign { local, value } => {
            let mut optimized = false;
            let dst = *local as u8;
            if let crate::hir::hir::HirExprKind::Binary { left, op, right } = &value.kind {
                if *op == crate::hir::hir::HirBinOp::Add {
                    let is_inc = match (&left.kind, &right.kind) {
                        (crate::hir::hir::HirExprKind::Local(l), crate::hir::hir::HirExprKind::IntLiteral(1)) if *l == *local => true,
                        (crate::hir::hir::HirExprKind::IntLiteral(1), crate::hir::hir::HirExprKind::Local(l)) if *l == *local => true,
                        _ => false,
                    };
                    if is_inc {
                        compiler.emit(OpCode::IncLocal { reg: dst }, &stmt.span);
                        optimized = true;
                    } else {
                        let mut chain = Vec::new();
                        let is_str_append = collect_hir_concat_local(value, *local, &mut chain);
                        if is_str_append && !chain.iter().any(|e| hir_expr_contains_local(e, *local)) {
                            let local_def = &func_locals[*local as usize];
                            if local_def.ty == Type::String {
                                chain.reverse();
                                for right_expr in chain {
                                    let src = compile_expr(compiler, right_expr, ctx);
                                    compiler.emit(OpCode::StrAppendLocal { local_idx: dst, src }, &stmt.span);
                                    compiler.pop_reg();
                                }
                                optimized = true;
                            }
                        }
                    }
                } else if *op == crate::hir::hir::HirBinOp::Sub {
                    let is_dec = match (&left.kind, &right.kind) {
                        (crate::hir::hir::HirExprKind::Local(l), crate::hir::hir::HirExprKind::IntLiteral(1)) if *l == *local => true,
                        _ => false,
                    };
                    if is_dec {
                        compiler.emit(OpCode::DecLocal { reg: dst }, &stmt.span);
                        optimized = true;
                    }
                }
            }
            if !optimized {
                let src = compile_expr(compiler, value, ctx);
                if src != dst {
                    compiler.emit(OpCode::Move { dst, src }, &stmt.span);
                }
                compiler.pop_reg();
            }
        }
        HirStmtKind::AssignGlobal { name, value } => {
            let mut optimized = false;
            let idx = ctx.globals.get(name).copied().unwrap_or(0);
            if let crate::hir::hir::HirExprKind::Binary { left, op, right } = &value.kind {
                if *op == crate::hir::hir::HirBinOp::Add {
                    let is_inc = match (&left.kind, &right.kind) {
                        (crate::hir::hir::HirExprKind::Global(n), crate::hir::hir::HirExprKind::IntLiteral(1)) if n == name => true,
                        (crate::hir::hir::HirExprKind::IntLiteral(1), crate::hir::hir::HirExprKind::Global(n)) if n == name => true,
                        _ => false,
                    };
                    if is_inc {
                        compiler.emit(OpCode::IncVar { idx: idx as u32 }, &stmt.span);
                        optimized = true;
                    } else {
                        let mut chain = Vec::new();
                        let is_str_append = collect_hir_concat_global(value, *name, &mut chain);
                        if is_str_append && !chain.iter().any(|e| hir_expr_contains_global(e, *name)) {
                            if let Some(Type::String) = ctx.global_types.get(name) {
                                chain.reverse();
                                for right_expr in chain {
                                    let src = compile_expr(compiler, right_expr, ctx);
                                    compiler.emit(OpCode::StrAppendVar { var_idx: idx as u32, src }, &stmt.span);
                                    compiler.pop_reg();
                                }
                                optimized = true;
                            }
                        }
                    }
                } else if *op == crate::hir::hir::HirBinOp::Sub {
                    let is_dec = match (&left.kind, &right.kind) {
                        (crate::hir::hir::HirExprKind::Global(n), crate::hir::hir::HirExprKind::IntLiteral(1)) if n == name => true,
                        _ => false,
                    };
                    if is_dec {
                        compiler.emit(OpCode::DecVar { idx: idx as u32 }, &stmt.span);
                        optimized = true;
                    }
                }
            }
            if !optimized {
                let src = compile_expr(compiler, value, ctx);
                compiler.emit(OpCode::SetVar { idx: idx as u32, src }, &stmt.span);
                compiler.pop_reg();
            }
        }
        HirStmtKind::Halt { level, message } => {
            let src = compile_expr(compiler, message, ctx);
            match level {
                crate::frontend::ast::HaltLevel::Alert => compiler.emit(OpCode::HaltAlert { src }, &stmt.span),
                crate::frontend::ast::HaltLevel::Error => compiler.emit(OpCode::HaltError { src }, &stmt.span),
                crate::frontend::ast::HaltLevel::Fatal => compiler.emit(OpCode::HaltFatal { src }, &stmt.span),
            }
            compiler.pop_reg();
        }
        HirStmtKind::Return(expr) => {
            if !compiler.inline_result_locals.is_empty() {
                let target_reg = *compiler.inline_result_locals.last().unwrap();
                if let Some(e) = expr {
                    let src = compile_expr(compiler, e, ctx);
                    if let Some(dst) = target_reg {
                        if src != dst {
                            compiler.emit(OpCode::Move { dst, src }, &stmt.span);
                        }
                    }
                    compiler.pop_reg();
                }
                let jmp = compiler.bytecode.len();
                compiler.emit(OpCode::Jump { target: 0 }, &stmt.span);
                let loop_idx = *compiler.inline_stack.last().unwrap();
                compiler.loop_stack[loop_idx].breaks.push(jmp);
            } else {
                if let Some(e) = expr {
                    let src = compile_expr(compiler, e, ctx);
                    compiler.emit(OpCode::Return { src }, &stmt.span);
                    compiler.pop_reg();
                } else {
                    compiler.emit(OpCode::ReturnVoid, &stmt.span);
                }
            }
        }
        HirStmtKind::FunctionCallStmt { name, args } => {
            let base = compiler.next_local as u8;
            for (i, arg) in args.iter().enumerate() {
                let dst = (base as usize + i) as u8;
                let src = compile_expr(compiler, arg.expr(), ctx);
                if src != dst {
                    compiler.emit(OpCode::Move { dst, src }, &stmt.span);
                }
                compiler.next_local = dst as usize + 1;
                compiler.sync_max_locals();
            }
            if let Some(&func_id) = ctx.func_indices.get(name) {
                let dst = base;
                compiler.emit(OpCode::Call { dst, func_idx: func_id as u32, base, arg_count: args.len() as u8 }, &stmt.span);
            }
            compiler.next_local = base as usize;
        }
        HirStmtKind::If { condition, then_branch, else_ifs, else_branch } => {
            let mut end_jumps = Vec::new();
            let cond_reg = compile_expr(compiler, condition, ctx);
            let jmp_idx = compiler.bytecode.len();
            compiler.emit(OpCode::JumpIfFalse { src: cond_reg, target: 0 }, &stmt.span);
            compiler.pop_reg();

            for s in then_branch {
                compile_stmt(compiler, s, ctx, func_locals);
            }

            if !else_ifs.is_empty() || else_branch.is_some() {
                end_jumps.push(compiler.bytecode.len());
                compiler.emit(OpCode::Jump { target: 0 }, &stmt.span);
            }
            let jump_target = compiler.bytecode.len() as u32;
            if let OpCode::JumpIfFalse { ref mut target, .. } = compiler.bytecode[jmp_idx] {
                *target = jump_target;
            }

            for (elif_cond, elif_branch) in else_ifs {
                let elif_cond_reg = compile_expr(compiler, elif_cond, ctx);
                let elif_jmp = compiler.bytecode.len();
                compiler.emit(OpCode::JumpIfFalse { src: elif_cond_reg, target: 0 }, &stmt.span);
                compiler.pop_reg();

                for s in elif_branch {
                    compile_stmt(compiler, s, ctx, func_locals);
                }
                end_jumps.push(compiler.bytecode.len());
                compiler.emit(OpCode::Jump { target: 0 }, &stmt.span);
                let elif_target = compiler.bytecode.len() as u32;
                if let OpCode::JumpIfFalse { ref mut target, .. } = compiler.bytecode[elif_jmp] {
                    *target = elif_target;
                }
            }
            if let Some(branch) = else_branch {
                for s in branch {
                    compile_stmt(compiler, s, ctx, func_locals);
                }
            }
            let final_idx = compiler.bytecode.len() as u32;
            for idx in end_jumps {
                if let OpCode::Jump { ref mut target } = compiler.bytecode[idx] {
                    *target = final_idx;
                }
            }
        }
        HirStmtKind::While { condition, body } => {
            let mut is_opt_candidate = false;
            let mut loop_counter_local = None;
            let mut limit_expr = None;
            let mut is_less = false;
            let mut is_greater = false;
            let mut is_greater_equal = false;

            if let crate::hir::hir::HirExprKind::Binary { left, op, right } = &condition.kind {
                if *op == crate::hir::hir::HirBinOp::Less || *op == crate::hir::hir::HirBinOp::LessEqual {
                    if let crate::hir::hir::HirExprKind::Local(l) = &left.kind {
                        loop_counter_local = Some(*l);
                        limit_expr = Some(right.as_ref());
                        is_less = *op == crate::hir::hir::HirBinOp::Less;
                        is_opt_candidate = true;
                    }
                } else if *op == crate::hir::hir::HirBinOp::Greater || *op == crate::hir::hir::HirBinOp::GreaterEqual {
                    if let crate::hir::hir::HirExprKind::Local(l) = &left.kind {
                        loop_counter_local = Some(*l);
                        limit_expr = Some(right.as_ref());
                        is_greater = *op == crate::hir::hir::HirBinOp::Greater;
                        is_greater_equal = *op == crate::hir::hir::HirBinOp::GreaterEqual;
                        is_opt_candidate = true;
                    }
                }
            }

            compiler.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: None });
            let is_downward = is_greater || is_greater_equal;

            if is_opt_candidate {
                let counter_reg = loop_counter_local.unwrap() as u8;
                let original_limit_reg = compile_expr(compiler, limit_expr.unwrap(), ctx);

                let limit_reg = if is_less {
                    let one_idx = ctx.add_constant(Value::from_i64(1));
                    let one_reg = compiler.push_reg();
                    compiler.emit(OpCode::LoadConst { dst: one_reg, idx: one_idx }, &stmt.span);
                    compiler.emit(OpCode::Sub { dst: original_limit_reg, src1: original_limit_reg, src2: one_reg }, &stmt.span);
                    compiler.pop_reg(); // free one_reg
                    original_limit_reg
                } else if is_greater {
                    let one_idx = ctx.add_constant(Value::from_i64(1));
                    let one_reg = compiler.push_reg();
                    compiler.emit(OpCode::LoadConst { dst: one_reg, idx: one_idx }, &stmt.span);
                    compiler.emit(OpCode::Add { dst: original_limit_reg, src1: original_limit_reg, src2: one_reg }, &stmt.span);
                    compiler.pop_reg(); // free one_reg
                    original_limit_reg
                } else {
                    original_limit_reg
                };
                compiler.next_local = compiler.next_local.max(limit_reg as usize + 1);

                let start_p = compiler.bytecode.len();
                if let Some(l) = compiler.loop_stack.last_mut() { l.start_pc = start_p; }

                let test_reg = compiler.push_reg();
                if is_downward {
                    compiler.emit(OpCode::GreaterEqual { dst: test_reg, src1: counter_reg, src2: limit_reg }, &stmt.span);
                } else {
                    compiler.emit(OpCode::LessEqual { dst: test_reg, src1: counter_reg, src2: limit_reg }, &stmt.span);
                }
                let exit_jmp = compiler.bytecode.len();
                compiler.emit(OpCode::JumpIfFalse { src: test_reg, target: 0 }, &stmt.span);
                compiler.pop_reg(); // free test_reg

                let body_p = compiler.bytecode.len();
                for s in body { compile_stmt(compiler, s, ctx, func_locals); }

                let len = compiler.bytecode.len();
                let mut fused = false;
                if len > 0 {
                    if !is_downward {
                        if let OpCode::IncLocal { reg } = compiler.bytecode[len - 1] {
                            if reg == counter_reg {
                                compiler.bytecode.pop();
                                compiler.spans.pop();
                                compiler.emit(OpCode::LoopNext { reg: counter_reg, limit_reg, target: body_p as u32 }, &stmt.span);
                                fused = true;
                            }
                        }
                    } else {
                        if let OpCode::DecLocal { reg } = compiler.bytecode[len - 1] {
                            if reg == counter_reg {
                                compiler.bytecode.pop();
                                compiler.spans.pop();
                                compiler.emit(OpCode::LoopPrev { reg: counter_reg, limit_reg, target: body_p as u32 }, &stmt.span);
                                fused = true;
                            }
                        }
                    }
                }

                if !fused {
                    compiler.emit(OpCode::Jump { target: start_p as u32 }, &stmt.span);
                }

                let exit_target = compiler.bytecode.len() as u32;
                if let OpCode::JumpIfFalse { ref mut target, .. } = compiler.bytecode[exit_jmp] { *target = exit_target; }

                let frame = compiler.loop_stack.pop().unwrap();
                let breaks = frame.breaks;
                let continues = frame.continues;
                let end_label = compiler.bytecode.len() as u32;
                for b in breaks { if let OpCode::Jump { ref mut target } = compiler.bytecode[b] { *target = end_label; } }
                for c in continues { if let OpCode::Jump { ref mut target } = compiler.bytecode[c] { *target = start_p as u32; } }

                compiler.pop_reg(); // free limit_reg
            } else {
                let start_p = compiler.bytecode.len();
                if let Some(l) = compiler.loop_stack.last_mut() { l.start_pc = start_p; }
                let cond_reg = compile_expr(compiler, condition, ctx);
                let exit_jmp = compiler.bytecode.len();
                compiler.emit(OpCode::JumpIfFalse { src: cond_reg, target: 0 }, &stmt.span);
                compiler.pop_reg();
                for s in body { compile_stmt(compiler, s, ctx, func_locals); }
                compiler.emit(OpCode::Jump { target: start_p as u32 }, &stmt.span);
                let exit_target = compiler.bytecode.len() as u32;
                if let OpCode::JumpIfFalse { ref mut target, .. } = compiler.bytecode[exit_jmp] { *target = exit_target; }
                let frame = compiler.loop_stack.pop().unwrap();
                let breaks = frame.breaks;
                let continues = frame.continues;
                let end_label = compiler.bytecode.len() as u32;
                for b in breaks { if let OpCode::Jump { ref mut target } = compiler.bytecode[b] { *target = end_label; } }
                for c in continues { if let OpCode::Jump { ref mut target } = compiler.bytecode[c] { *target = start_p as u32; } }
            }
        }
        HirStmtKind::For { local, start, end, step, body, iter_type } => {
            compiler.local_regs.insert(*local as usize);
            match iter_type {
                crate::frontend::ast::stmt::ForIterType::Range => {
                    let start_reg_raw = compile_expr(compiler, start, ctx);
                    compiler.pop_reg();
                    let loop_var_reg = *local as u8;
                    compiler.emit(OpCode::Move { dst: loop_var_reg, src: start_reg_raw }, &stmt.span);

                    let limit_reg = compile_expr(compiler, end, ctx);
                    compiler.next_local = compiler.next_local.max(limit_reg as usize + 1);
                    compiler.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: None });
                    let start_p = compiler.bytecode.len();
                    if let Some(l) = compiler.loop_stack.last_mut() {
                        l.start_pc = start_p;
                    }
                    let saved_next_local = compiler.next_local;
                    let test_reg = compiler.push_reg();
                    compiler.emit(OpCode::LessEqual { dst: test_reg, src1: loop_var_reg, src2: limit_reg }, &stmt.span);
                    let exit_jmp = compiler.bytecode.len();
                    compiler.emit(OpCode::JumpIfFalse { src: test_reg, target: 0 }, &stmt.span);
                    compiler.pop_reg();

                    let body_p = compiler.bytecode.len();
                    let mut last_stmt_start = body_p;
                    let body_len = body.len();
                    for (i, s) in body.iter().enumerate() {
                        if i == body_len - 1 { last_stmt_start = compiler.bytecode.len(); }
                        compile_stmt(compiler, s, ctx, func_locals);
                    }
                    compiler.next_local = saved_next_local;
                    let mut cont_label = compiler.bytecode.len();
                    if step.is_none() {
                        let len = compiler.bytecode.len();
                        let mut fused = false;
                        let has_continues = if let Some(l) = compiler.loop_stack.last() {
                            !l.continues.is_empty()
                        } else {
                            false
                        };
                        if len == last_stmt_start + 1 && !has_continues {
                            match compiler.bytecode[len - 1] {
                                OpCode::IncVar { idx } => {
                                    compiler.bytecode.pop(); compiler.spans.pop();
                                    cont_label = compiler.bytecode.len();
                                    compiler.emit(OpCode::IncVarLoopNext { g_idx: idx, reg: loop_var_reg, limit_reg, target: body_p as u32 }, &stmt.span);
                                    fused = true;
                                }
                                OpCode::IncLocal { reg } => {
                                    compiler.bytecode.pop(); compiler.spans.pop();
                                    cont_label = compiler.bytecode.len();
                                    compiler.emit(OpCode::IncLocalLoopNext { inc_reg: reg, reg: loop_var_reg, limit_reg, target: body_p as u32 }, &stmt.span);
                                    fused = true;
                                }
                                _ => {}
                            }
                        }
                        if !fused { compiler.emit(OpCode::LoopNext { reg: loop_var_reg, limit_reg, target: body_p as u32 }, &stmt.span); }
                    } else {
                        let step_reg = compile_expr(compiler, step.as_ref().unwrap(), ctx);
                        compiler.emit(OpCode::Add { dst: loop_var_reg, src1: loop_var_reg, src2: step_reg }, &stmt.span);
                        compiler.emit(OpCode::Jump { target: start_p as u32 }, &stmt.span);
                        compiler.pop_reg();
                    }
                    let end_label = compiler.bytecode.len() as u32;
                    if let OpCode::JumpIfFalse { ref mut target, .. } = compiler.bytecode[exit_jmp] {
                        *target = end_label;
                    }
                    let frame = compiler.loop_stack.pop().unwrap();
                    let breaks = frame.breaks;
                    let continues = frame.continues;
                    for b in breaks {
                        if let OpCode::Jump { ref mut target } = compiler.bytecode[b] {
                            *target = end_label;
                        }
                    }
                    for c in continues {
                        if let OpCode::Jump { ref mut target } = compiler.bytecode[c] {
                            *target = cont_label as u32;
                        }
                    }
                    compiler.pop_reg();
                }
                crate::frontend::ast::stmt::ForIterType::Array | crate::frontend::ast::stmt::ForIterType::Set => {
                    let source_reg_raw = compile_expr(compiler, start, ctx);
                    compiler.pop_reg();
                    let receiver_reg = compiler.push_reg();
                    if *iter_type == crate::frontend::ast::stmt::ForIterType::Set {
                        compiler.emit(OpCode::MethodCall { dst: receiver_reg, kind: MethodKind::Values, base: source_reg_raw, arg_count: 0 }, &stmt.span);
                    } else {
                        compiler.emit(OpCode::Move { dst: receiver_reg, src: source_reg_raw }, &stmt.span);
                    }
                    let arg_reg = compiler.push_reg();
                    let size_reg = compiler.push_reg();
                    let index_reg = compiler.push_reg();
                    compiler.emit(OpCode::MethodCall { dst: size_reg, kind: MethodKind::Size, base: receiver_reg, arg_count: 0 }, &stmt.span);
                    let zero_idx = ctx.add_constant(Value::from_i64(0));
                    compiler.emit(OpCode::LoadConst { dst: index_reg, idx: zero_idx }, &stmt.span);
                    let loop_var_reg = *local as u8;
                    compiler.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: None });
                    let start_label = compiler.bytecode.len();
                    if let Some(l) = compiler.loop_stack.last_mut() {
                        l.start_pc = start_label;
                    }
                    let saved_next_local = compiler.next_local;
                    let test_reg = compiler.push_reg();
                    compiler.emit(OpCode::Less { dst: test_reg, src1: index_reg, src2: size_reg }, &stmt.span);
                    let exit_jmp = compiler.bytecode.len();
                    compiler.emit(OpCode::JumpIfFalse { src: test_reg, target: 0 }, &stmt.span);
                    compiler.pop_reg();
                    let body_start = compiler.bytecode.len();
                    compiler.emit(OpCode::Move { dst: arg_reg, src: index_reg }, &stmt.span);
                    if *iter_type == crate::frontend::ast::stmt::ForIterType::Set {
                        compiler.emit(OpCode::GetIndex { dst: loop_var_reg, container: receiver_reg, index: arg_reg }, &stmt.span);
                    } else {
                        compiler.emit(OpCode::MethodCall { dst: loop_var_reg, kind: MethodKind::Get, base: receiver_reg, arg_count: 1 }, &stmt.span);
                    }
                    for s in body {
                        compile_stmt(compiler, s, ctx, func_locals);
                    }
                    compiler.next_local = saved_next_local;
                    let cont_label = compiler.bytecode.len();
                    compiler.emit(OpCode::ArrayLoopNext { idx_reg: index_reg, size_reg, target: body_start as u32 }, &stmt.span);
                    let end_label = compiler.bytecode.len() as u32;
                    if let OpCode::JumpIfFalse { ref mut target, .. } = compiler.bytecode[exit_jmp] {
                        *target = end_label;
                    }
                    let frame = compiler.loop_stack.pop().unwrap();
                    let breaks = frame.breaks;
                    let continues = frame.continues;
                    for b in breaks {
                        if let OpCode::Jump { ref mut target } = compiler.bytecode[b] {
                            *target = end_label;
                        }
                    }
                    for c in continues {
                        if let OpCode::Jump { ref mut target } = compiler.bytecode[c] {
                            *target = cont_label as u32;
                        }
                    }
                    compiler.next_local = receiver_reg as usize;
                }
                crate::frontend::ast::stmt::ForIterType::Fiber => {
                    let fiber_reg = compile_expr(compiler, start, ctx);
                    let loop_var_reg = *local as u8;
                    compiler.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: Some(fiber_reg as usize) });
                    let start_label = compiler.bytecode.len();
                    if let Some(l) = compiler.loop_stack.last_mut() {
                        l.start_pc = start_label;
                    }
                    let saved_next_local = compiler.next_local;
                    let test_reg = compiler.push_reg();
                    compiler.emit(OpCode::MethodCall { dst: test_reg, kind: MethodKind::IsDone, base: fiber_reg, arg_count: 0 }, &stmt.span);
                    let exit_jmp = compiler.bytecode.len();
                    compiler.emit(OpCode::JumpIfTrue { src: test_reg, target: 0 }, &stmt.span);
                    compiler.pop_reg();
                    compiler.emit(OpCode::MethodCall { dst: loop_var_reg, kind: MethodKind::Next, base: fiber_reg, arg_count: 0 }, &stmt.span);
                    for s in body {
                        compile_stmt(compiler, s, ctx, func_locals);
                    }
                    compiler.next_local = saved_next_local;
                    let cont_label = compiler.bytecode.len();
                    compiler.emit(OpCode::Jump { target: start_label as u32 }, &stmt.span);
                    let end_label = compiler.bytecode.len() as u32;
                    if let OpCode::JumpIfTrue { ref mut target, .. } = compiler.bytecode[exit_jmp] {
                        *target = end_label;
                    }
                    let frame = compiler.loop_stack.pop().unwrap();
                    let breaks = frame.breaks;
                    let continues = frame.continues;
                    for b in breaks {
                        if let OpCode::Jump { ref mut target } = compiler.bytecode[b] {
                            *target = end_label;
                        }
                    }
                    for c in continues {
                        if let OpCode::Jump { ref mut target } = compiler.bytecode[c] {
                            *target = cont_label as u32;
                        }
                    }
                    compiler.next_local = fiber_reg as usize;
                }
            }
        }
        HirStmtKind::Break => {
            if let Some(&LoopFrame { fiber_reg: Some(fiber_reg_idx), .. }) = compiler.loop_stack.last() {
                let tmp = compiler.push_reg();
                compiler.emit(OpCode::MethodCall { dst: tmp, kind: MethodKind::Close, base: fiber_reg_idx as u8, arg_count: 0 }, &stmt.span);
                compiler.pop_reg();
            }
            let jmp = compiler.bytecode.len();
            compiler.emit(OpCode::Jump { target: 0 }, &stmt.span);
            if let Some(l) = compiler.loop_stack.last_mut() {
                l.breaks.push(jmp);
            }
        }
        HirStmtKind::Continue => {
            let jmp = compiler.bytecode.len();
            compiler.emit(OpCode::Jump { target: 0 }, &stmt.span);
            if let Some(l) = compiler.loop_stack.last_mut() {
                l.continues.push(jmp);
            }
        }
        HirStmtKind::JsonBind { json, path, target } => {
            compiler.local_regs.insert(*target as usize);
            let saved_next_local = compiler.next_local;
            let json_src = compile_expr(compiler, json, ctx);
            let path_src = compile_expr(compiler, path, ctx);
            let dst = *target as u8;
            compiler.emit(OpCode::JsonBindLocal { dst, json_src, path_src }, &stmt.span);
            compiler.next_local = saved_next_local;
        }
        HirStmtKind::JsonBindGlobal { json, path, target } => {
            let saved_next_local = compiler.next_local;
            let json_src = compile_expr(compiler, json, ctx);
            let path_src = compile_expr(compiler, path, ctx);
            let idx = ctx.globals.get(target).copied().unwrap_or(0);
            compiler.emit(OpCode::JsonBind { idx: idx as u32, json_src, path_src }, &stmt.span);
            compiler.next_local = saved_next_local;
        }
        HirStmtKind::JsonInject { json, mapping, table } => {
            let saved_next_local = compiler.next_local;
            let json_src = compile_expr(compiler, json, ctx);
            let mapping_src = compile_expr(compiler, mapping, ctx);
            let idx = ctx.globals.get(table).copied().unwrap_or(0);
            compiler.emit(OpCode::JsonInject { table_idx: idx as u32, json_src, mapping_src }, &stmt.span);
            compiler.next_local = saved_next_local;
        }
        HirStmtKind::JsonInjectLocal { json, mapping, table } => {
            compiler.local_regs.insert(*table as usize);
            let saved_next_local = compiler.next_local;
            let json_src = compile_expr(compiler, json, ctx);
            let mapping_src = compile_expr(compiler, mapping, ctx);
            let table_reg = *table as u8;
            compiler.emit(OpCode::JsonInjectLocal { table_reg, json_src, mapping_src }, &stmt.span);
            compiler.next_local = saved_next_local;
        }
        HirStmtKind::Wait(expr) => {
            let src = compile_expr(compiler, expr, ctx);
            compiler.emit(OpCode::Wait { src }, &stmt.span);
            compiler.pop_reg();
        }
        HirStmtKind::InlineBlock { stmts, result_local } => {
            let loop_idx = compiler.loop_stack.len();
            compiler.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: None });
            compiler.inline_stack.push(loop_idx);
            compiler.inline_result_locals.push(result_local.map(|r| r as u8));
            for s in stmts {
                compile_stmt(compiler, s, ctx, func_locals);
            }
            let end_label = compiler.bytecode.len() as u32;
            compiler.inline_result_locals.pop();
            compiler.inline_stack.pop();
            let frame = compiler.loop_stack.pop().unwrap();
            let breaks = frame.breaks;
            for b in breaks {
                if let OpCode::Jump { ref mut target } = compiler.bytecode[b] {
                    *target = end_label;
                }
            }
        }
        HirStmtKind::FiberDecl { inner_type: _, target, fiber_name, args } => {
            compiler.local_regs.insert(*target as usize);
            let fid = ctx.func_indices.get(fiber_name).copied().unwrap_or(0);
            let base = compiler.next_local as u8;
            for (i, arg) in args.iter().enumerate() {
                let dst = base + i as u8;
                let src = compile_expr(compiler, arg.expr(), ctx);
                if src != dst {
                    compiler.emit(OpCode::Move { dst, src }, &stmt.span);
                }
                compiler.next_local = (dst + 1) as usize;
                compiler.sync_max_locals();
            }
            let dst = *target as u8;
            compiler.emit(
                OpCode::FiberCreate {
                    dst,
                    func_idx: fid as u32,
                    base,
                    arg_count: args.len() as u8,
                },
                &stmt.span,
            );
            compiler.next_local = base as usize;
            compiler.sync_max_locals();
        }
        HirStmtKind::Yield { value, target } => {
            let src = compile_expr(compiler, value, ctx);
            if let Some(t_id) = target {
                let dst = if let Some(slot) = compiler.lookup_local(t_id) {
                    slot as u8
                } else {
                    let slot = compiler.push_reg();
                    compiler.define_local(*t_id, slot as usize);
                    slot
                };
                compiler.emit(OpCode::YieldWithTarget { dst, src }, &stmt.span);
                if dst == src + 1 {
                    compiler.next_local = (dst + 1) as usize;
                } else {
                    compiler.next_local = src as usize;
                }
            } else {
                compiler.emit(OpCode::Yield { src }, &stmt.span);
                compiler.pop_reg();
            }
        }
        HirStmtKind::YieldFrom(expr) => {
            let fiber_reg = compile_expr(compiler, expr, ctx);
            let start_label = compiler.bytecode.len();
            let test_reg = compiler.push_reg();
            compiler.emit(OpCode::MethodCall { dst: test_reg, kind: MethodKind::IsDone, base: fiber_reg, arg_count: 0 }, &stmt.span);
            let exit_jmp = compiler.bytecode.len();
            compiler.emit(OpCode::JumpIfTrue { src: test_reg, target: 0 }, &stmt.span);
            compiler.pop_reg();

            let val_reg = compiler.push_reg();
            compiler.emit(OpCode::MethodCall { dst: val_reg, kind: MethodKind::Next, base: fiber_reg, arg_count: 0 }, &stmt.span);
            let test_reg2 = compiler.push_reg();
            compiler.emit(OpCode::MethodCall { dst: test_reg2, kind: MethodKind::IsDone, base: fiber_reg, arg_count: 0 }, &stmt.span);
            let skip_jmp = compiler.bytecode.len();
            compiler.emit(OpCode::JumpIfTrue { src: test_reg2, target: 0 }, &stmt.span);
            compiler.pop_reg();

            compiler.emit(OpCode::Yield { src: val_reg }, &stmt.span);
            let skip_target = compiler.bytecode.len() as u32;
            if let OpCode::JumpIfTrue { ref mut target, .. } = compiler.bytecode[skip_jmp] {
                *target = skip_target;
            }
            compiler.pop_reg();
            compiler.emit(OpCode::Jump { target: start_label as u32 }, &stmt.span);
            let end_label = compiler.bytecode.len() as u32;
            if let OpCode::JumpIfTrue { ref mut target, .. } = compiler.bytecode[exit_jmp] {
                *target = end_label;
            }
            compiler.next_local = fiber_reg as usize;
        }
        HirStmtKind::YieldVoid => {
            compiler.emit(OpCode::YieldVoid, &stmt.span);
        }
        HirStmtKind::DatabaseDecl { name, fields } => {
            let mut engine_src = 0;
            let mut path_src = 0;
            let mut tables_base = compiler.next_local as u8;
            let mut table_count = 0;

            for (f_name, f_val) in fields {
                let n = ctx.interner.lookup(*f_name).to_string();
                if n == "engine" {
                    engine_src = compile_expr(compiler, f_val, ctx);
                } else if n == "path" {
                    path_src = compile_expr(compiler, f_val, ctx);
                } else {
                    let reg = compile_expr(compiler, f_val, ctx);
                    if table_count == 0 {
                        tables_base = reg;
                    }
                    table_count += 1;
                }
            }

            let dst = if let Some(&slot) = compiler.scopes[0].get(name) {
                slot as u8
            } else {
                let slot = compiler.push_reg();
                compiler.scopes[0].insert(*name, slot as usize);
                slot
            };

            compiler.emit(
                OpCode::DatabaseInit {
                    dst,
                    engine_src,
                    path_src,
                    tables_base_reg: tables_base,
                    table_count,
                },
                &stmt.span,
            );

            let is_main = compiler.is_main;
            if is_main {
                if let Some(&idx) = ctx.globals.get(name) {
                    compiler.emit(OpCode::SetVar { idx: idx as u32, src: dst }, &stmt.span);
                }
            }

            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
        }
        HirStmtKind::NetRequestStmt { method, url, headers, body, timeout, target } => {
            compiler.local_regs.insert(*target as usize);
            let mut elements = Vec::new();
            elements.push((
                crate::hir::hir::HirExpr {
                    kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("method")),
                    span: Span::default(),
                    ty: Type::String,
                },
                *method.clone()
            ));
            elements.push((
                crate::hir::hir::HirExpr {
                    kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("url")),
                    span: Span::default(),
                    ty: Type::String,
                },
                *url.clone()
            ));
            if let Some(h) = headers {
                elements.push((
                    crate::hir::hir::HirExpr {
                        kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("headers")),
                        span: Span::default(),
                        ty: Type::Json,
                    },
                    *h.clone()
                ));
            }
            if let Some(b) = body {
                elements.push((
                    crate::hir::hir::HirExpr {
                        kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("body")),
                        span: Span::default(),
                        ty: Type::Json,
                    },
                    *b.clone()
                ));
            }
            if let Some(t) = timeout {
                elements.push((
                    crate::hir::hir::HirExpr {
                        kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("timeout")),
                        span: Span::default(),
                        ty: Type::Int,
                    },
                    *t.clone()
                ));
            }
            let map_expr = crate::hir::hir::HirExpr {
                kind: crate::hir::hir::HirExprKind::MapLiteral {
                    key_type: Box::new(Type::String),
                    value_type: Box::new(Type::Json),
                    elements,
                },
                span: Span::default(),
                ty: Type::Json,
            };
            let arg_src = compile_expr(compiler, &map_expr, ctx);
            let dst = *target as u8;
            compiler.emit(OpCode::HttpRequest { dst, arg_src }, &stmt.span);
            compiler.pop_reg();
        }
        HirStmtKind::NetRequestStmtGlobal { method, url, headers, body, timeout, target } => {
            let mut elements = Vec::new();
            elements.push((
                crate::hir::hir::HirExpr {
                    kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("method")),
                    span: Span::default(),
                    ty: Type::String,
                },
                *method.clone()
            ));
            elements.push((
                crate::hir::hir::HirExpr {
                    kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("url")),
                    span: Span::default(),
                    ty: Type::String,
                },
                *url.clone()
            ));
            if let Some(h) = headers {
                elements.push((
                    crate::hir::hir::HirExpr {
                        kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("headers")),
                        span: Span::default(),
                        ty: Type::Json,
                    },
                    *h.clone()
                ));
            }
            if let Some(b) = body {
                elements.push((
                    crate::hir::hir::HirExpr {
                        kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("body")),
                        span: Span::default(),
                        ty: Type::Json,
                    },
                    *b.clone()
                ));
            }
            if let Some(t) = timeout {
                elements.push((
                    crate::hir::hir::HirExpr {
                        kind: crate::hir::hir::HirExprKind::StringLiteral(ctx.interner.intern("timeout")),
                        span: Span::default(),
                        ty: Type::Int,
                    },
                    *t.clone()
                ));
            }
            let map_expr = crate::hir::hir::HirExpr {
                kind: crate::hir::hir::HirExprKind::MapLiteral {
                    key_type: Box::new(Type::String),
                    value_type: Box::new(Type::Json),
                    elements,
                },
                span: Span::default(),
                ty: Type::Json,
            };
            let arg_src = compile_expr(compiler, &map_expr, ctx);
            let dst = compiler.push_reg();
            compiler.emit(OpCode::HttpRequest { dst, arg_src }, &stmt.span);
            let idx = ctx.globals.get(target).copied().unwrap_or(0);
            compiler.emit(OpCode::SetVar { idx: idx as u32, src: dst }, &stmt.span);
            compiler.pop_reg();
            compiler.pop_reg();
        }
        HirStmtKind::Serve { name, port, host, workers, routes } => {
            let saved_next_local = compiler.next_local;
            let port_src = compile_expr(compiler, port, ctx);
            let host_src = host.as_ref().map(|h| compile_expr(compiler, h, ctx)).unwrap_or_else(|| {
                let idx = ctx.add_constant(Value::from_bool(false));
                let r = compiler.push_reg();
                compiler.emit(OpCode::LoadConst { dst: r, idx }, &stmt.span);
                r
            });
            let workers_src = workers.as_ref().map(|w| compile_expr(compiler, w, ctx)).unwrap_or_else(|| {
                let idx = ctx.add_constant(Value::from_bool(false));
                let r = compiler.push_reg();
                compiler.emit(OpCode::LoadConst { dst: r, idx }, &stmt.span);
                r
            });
            let routes_src = compile_expr(compiler, routes, ctx);
            let func_idx = ctx.func_indices.get(name).copied().unwrap_or(0);
            compiler.emit(OpCode::HttpServe { func_idx: func_idx as u32, port_src, host_src, workers_src, routes_src }, &stmt.span);
            compiler.next_local = saved_next_local;
            compiler.sync_max_locals();
        }
        _ => {}
    }
    if compiler.next_local < entry_next_local {
        compiler.next_local = entry_next_local;
    }
}

fn collect_hir_concat_local<'a>(
    expr: &'a crate::hir::hir::HirExpr,
    local: crate::hir::hir::HirLocal,
    args: &mut Vec<&'a crate::hir::hir::HirExpr>,
) -> bool {
    match &expr.kind {
        crate::hir::hir::HirExprKind::Binary { left, op, right } if *op == crate::hir::hir::HirBinOp::Add => {
            args.push(right);
            collect_hir_concat_local(left, local, args)
        }
        crate::hir::hir::HirExprKind::Local(l) if *l == local => true,
        _ => false,
    }
}

fn collect_hir_concat_global<'a>(
    expr: &'a crate::hir::hir::HirExpr,
    name: crate::intern::StringId,
    args: &mut Vec<&'a crate::hir::hir::HirExpr>,
) -> bool {
    match &expr.kind {
        crate::hir::hir::HirExprKind::Binary { left, op, right } if *op == crate::hir::hir::HirBinOp::Add => {
            args.push(right);
            collect_hir_concat_global(left, name, args)
        }
        crate::hir::hir::HirExprKind::Global(n) if *n == name => true,
        _ => false,
    }
}

fn hir_expr_contains_local(expr: &crate::hir::hir::HirExpr, local: crate::hir::hir::HirLocal) -> bool {
    match &expr.kind {
        crate::hir::hir::HirExprKind::Local(l) => *l == local,
        crate::hir::hir::HirExprKind::Binary { left, right, .. } => {
            hir_expr_contains_local(left, local) || hir_expr_contains_local(right, local)
        }
        crate::hir::hir::HirExprKind::Unary { right, .. } => hir_expr_contains_local(right, local),
        crate::hir::hir::HirExprKind::ArrayLiteral { elements } | crate::hir::hir::HirExprKind::ArrayOrSetLiteral { elements } => {
            elements.iter().any(|e| hir_expr_contains_local(e, local))
        }
        crate::hir::hir::HirExprKind::SetLiteral { elements, range, .. } => {
            elements.iter().any(|e| hir_expr_contains_local(e, local)) ||
            range.as_ref().map_or(false, |r| {
                hir_expr_contains_local(&r.start, local) ||
                hir_expr_contains_local(&r.end, local) ||
                r.step.as_ref().map_or(false, |s| hir_expr_contains_local(s, local))
            })
        }
        crate::hir::hir::HirExprKind::MapLiteral { elements, .. } => {
            elements.iter().any(|(k, v)| hir_expr_contains_local(k, local) || hir_expr_contains_local(v, local))
        }
        crate::hir::hir::HirExprKind::TableLiteral { rows, .. } => {
            rows.iter().any(|row| row.iter().any(|e| hir_expr_contains_local(e, local)))
        }
        crate::hir::hir::HirExprKind::DatabaseLiteral(fields) => {
            fields.iter().any(|(_, e)| hir_expr_contains_local(e, local))
        }
        crate::hir::hir::HirExprKind::FunctionCall { args, .. } | crate::hir::hir::HirExprKind::MethodCall { args, .. } | crate::hir::hir::HirExprKind::ModuleCall { args, .. } => {
            args.iter().any(|arg| hir_expr_contains_local(arg.expr(), local))
        }
        crate::hir::hir::HirExprKind::Index { receiver, index } => {
            hir_expr_contains_local(receiver, local) || hir_expr_contains_local(index, local)
        }
        crate::hir::hir::HirExprKind::MemberAccess { receiver, .. } => {
            hir_expr_contains_local(receiver, local)
        }
        crate::hir::hir::HirExprKind::TerminalCommand(_, args) => {
            args.iter().any(|e| hir_expr_contains_local(e, local))
        }
        crate::hir::hir::HirExprKind::Lambda { body, .. } => {
            hir_expr_contains_local(body, local)
        }
        crate::hir::hir::HirExprKind::Tuple(elements) => {
            elements.iter().any(|e| hir_expr_contains_local(e, local))
        }
        crate::hir::hir::HirExprKind::As { expr, .. } | crate::hir::hir::HirExprKind::Yield(expr) => {
            hir_expr_contains_local(expr, local)
        }
        _ => false,
    }
}

fn hir_expr_contains_global(expr: &crate::hir::hir::HirExpr, name: crate::intern::StringId) -> bool {
    match &expr.kind {
        crate::hir::hir::HirExprKind::Global(n) => *n == name,
        crate::hir::hir::HirExprKind::Binary { left, right, .. } => {
            hir_expr_contains_global(left, name) || hir_expr_contains_global(right, name)
        }
        crate::hir::hir::HirExprKind::Unary { right, .. } => hir_expr_contains_global(right, name),
        crate::hir::hir::HirExprKind::ArrayLiteral { elements } | crate::hir::hir::HirExprKind::ArrayOrSetLiteral { elements } => {
            elements.iter().any(|e| hir_expr_contains_global(e, name))
        }
        crate::hir::hir::HirExprKind::SetLiteral { elements, range, .. } => {
            elements.iter().any(|e| hir_expr_contains_global(e, name)) ||
            range.as_ref().map_or(false, |r| {
                hir_expr_contains_global(&r.start, name) ||
                hir_expr_contains_global(&r.end, name) ||
                r.step.as_ref().map_or(false, |s| hir_expr_contains_global(s, name))
            })
        }
        crate::hir::hir::HirExprKind::MapLiteral { elements, .. } => {
            elements.iter().any(|(k, v)| hir_expr_contains_global(k, name) || hir_expr_contains_global(v, name))
        }
        crate::hir::hir::HirExprKind::TableLiteral { rows, .. } => {
            rows.iter().any(|row| row.iter().any(|e| hir_expr_contains_global(e, name)))
        }
        crate::hir::hir::HirExprKind::DatabaseLiteral(fields) => {
            fields.iter().any(|(_, e)| hir_expr_contains_global(e, name))
        }
        crate::hir::hir::HirExprKind::FunctionCall { args, .. } | crate::hir::hir::HirExprKind::MethodCall { args, .. } | crate::hir::hir::HirExprKind::ModuleCall { args, .. } => {
            args.iter().any(|arg| hir_expr_contains_global(arg.expr(), name))
        }
        crate::hir::hir::HirExprKind::Index { receiver, index } => {
            hir_expr_contains_global(receiver, name) || hir_expr_contains_global(index, name)
        }
        crate::hir::hir::HirExprKind::MemberAccess { receiver, .. } => {
            hir_expr_contains_global(receiver, name)
        }
        crate::hir::hir::HirExprKind::TerminalCommand(_, args) => {
            args.iter().any(|e| hir_expr_contains_global(e, name))
        }
        crate::hir::hir::HirExprKind::Lambda { body, .. } => {
            hir_expr_contains_global(body, name)
        }
        crate::hir::hir::HirExprKind::Tuple(elements) => {
            elements.iter().any(|e| hir_expr_contains_global(e, name))
        }
        crate::hir::hir::HirExprKind::As { expr, .. } | crate::hir::hir::HirExprKind::Yield(expr) => {
            hir_expr_contains_global(expr, name)
        }
        _ => false,
    }
}

