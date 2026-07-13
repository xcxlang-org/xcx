use std::sync::Arc;
use crate::vm::opcode::{OpCode, MethodKind, Chunk};
use crate::vm::value::Value;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use crate::hir::hir::{HirExpr, HirExprKind, LAMBDA_LOCAL_OFFSET};

pub fn compile_expr(compiler: &mut FunctionCompiler, expr: &HirExpr, ctx: &mut CompileContext) -> u8 {
    match &expr.kind {
        HirExprKind::IntLiteral(val) => {
            let idx = ctx.add_constant(Value::from_i64(*val));
            let dst = compiler.push_reg();
            compiler.emit(OpCode::LoadConst { dst, idx }, &expr.span);
            dst
        }
        HirExprKind::FloatLiteral(val) => {
            let idx = ctx.add_constant(Value::from_f64(*val));
            let dst = compiler.push_reg();
            compiler.emit(OpCode::LoadConst { dst, idx }, &expr.span);
            dst
        }
        HirExprKind::BoolLiteral(val) => {
            let idx = ctx.add_constant(Value::from_bool(*val));
            let dst = compiler.push_reg();
            compiler.emit(OpCode::LoadConst { dst, idx }, &expr.span);
            dst
        }
        HirExprKind::StringLiteral(val) => {
            let s_obj = crate::vm::object::StringObj::new(ctx.interner.lookup(*val).to_string().into_bytes());
            let idx = ctx.add_constant(Value::from_string(Arc::new(s_obj)));
            let dst = compiler.push_reg();
            compiler.emit(OpCode::LoadConst { dst, idx }, &expr.span);
            dst
        }
        HirExprKind::Local(local) => {
            let name = HIR_CAPTURE_MAP.with(|c| c.borrow().get(local).cloned());
            if let Some(name_id) = name {
                if let Some(slot) = compiler.lookup_local(&name_id) {
                    slot as u8
                } else {
                    *local as u8
                }
            } else {
                *local as u8
            }
        }
        HirExprKind::Global(name) => {
            let dst = compiler.push_reg();
            if compiler.is_table_lambda {
                let s = ctx.interner.lookup(*name).to_string();
                let mi = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(s.into_bytes()))));
                compiler.emit(OpCode::MethodCallCustom { dst, method_name_idx: mi, base: 0, arg_count: 0 }, &expr.span);
            } else {
                let idx = ctx.globals.get(name).copied().unwrap_or(0);
                compiler.emit(OpCode::GetVar { dst, idx: idx as u32 }, &expr.span);
            }
            dst
        }
        HirExprKind::Binary { left, op, right } => {
            let src1 = compile_expr(compiler, left, ctx);
            let src2 = compile_expr(compiler, right, ctx);
            
            let is_v1_local = compiler.local_regs.contains(&(src1 as usize));
            let is_v2_local = compiler.local_regs.contains(&(src2 as usize));
            
            let dst = if !is_v1_local {
                src1
            } else if !is_v2_local {
                src2
            } else {
                compiler.push_reg()
            };
            
            match op {
                crate::hir::hir::HirBinOp::Add => compiler.emit(OpCode::Add { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Sub => compiler.emit(OpCode::Sub { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Mul => compiler.emit(OpCode::Mul { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Div => compiler.emit(OpCode::Div { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Mod => compiler.emit(OpCode::Mod { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Pow => compiler.emit(OpCode::Pow { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Equal => compiler.emit(OpCode::Equal { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::NotEqual => compiler.emit(OpCode::NotEqual { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Less => compiler.emit(OpCode::Less { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::LessEqual => compiler.emit(OpCode::LessEqual { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Greater => compiler.emit(OpCode::Greater { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::GreaterEqual => compiler.emit(OpCode::GreaterEqual { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::And => compiler.emit(OpCode::And { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Or => compiler.emit(OpCode::Or { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::SetUnion => compiler.emit(OpCode::SetUnion { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::SetIntersection => compiler.emit(OpCode::SetIntersection { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::SetDifference => compiler.emit(OpCode::SetDifference { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::SetSymDifference => compiler.emit(OpCode::SetSymDifference { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::Has => compiler.emit(OpCode::Has { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::IntConcat => compiler.emit(OpCode::IntConcat { dst, src1, src2 }, &expr.span),
                crate::hir::hir::HirBinOp::MapConcat => {
                    let base = compiler.next_local as u8;
                    compiler.next_local += 2;
                    compiler.sync_max_locals();
                    compiler.emit(OpCode::Move { dst: base + 1, src: src2 }, &expr.span);
                    compiler.emit(OpCode::Move { dst: base, src: src1 }, &expr.span);
                    compiler.emit(OpCode::MapInit { dst, base, count: 1 }, &expr.span);
                }
            }
            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::Unary { op, right } => {
            let src = compile_expr(compiler, right, ctx);
            let is_v_local = compiler.local_regs.contains(&(src as usize));
            let dst = if !is_v_local { src } else { compiler.push_reg() };
            match op {
                crate::hir::hir::HirUnOp::Neg => compiler.emit(OpCode::Neg { dst, src }, &expr.span),
                crate::hir::hir::HirUnOp::Not => compiler.emit(OpCode::Not { dst, src }, &expr.span),
            }
            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::ArrayLiteral { elements } | HirExprKind::ArrayOrSetLiteral { elements } => {
            let base = compiler.next_local as u8;
            for (i, e) in elements.iter().enumerate() {
                let dst = base + i as u8;
                compiler.next_local = dst as usize;
                let src = compile_expr(compiler, e, ctx);
                if src != dst {
                    compiler.emit(OpCode::Move { dst, src }, &expr.span);
                }
                compiler.next_local = (dst + 1) as usize;
                compiler.sync_max_locals();
            }
            let dst = base;
            compiler.emit(OpCode::ArrayInit { dst, base, count: elements.len() as u32 }, &expr.span);
            compiler.next_local = (base + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::SetLiteral { set_type: _, elements, range } => {
            if let Some(r) = range {
                let start = compile_expr(compiler, &r.start, ctx);
                let end = compile_expr(compiler, &r.end, ctx);
                let (step, has_step_reg) = if let Some(s) = &r.step {
                    let step_reg = compile_expr(compiler, s, ctx);
                    let h_idx = ctx.add_constant(Value::from_bool(true));
                    let h_reg = compiler.push_reg();
                    compiler.emit(OpCode::LoadConst { dst: h_reg, idx: h_idx }, &expr.span);
                    (step_reg, h_reg)
                } else {
                    let dummy_idx = ctx.add_constant(Value::from_bool(false));
                    let dummy = compiler.push_reg();
                    compiler.emit(OpCode::LoadConst { dst: dummy, idx: dummy_idx }, &expr.span);
                    let f_idx = ctx.add_constant(Value::from_bool(false));
                    let f_reg = compiler.push_reg();
                    compiler.emit(OpCode::LoadConst { dst: f_reg, idx: f_idx }, &expr.span);
                    (dummy, f_reg)
                };
                let dst = start;
                compiler.emit(OpCode::SetRange { dst, start, end, step, has_step: has_step_reg }, &expr.span);
                compiler.next_local = (dst + 1) as usize;
                compiler.sync_max_locals();
                dst
            } else {
                let base = compiler.next_local as u8;
                for (i, e) in elements.iter().enumerate() {
                    let dst = base + i as u8;
                    compiler.next_local = dst as usize;
                    let src = compile_expr(compiler, e, ctx);
                    if src != dst {
                        compiler.emit(OpCode::Move { dst, src }, &expr.span);
                    }
                    compiler.next_local = (dst + 1) as usize;
                    compiler.sync_max_locals();
                }
                let dst = base;
                compiler.emit(OpCode::SetInit { dst, base, count: elements.len() as u32 }, &expr.span);
                compiler.next_local = (base + 1) as usize;
                compiler.sync_max_locals();
                dst
            }
        }
        HirExprKind::MethodCall { receiver, method, args, wait_after } => {
            let method_name = ctx.interner.lookup(*method).to_string();
            if let Some(dst) = super::compile_expr_special::compile_special_method_call(
                compiler,
                receiver,
                &method_name,
                args,
                &expr.span,
                *wait_after,
                ctx,
            ) {
                return dst;
            }

            let base = compiler.next_local as u8;
            let receiver_reg = compile_expr(compiler, receiver, ctx);
            if receiver_reg != base {
                compiler.emit(OpCode::Move { dst: base, src: receiver_reg }, &expr.span);
            }
            compiler.next_local = (base + 1) as usize;
            compiler.sync_max_locals();
            let mut arg_count = 0u8;
            let mut arg_names = Vec::new();
            let mut has_named = false;

            for a in args {
                let arg_reg = base + 1 + arg_count;
                compiler.next_local = arg_reg as usize;
                let src = compile_expr(compiler, a.expr(), ctx);
                if src != arg_reg {
                    compiler.emit(OpCode::Move { dst: arg_reg, src }, &a.expr().span);
                }
                arg_count += 1;

                if let crate::hir::hir::HirArgument::Named(id, _) = a {
                    has_named = true;
                    arg_names.push(ctx.interner.lookup(*id).to_string());
                } else {
                    arg_names.push(String::new());
                }

                if let HirExprKind::Lambda { body, .. } = &a.expr().kind {
                    let mut local_idx_to_name = std::collections::HashMap::new();
                    for scope in &compiler.scopes {
                        for (&name, &slot) in scope {
                            local_idx_to_name.insert(slot as u32, name);
                        }
                    }
                    let flat_locals = compiler.convert_to_flat_locals();
                    let mut lambda_captures = Vec::new();
                    collect_captures_hir(body, &flat_locals, &local_idx_to_name, &mut lambda_captures);
                    for id in &lambda_captures {
                        if let Some(slot) = compiler.lookup_local(id) {
                            let r_cap = base + 1 + arg_count;
                            compiler.next_local = r_cap as usize;
                            compiler.emit(OpCode::Move { dst: r_cap, src: slot as u8 }, &a.expr().span);
                            arg_count += 1;
                        }
                    }
                }
            }
            let dst = base;

            let is_builtin_property = match method_name.as_str() {
                "length" | "year" | "month" | "day" | "hour" | "minute" | "second" | "size" | "count" | "status" | "ok" | "error" => true,
                _ => false
            };

            if is_builtin_property && args.is_empty() {
                let name_idx = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(method_name.into_bytes()))));
                compiler.emit(OpCode::GetMember { dst, container: base, name_idx }, &expr.span);
            } else if let Some(method_kind) = compiler.map_method_kind(&method_name) {
                if has_named {
                    let names_val = Value::from_string_array(Arc::new(arg_names));
                    let names_idx = ctx.add_constant(names_val);
                    compiler.emit(
                        OpCode::MethodCallNamed {
                            dst,
                            kind: method_kind,
                            base,
                            arg_count,
                            names_idx,
                        },
                        &expr.span,
                    );
                } else {
                    compiler.emit(
                        OpCode::MethodCall {
                            dst,
                            kind: method_kind,
                            base,
                            arg_count,
                        },
                        &expr.span,
                    );
                }
            } else {
                let mi = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(method_name.into_bytes()))));
                compiler.emit(OpCode::MethodCallCustom { dst, method_name_idx: mi, base, arg_count }, &expr.span);
            }

            if *wait_after {
                compiler.emit(OpCode::Wait { src: dst }, &expr.span);
            }
            compiler.next_local = (base + 1 + arg_count) as usize;
            compiler.sync_max_locals();
            compiler.next_local = base as usize + 1;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::FunctionCall { name, args } => {
            if let Some(dst) = super::compile_expr_special::compile_special_function_call(
                compiler,
                *name,
                args,
                &expr.span,
                ctx,
            ) {
                return dst;
            }
            let base = compiler.next_local as u8;
            for (i, arg) in args.iter().enumerate() {
                let dst = (base as usize + i) as u8;
                let src = compile_expr(compiler, arg.expr(), ctx);
                if src != dst {
                    compiler.emit(OpCode::Move { dst, src }, &expr.span);
                }
                compiler.next_local = dst as usize + 1;
                compiler.sync_max_locals();
            }
            let dst = base;
            if let Some(&func_id) = ctx.func_indices.get(name) {
                if ctx.functions[func_id].is_fiber {
                    compiler.emit(OpCode::FiberCreate { dst, func_idx: func_id as u32, base, arg_count: args.len() as u8 }, &expr.span);
                } else {
                    compiler.emit(OpCode::Call { dst, func_idx: func_id as u32, base, arg_count: args.len() as u8 }, &expr.span);
                }
            } else {
                compiler.emit(OpCode::Halt, &expr.span);
            }
            compiler.next_local = base as usize + 1;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::Index { receiver, index } => {
            let base = compiler.next_local as u8;
            let r_src = compile_expr(compiler, receiver, ctx);
            if r_src != base {
                compiler.emit(OpCode::Move { dst: base, src: r_src }, &expr.span);
            }
            compiler.next_local = (base + 1) as usize;
            let i_src = compile_expr(compiler, index, ctx);
            let i_dst = base + 1;
            if i_src != i_dst {
                compiler.emit(OpCode::Move { dst: i_dst, src: i_src }, &expr.span);
            }
            compiler.next_local = (i_dst + 1) as usize;
            let dst = compiler.push_reg();
            compiler.emit(OpCode::MethodCall { dst, kind: MethodKind::Get, base, arg_count: 1 }, &expr.span);
            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::MemberAccess { receiver, member } => {
            let base = compiler.next_local as u8;
            let receiver_reg = compile_expr(compiler, receiver, ctx);
            if receiver_reg != base {
                compiler.emit(OpCode::Move { dst: base, src: receiver_reg }, &expr.span);
            }
            compiler.next_local = (base + 1) as usize;
            let method_name = ctx.interner.lookup(*member).to_string();
            let dst = compiler.push_reg();

            let is_property = match method_name.as_str() {
                "length" | "year" | "month" | "day" | "hour" | "minute" | "second" | "affected" | "insertId" | "status" | "ok" | "error" => true,
                _ => false
            };

            if is_property {
                let name_idx = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(method_name.clone().into_bytes()))));
                compiler.emit(OpCode::GetMember { dst, container: base, name_idx }, &expr.span);
            } else if let Some(kind) = compiler.map_method_kind(&method_name) {
                compiler.emit(OpCode::MethodCall { dst, kind, base, arg_count: 0 }, &expr.span);
            } else {
                let mi = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(method_name.into_bytes()))));
                compiler.emit(OpCode::MethodCallCustom { dst, method_name_idx: mi, base, arg_count: 0 }, &expr.span);
            }
            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::MapLiteral { key_type: _, value_type: _, elements } => {
            let base = compiler.next_local as u8;
            for (i, (k, v)) in elements.iter().enumerate() {
                let k_dst = base + (i as u8 * 2);
                compiler.next_local = k_dst as usize;
                let k_src = compile_expr(compiler, k, ctx);
                if k_src != k_dst {
                    compiler.emit(OpCode::Move { dst: k_dst, src: k_src }, &expr.span);
                }
                compiler.next_local = (k_dst + 1) as usize;
                compiler.sync_max_locals();

                let v_dst = k_dst + 1;
                compiler.next_local = v_dst as usize;
                let v_src = compile_expr(compiler, v, ctx);
                if v_src != v_dst {
                    compiler.emit(OpCode::Move { dst: v_dst, src: v_src }, &expr.span);
                }
                compiler.next_local = (v_dst + 1) as usize;
                compiler.sync_max_locals();
            }
            let dst = base;
            compiler.emit(OpCode::MapInit { dst, base, count: elements.len() as u32 }, &expr.span);
            compiler.next_local = (base + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::RandomInt { min, max, step } => {
            let min_reg = compile_expr(compiler, min, ctx);
            let max_reg = compile_expr(compiler, max, ctx);
            let (step_reg, has_step_reg) = if let Some(s) = step {
                let s_reg = compile_expr(compiler, s, ctx);
                let h_idx = ctx.add_constant(Value::from_bool(true));
                let h_reg = compiler.push_reg();
                compiler.emit(OpCode::LoadConst { dst: h_reg, idx: h_idx }, &expr.span);
                (s_reg, h_reg)
            } else {
                let dummy_idx = ctx.add_constant(Value::from_bool(false));
                let dummy = compiler.push_reg();
                compiler.emit(OpCode::LoadConst { dst: dummy, idx: dummy_idx }, &expr.span);
                let f_idx = ctx.add_constant(Value::from_bool(false));
                let f_reg = compiler.push_reg();
                compiler.emit(OpCode::LoadConst { dst: f_reg, idx: f_idx }, &expr.span);
                (dummy, f_reg)
            };
            let dst = min_reg;
            compiler.emit(
                OpCode::RandomInt {
                    dst,
                    min: min_reg,
                    max: max_reg,
                    step: step_reg,
                    has_step: has_step_reg,
                },
                &expr.span,
            );
            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::RandomFloat { min, max, step } => {
            let min_reg = compile_expr(compiler, min, ctx);
            let max_reg = compile_expr(compiler, max, ctx);
            let (step_reg, has_step_reg) = if let Some(s) = step {
                let s_reg = compile_expr(compiler, s, ctx);
                let h_idx = ctx.add_constant(Value::from_bool(true));
                let h_reg = compiler.push_reg();
                compiler.emit(OpCode::LoadConst { dst: h_reg, idx: h_idx }, &expr.span);
                (s_reg, h_reg)
            } else {
                let dummy_idx = ctx.add_constant(Value::from_bool(false));
                let dummy = compiler.push_reg();
                compiler.emit(OpCode::LoadConst { dst: dummy, idx: dummy_idx }, &expr.span);
                let f_idx = ctx.add_constant(Value::from_bool(false));
                let f_reg = compiler.push_reg();
                compiler.emit(OpCode::LoadConst { dst: f_reg, idx: f_idx }, &expr.span);
                (dummy, f_reg)
            };
            let dst = min_reg;
            compiler.emit(
                OpCode::RandomFloat {
                    dst,
                    min: min_reg,
                    max: max_reg,
                    step: step_reg,
                    has_step: has_step_reg,
                },
                &expr.span,
            );
            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::RandomChoice { set } => {
            let src = compile_expr(compiler, set, ctx);
            let dst = compiler.push_reg();
            compiler.emit(OpCode::RandomChoice { dst, src }, &expr.span);
            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::As { expr: inner, name } => {
            let src = compile_expr(compiler, inner, ctx);
            if let Some(&slot) = compiler.scopes[0].get(name) {
                compiler.emit(OpCode::Move { dst: slot as u8, src }, &expr.span);
                src
            } else if let Some(&idx) = ctx.globals.get(name) {
                compiler.emit(OpCode::SetVar { idx: idx as u32, src }, &expr.span);
                src
            } else {
                let slot = src as usize;
                compiler.scopes[0].insert(*name, slot);
                if slot >= compiler.next_local {
                    compiler.next_local = slot + 1;
                    compiler.sync_max_locals();
                }
                src
            }
        }
        HirExprKind::ModuleCall { module, method, args } => {
            let method_name = ctx.interner.lookup(*method).to_string();
            match module {
                crate::frontend::lexer::TokenKind::Net => {
                    match method_name.as_str() {
                        "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
                            let url_src = compile_expr(compiler, args[0].expr(), ctx);
                            let body_src = if let Some(arg) = args.get(1) {
                                compile_expr(compiler, arg.expr(), ctx)
                            } else {
                                let f = ctx.add_constant(Value::from_bool(false));
                                let r = compiler.push_reg();
                                compiler.emit(OpCode::LoadConst { dst: r, idx: f }, &expr.span);
                                r
                            };
                            let method_idx = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(method_name.into_bytes()))));
                            let dst = url_src;
                            compiler.emit(OpCode::HttpCall { dst, method_idx, url_src, body_src }, &expr.span);
                            compiler.next_local = (dst + 1) as usize; compiler.sync_max_locals();
                            dst
                        }
                        "respond" => {
                            let status_src = compile_expr(compiler, args[0].expr(), ctx);
                            let body_src   = compile_expr(compiler, args[1].expr(), ctx);
                            let headers_src = if let Some(arg) = args.get(2) {
                                compile_expr(compiler, arg.expr(), ctx)
                            } else {
                                let f = ctx.add_constant(Value::from_bool(false));
                                let r = compiler.push_reg();
                                compiler.emit(OpCode::LoadConst { dst: r, idx: f }, &expr.span);
                                r
                            };
                            let dst = status_src;
                            compiler.emit(OpCode::HttpRespond { dst, status_src, body_src, headers_src }, &expr.span);
                            compiler.next_local = (dst + 1) as usize;
                            dst
                        }
                        _ => compiler.push_reg()
                    }
                }
                crate::frontend::lexer::TokenKind::Json => {
                    match method_name.as_str() {
                        "parse" => {
                            let src = compile_expr(compiler, args[0].expr(), ctx);
                            let dst = compiler.push_reg();
                            compiler.emit(OpCode::JsonParse { dst, src }, &expr.span);
                            compiler.next_local = (dst + 1) as usize;
                            compiler.sync_max_locals();
                            dst
                        }
                        "toStr" | "stringify" => {
                            let base = compiler.next_local as u8;
                            let receiver_reg = compile_expr(compiler, args[0].expr(), ctx);
                            if receiver_reg != base { compiler.emit(OpCode::Move { dst: base, src: receiver_reg }, &expr.span); }
                            compiler.next_local = (base + 1) as usize;
                            let dst = compiler.push_reg();
                            compiler.emit(OpCode::MethodCall { dst, kind: MethodKind::ToStr, base, arg_count: 0 }, &expr.span);
                            compiler.next_local = (dst + 1) as usize;
                            dst
                        }
                        _ => compiler.push_reg()
                    }
                }
                crate::frontend::lexer::TokenKind::Crypto => {
                    match method_name.as_str() {
                        "hash" => {
                            let pass_src = compile_expr(compiler, args[0].expr(), ctx);
                            let alg_src = if let Some(arg) = args.get(1) {
                                compile_expr(compiler, arg.expr(), ctx)
                            } else {
                                let s = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(b"sha256".to_vec()))));
                                let r = compiler.push_reg();
                                compiler.emit(OpCode::LoadConst { dst: r, idx: s }, &expr.span);
                                r
                            };
                            let dst = compiler.push_reg();
                            compiler.emit(OpCode::CryptoHash { dst, pass_src, alg_src }, &expr.span);
                            compiler.next_local = (dst + 1) as usize;
                            compiler.sync_max_locals();
                            dst
                        }
                        "verify" => {
                            let pass_src = compile_expr(compiler, args[0].expr(), ctx);
                            let hash_src = compile_expr(compiler, args[1].expr(), ctx);
                            let alg_src = if let Some(arg) = args.get(2) {
                                compile_expr(compiler, arg.expr(), ctx)
                            } else {
                                let s = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(b"sha256".to_vec()))));
                                let r = compiler.push_reg();
                                compiler.emit(OpCode::LoadConst { dst: r, idx: s }, &expr.span);
                                r
                            };
                            let dst = compiler.push_reg();
                            compiler.emit(OpCode::CryptoVerify { dst, pass_src, hash_src, alg_src }, &expr.span);
                            compiler.next_local = (dst + 1) as usize;
                            compiler.sync_max_locals();
                            dst
                        }
                        "token" => {
                            let len_src = if let Some(arg) = args.get(0) {
                                compile_expr(compiler, arg.expr(), ctx)
                            } else {
                                let i = ctx.add_constant(Value::from_i64(32));
                                let r = compiler.push_reg();
                                compiler.emit(OpCode::LoadConst { dst: r, idx: i }, &expr.span);
                                r
                            };
                            let dst = compiler.push_reg();
                            compiler.emit(OpCode::CryptoToken { dst, len_src }, &expr.span);
                            compiler.next_local = (dst + 1) as usize;
                            compiler.sync_max_locals();
                            dst
                        }
                        _ => compiler.push_reg()
                    }
                }
                crate::frontend::lexer::TokenKind::Env => {
                    match method_name.as_str() {
                        "get" => {
                            let src = compile_expr(compiler, args[0].expr(), ctx);
                            let dst = compiler.push_reg();
                            compiler.emit(OpCode::EnvGet { dst, src }, &expr.span);
                            compiler.next_local = (dst + 1) as usize;
                            compiler.sync_max_locals();
                            dst
                        }
                        "args" => {
                            let dst = compiler.push_reg();
                            compiler.emit(OpCode::EnvArgs { dst }, &expr.span);
                            dst
                        }
                        _ => compiler.push_reg()
                    }
                }
                crate::frontend::lexer::TokenKind::Date => {
                    if method_name == "now" {
                        let dst = compiler.push_reg();
                        compiler.emit(OpCode::DateNow { dst }, &expr.span);
                        dst
                    } else { compiler.push_reg() }
                }
                crate::frontend::lexer::TokenKind::Perf => {
                    let dst = compiler.push_reg();
                    match method_name.as_str() {
                        "ms" => compiler.emit(OpCode::PerfMs { dst }, &expr.span),
                        "us" => compiler.emit(OpCode::PerfUs { dst }, &expr.span),
                        "ns" => compiler.emit(OpCode::PerfNs { dst }, &expr.span),
                        _ => {}
                    }
                    dst
                }
                crate::frontend::lexer::TokenKind::Store => {
                    let base = compiler.next_local as u8;
                    let mut arg_count = 0u8;
                    for arg in args {
                        let arg_reg = base + arg_count;
                        compiler.next_local = arg_reg as usize;
                        let src = compile_expr(compiler, arg.expr(), ctx);
                        if src != arg_reg { compiler.emit(OpCode::Move { dst: arg_reg, src }, &expr.span); }
                        arg_count += 1;
                    }
                    compiler.next_local = (base + arg_count) as usize;
                    compiler.sync_max_locals();
                    let dst = base;
                    match method_name.as_str() {
                        "write"  => compiler.emit(OpCode::StoreWrite { dst, base }, &expr.span),
                        "read"   => compiler.emit(OpCode::StoreRead { dst, base }, &expr.span),
                        "append" => compiler.emit(OpCode::StoreAppend { dst, base }, &expr.span),
                        "exists" => compiler.emit(OpCode::StoreExists { dst, base }, &expr.span),
                        "delete" => compiler.emit(OpCode::StoreDelete { dst, base }, &expr.span),
                        "list"   => compiler.emit(OpCode::StoreList { dst, base }, &expr.span),
                        "isDir"  => compiler.emit(OpCode::StoreIsDir { dst, base }, &expr.span),
                        "size"   => compiler.emit(OpCode::StoreSize { dst, base }, &expr.span),
                        "mkdir"  => compiler.emit(OpCode::StoreMkdir { dst, base }, &expr.span),
                        "glob"   => compiler.emit(OpCode::StoreGlob { dst, base }, &expr.span),
                        "zip"    => compiler.emit(OpCode::StoreZip { dst, base }, &expr.span),
                        "unzip"  => compiler.emit(OpCode::StoreUnzip { dst, base }, &expr.span),
                        _ => { 
                            return compiler.push_reg();
                        }
                    };
                    compiler.next_local = (dst + 1) as usize;
                    dst
                }
                _ => compiler.push_reg(),
            }
        }
        HirExprKind::TerminalCommand(cmd_id, args) => {
            let cmd = ctx.interner.lookup(*cmd_id);
            let dst = compiler.push_reg();
            if cmd == "exit" { compiler.emit(OpCode::TerminalExit { dst }, &expr.span); }
            else if cmd == "clear" { compiler.emit(OpCode::TerminalClear { dst }, &expr.span); }
            else if cmd == "run" {
                if let Some(a) = args.get(0) {
                    let cmd_src = compile_expr(compiler, a, ctx);
                    compiler.emit(OpCode::TerminalRun { dst, cmd_src }, &expr.span);
                    compiler.next_local = (dst + 1) as usize; 
                }
            } else if cmd == "raw" { compiler.emit(OpCode::TerminalRaw { dst }, &expr.span); }
            else if cmd == "normal" || cmd == "cooked" { compiler.emit(OpCode::TerminalNormal { dst }, &expr.span); }
            else if cmd == "cursor" {
                if let Some(a) = args.get(0) {
                    let val_str = match &a.kind {
                        HirExprKind::Tag(id) => ctx.interner.lookup(*id),
                        _ => "",
                    };
                    if val_str == "on" { compiler.emit(OpCode::TerminalCursor { dst, on: true }, &expr.span); }
                    else if val_str == "off" { compiler.emit(OpCode::TerminalCursor { dst, on: false }, &expr.span); }
                }
            } else if cmd == "move" {
                if args.len() >= 2 {
                    let x_src = compile_expr(compiler, &args[0], ctx);
                    let y_src = compile_expr(compiler, &args[1], ctx);
                    compiler.emit(OpCode::TerminalMove { dst, x_src, y_src }, &expr.span);
                    compiler.next_local = (dst + 1) as usize;
                }
            }
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::Lambda { params, return_type: _, body, locals: _ } => {
            let mut local_idx_to_name = std::collections::HashMap::new();
            for scope in &compiler.scopes {
                for (&name, &slot) in scope {
                    local_idx_to_name.insert(slot as u32, name);
                }
            }
            let flat_locals = compiler.convert_to_flat_locals();
            let mut captures = Vec::new();
            
            collect_captures_hir(body, &flat_locals, &local_idx_to_name, &mut captures);
            let mut sub = FunctionCompiler::new(false, Some(flat_locals));
            for (i, param) in params.iter().enumerate() {
                sub.define_local(param.name, i);
            }
            for id in &captures { sub.lookup_local(id); }
            sub.next_local = params.len() + captures.len();

            let old_map = HIR_CAPTURE_MAP.with(|c| c.borrow().clone());
            let mut new_map = std::collections::HashMap::new();
            for (k, v) in &old_map {
                new_map.insert(k + LAMBDA_LOCAL_OFFSET, *v);
            }
            for scope in &sub.scopes {
                for (&name, &slot) in scope {
                    new_map.insert(slot as u32, name);
                }
            }
            HIR_CAPTURE_MAP.with(|c| {
                *c.borrow_mut() = new_map;
            });

            let res = compile_expr(&mut sub, body, ctx);
            sub.emit(OpCode::Return { src: res }, &expr.span);
            
            HIR_CAPTURE_MAP.with(|c| {
                *c.borrow_mut() = old_map;
            });

            let fid = ctx.functions.len();
            let has_loops = crate::vm::opcode::calculate_has_loops(&sub.bytecode);
            ctx.functions.push(std::sync::Arc::new(Chunk::new(sub.bytecode, sub.spans, false, sub.max_locals_used.max(sub.next_local), has_loops, "lambda".to_string(), params.len())));
            let f_val = Value::from_function(fid as u32);
            let f_idx = ctx.add_constant(f_val);
            let dst = compiler.push_reg();
            compiler.emit(OpCode::LoadConst { dst, idx: f_idx }, &expr.span);
            dst
        }
        HirExprKind::Yield(v) => {
            let src = compile_expr(compiler, v, ctx);
            compiler.emit(OpCode::Yield { src }, &expr.span);
            src
        }
        HirExprKind::Tag(name) => {
            let idx = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(ctx.interner.lookup(*name).to_string().into_bytes()))));
            let dst = compiler.push_reg();
            compiler.emit(OpCode::LoadConst { dst, idx }, &expr.span);
            dst
        }
        HirExprKind::RawBlock(id) => {
            let s = ctx.interner.lookup(*id).to_string();
            let s_obj = crate::vm::object::StringObj::new(s.into_bytes());
            let idx = ctx.add_constant(Value::from_string(Arc::new(s_obj)));
            let dst = compiler.push_reg();
            compiler.emit(OpCode::LoadConst { dst, idx }, &expr.span);
            dst
        }
        HirExprKind::TableLiteral { columns, rows } => {
            let vm_cols = columns.iter().map(|c| crate::vm::object::VMColumn {
                name: ctx.interner.lookup(c.name).to_string(),
                ty: c.ty.clone(),
                is_auto: c.is_auto(),
                is_pk: c.is_pk(),
                is_unique: c.is_unique(),
            }).collect();
            let skeleton = Value::from_table(Arc::new(parking_lot::RwLock::new(
                crate::vm::object::TableObj {
                    table_name: String::new(),
                    columns: vm_cols,
                    rows: Vec::new(),
                    sql_binding: None,
                    sql_where: None,
                    pending_op: None,
                }
            )));
            let ncol = columns.iter().filter(|c| !c.is_auto()).count();
            let ci = ctx.add_constant(skeleton);
            let base = compiler.next_local as u8;

            if rows.len() * ncol > 200 {
                compiler.emit(OpCode::TableBegin { dst: base, skeleton_idx: ci }, &expr.span);
                let row_base = base + 1;
                for row in rows {
                    let mut col_idx = 0;
                    compiler.next_local = row_base as usize;
                    for val in row {
                        let dst = row_base + col_idx as u8;
                        let src = compile_expr(compiler, val, ctx);
                        if src != dst {
                            compiler.emit(OpCode::Move { dst, src }, &expr.span);
                        }
                        compiler.next_local = (dst + 1) as usize;
                        compiler.sync_max_locals();
                        col_idx += 1;
                    }
                    compiler.emit(OpCode::TableInitRow { tbl_dst: base, base: row_base, col_count: ncol as u8 }, &expr.span);
                }
                compiler.next_local = (base + 1) as usize;
                compiler.sync_max_locals();
                base
            } else {
                let mut current_idx = 0;
                for row in rows {
                    for val in row {
                        let dst = base + current_idx as u8;
                        let src = compile_expr(compiler, val, ctx);
                        if src != dst {
                            compiler.emit(OpCode::Move { dst, src }, &expr.span);
                        }
                        compiler.next_local = (dst + 1) as usize;
                        compiler.sync_max_locals();
                        current_idx += 1;
                    }
                }
                compiler.emit(OpCode::TableInit { dst: base, skeleton_idx: ci, base, row_count: rows.len() as u32, col_count: ncol as u32 }, &expr.span);
                compiler.next_local = (base + 1) as usize;
                compiler.sync_max_locals();
                base
            }
        }
        HirExprKind::DatabaseLiteral(fields) => {
            let mut engine_reg = 0;
            let mut path_reg = 0;
            let mut found_engine = false;
            let mut found_path = false;
            for (name, val) in fields {
                let name_str = ctx.interner.lookup(*name);
                if name_str == "engine" {
                    engine_reg = compile_expr(compiler, val, ctx);
                    found_engine = true;
                } else if name_str == "path" {
                    path_reg = compile_expr(compiler, val, ctx);
                    found_path = true;
                }
            }
            if !found_engine || !found_path {
                return compiler.push_reg();
            }
            let base = compiler.next_local as u8;
            let mut table_count = 0;
            for (name, val) in fields {
                let name_str = ctx.interner.lookup(*name);
                if name_str != "engine" && name_str != "path" {
                    let name_idx = ctx.add_constant(Value::from_string(Arc::new(crate::vm::object::StringObj::new(name_str.to_string().into_bytes()))));
                    let nr = base + (table_count as u8 * 2);
                    compiler.emit(OpCode::LoadConst { dst: nr, idx: name_idx }, &expr.span);
                    compiler.next_local = (nr + 1) as usize;
                    compiler.sync_max_locals();
                    let tr = nr + 1;
                    let src = compile_expr(compiler, val, ctx);
                    if src != tr {
                        compiler.emit(OpCode::Move { dst: tr, src }, &expr.span);
                    }
                    compiler.next_local = (tr + 1) as usize;
                    compiler.sync_max_locals();
                    table_count += 1;
                }
            }
            let dst = engine_reg;
            compiler.emit(OpCode::DatabaseInit { dst, engine_src: engine_reg, path_src: path_reg, tables_base_reg: base, table_count }, &expr.span);
            compiler.next_local = (dst + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
        HirExprKind::DateLiteral { date_string, format } => {
            let date_str = ctx.interner.lookup(*date_string).to_string();
            let date = if let Some(fmt_id) = format {
                let fmt_str = ctx.interner.lookup(*fmt_id).to_string();
                let chrono_fmt = fmt_str
                    .replace("YYYY", "%Y").replace("MM", "%m").replace("DD", "%d")
                    .replace("M", "%-m").replace("D", "%-d");
                chrono::NaiveDate::parse_from_str(&date_str, &chrono_fmt)
                    .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            } else {
                chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            };
            let dt = date.and_hms_opt(0, 0, 0).unwrap();
            let i = ctx.add_constant(Value::from_date(dt.and_utc().timestamp_millis()));
            let dst = compiler.push_reg();
            compiler.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
            dst
        }
        HirExprKind::Tuple(exprs) => {
            let base = compiler.next_local as u8;
            for (i, e) in exprs.iter().enumerate() {
                let dst = base + i as u8;
                compiler.next_local = dst as usize;
                let src = compile_expr(compiler, e, ctx);
                if src != dst {
                    compiler.emit(OpCode::Move { dst, src }, &expr.span);
                }
                compiler.next_local = (dst + 1) as usize;
                compiler.sync_max_locals();
            }
            let dst = base;
            compiler.emit(OpCode::ArrayInit { dst, base, count: exprs.len() as u32 }, &expr.span);
            compiler.next_local = (base + 1) as usize;
            compiler.sync_max_locals();
            dst
        }
    }
}

thread_local! {
    pub(crate) static HIR_CAPTURE_MAP: std::cell::RefCell<std::collections::HashMap<u32, crate::intern::StringId>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

pub(crate) fn init_capture_map(compiler: &crate::compiler::compiler::FunctionCompiler) {
    let mut map = std::collections::HashMap::new();
    for scope in &compiler.scopes {
        for (&name, &slot) in scope {
            map.insert(slot as u32, name);
        }
    }
    HIR_CAPTURE_MAP.with(|c| {
        *c.borrow_mut() = map;
    });
}

pub(crate) fn collect_captures_hir(
    expr: &HirExpr,
    parent_locals: &std::collections::HashMap<crate::intern::StringId, usize>,
    local_idx_to_name: &std::collections::HashMap<u32, crate::intern::StringId>,
    out: &mut Vec<crate::intern::StringId>,
) {
    match &expr.kind {
        HirExprKind::Local(id) => {
            if let Some(&name) = local_idx_to_name.get(id) {
                if parent_locals.contains_key(&name) && !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        HirExprKind::Binary { left, right, .. } => {
            collect_captures_hir(left, parent_locals, local_idx_to_name, out);
            collect_captures_hir(right, parent_locals, local_idx_to_name, out);
        }
        HirExprKind::Unary { right, .. } => {
            collect_captures_hir(right, parent_locals, local_idx_to_name, out);
        }
        HirExprKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_captures_hir(arg.expr(), parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::MethodCall { receiver, args, .. } => {
            collect_captures_hir(receiver, parent_locals, local_idx_to_name, out);
            for arg in args {
                collect_captures_hir(arg.expr(), parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::SetLiteral { elements, range, .. } => {
            for e in elements {
                collect_captures_hir(e, parent_locals, local_idx_to_name, out);
            }
            if let Some(r) = range {
                collect_captures_hir(&r.start, parent_locals, local_idx_to_name, out);
                collect_captures_hir(&r.end, parent_locals, local_idx_to_name, out);
                if let Some(s) = &r.step {
                    collect_captures_hir(s, parent_locals, local_idx_to_name, out);
                }
            }
        }
        HirExprKind::ArrayLiteral { elements } => {
            for e in elements {
                collect_captures_hir(e, parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::ArrayOrSetLiteral { elements } => {
            for e in elements {
                collect_captures_hir(e, parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::RandomChoice { set } => {
            collect_captures_hir(set, parent_locals, local_idx_to_name, out);
        }
        HirExprKind::RandomInt { min, max, step } => {
            collect_captures_hir(min, parent_locals, local_idx_to_name, out);
            collect_captures_hir(max, parent_locals, local_idx_to_name, out);
            if let Some(s) = step {
                collect_captures_hir(s, parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::RandomFloat { min, max, step } => {
            collect_captures_hir(min, parent_locals, local_idx_to_name, out);
            collect_captures_hir(max, parent_locals, local_idx_to_name, out);
            if let Some(s) = step {
                collect_captures_hir(s, parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::MapLiteral { elements, .. } => {
            for (k, v) in elements {
                collect_captures_hir(k, parent_locals, local_idx_to_name, out);
                collect_captures_hir(v, parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::TableLiteral { rows, .. } => {
            for row in rows {
                for e in row {
                    collect_captures_hir(e, parent_locals, local_idx_to_name, out);
                }
            }
        }
        HirExprKind::DatabaseLiteral(elements) => {
            for (_, e) in elements {
                collect_captures_hir(e, parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::Index { receiver, index } => {
            collect_captures_hir(receiver, parent_locals, local_idx_to_name, out);
            collect_captures_hir(index, parent_locals, local_idx_to_name, out);
        }
        HirExprKind::MemberAccess { receiver, .. } => {
            collect_captures_hir(receiver, parent_locals, local_idx_to_name, out);
        }
        HirExprKind::TerminalCommand(_, args) => {
            for e in args {
                collect_captures_hir(e, parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::Lambda { body, .. } => {
            collect_captures_hir(body, parent_locals, local_idx_to_name, out);
        }
        HirExprKind::Tuple(elements) => {
            for e in elements {
                collect_captures_hir(e, parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::ModuleCall { args, .. } => {
            for arg in args {
                collect_captures_hir(arg.expr(), parent_locals, local_idx_to_name, out);
            }
        }
        HirExprKind::As { expr, .. } => {
            collect_captures_hir(expr, parent_locals, local_idx_to_name, out);
        }
        HirExprKind::Yield(expr) => {
            collect_captures_hir(expr, parent_locals, local_idx_to_name, out);
        }
        _ => {}
    }
}
