use std::sync::Arc;
use crate::intern::StringId;
use crate::error::Span;
use crate::hir::hir::{HirExpr, HirExprKind, HirArgument};
use crate::vm::opcode::{OpCode, MethodKind, TypeTag, Chunk};
use crate::vm::value::Value;
use crate::vm::object::StringObj;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use super::compile_expr::{compile_expr, collect_captures_hir, HIR_CAPTURE_MAP};

pub(crate) fn compile_special_function_call(
    fc: &mut FunctionCompiler,
    name: StringId,
    args: &[HirArgument],
    expr_span: &Span,
    ctx: &mut CompileContext,
) -> Option<u8> {
    let name_str = ctx.interner.lookup(name).to_string();
    if name_str == "terminal.input" {
        let dst = fc.push_reg();
        fc.emit(OpCode::Input { dst, ty: TypeTag::Unknown }, expr_span);
        return Some(dst);
    }
    if args.len() == 1 {
        let src = compile_expr(fc, args[0].expr(), ctx);
        let is_v_local = fc.local_regs.contains(&(src as usize));
        let dst = if !is_v_local { src } else { fc.push_reg() };
        match name_str.as_str() {
            "i" => {
                fc.emit(OpCode::CastInt { dst, src }, expr_span);
                return Some(dst);
            }
            "f" => {
                fc.emit(OpCode::CastFloat { dst, src }, expr_span);
                return Some(dst);
            }
            "s" => {
                fc.emit(OpCode::CastString { dst, src }, expr_span);
                return Some(dst);
            }
            "b" => {
                fc.emit(OpCode::CastBool { dst, src }, expr_span);
                return Some(dst);
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn compile_special_method_call(
    fc: &mut FunctionCompiler,
    receiver: &HirExpr,
    method_name: &str,
    args: &[HirArgument],
    expr_span: &Span,
    wait_after: bool,
    ctx: &mut CompileContext,
) -> Option<u8> {
    // 1. input namespace checks
    if let HirExprKind::Global(rid) = &receiver.kind {
        let is_input = ctx.interner.lookup(*rid) == "input";
        if is_input {
            if method_name == "key" {
                let wait = wait_after || (if let Some(arg) = args.get(0) {
                    if let HirExprKind::Tag(t) = &arg.expr().kind {
                        ctx.interner.lookup(*t) == "wait"
                    } else { false }
                } else { false });
                let dst = fc.push_reg();
                if wait {
                    fc.emit(OpCode::InputKeyWait { dst }, expr_span);
                } else {
                    fc.emit(OpCode::InputKey { dst }, expr_span);
                }
                return Some(dst);
            }
            if method_name == "ready" {
                let dst = fc.push_reg();
                fc.emit(OpCode::InputReady { dst }, expr_span);
                return Some(dst);
            }
        }
    }

    // 2. json fast get push check
    if method_name == "push" && args.len() == 1 {
        if let HirExprKind::MethodCall { receiver: get_recv, method: get_method, args: get_args, .. } = &receiver.kind {
            let is_get = ctx.interner.lookup(*get_method) == "get";
            if is_get && get_args.len() == 1 {
                let json_src = compile_expr(fc, get_recv, ctx);
                let path_src = compile_expr(fc, get_args[0].expr(), ctx);
                let val_src = compile_expr(fc, args[0].expr(), ctx);
                
                let dst = fc.push_reg();
                fc.emit(OpCode::JsonFastGetPush { json_src, path_src, val_src }, expr_span);
                fc.pop_reg();
                return Some(dst);
            }
        }
    }

    // 3. Namespace/Module based method calls (date, json, env, crypto, net, perf)
    let mut is_date = false;
    let mut is_json = false;
    let mut is_env = false;
    let mut is_crypto = false;
    let mut is_net = false;
    let mut is_perf = false;

    if let HirExprKind::Global(rid) = &receiver.kind {
        let rname = ctx.interner.lookup(*rid);
        is_date = rname == "date";
        is_json = rname == "json";
        is_env = rname == "env";
        is_crypto = rname == "crypto";
        is_net = rname == "net";
        is_perf = rname == "perf";
    }

    if is_net {
        if method_name == "serve" {
            let mut port_src = 255u8;
            let mut host_src = 255u8;
            let mut workers_src = 255u8;
            let mut routes_src = 255u8;
            for arg in args {
                let arg_name = match arg {
                    HirArgument::Named(id, _) => Some(ctx.interner.lookup(*id).to_string()),
                    HirArgument::Positional(_) => None,
                };
                match arg_name.as_deref() {
                    Some("port") => port_src = compile_expr(fc, arg.expr(), ctx),
                    Some("host") => host_src = compile_expr(fc, arg.expr(), ctx),
                    Some("workers") => workers_src = compile_expr(fc, arg.expr(), ctx),
                    Some("routes") => routes_src = compile_expr(fc, arg.expr(), ctx),
                    _ => {}
                }
            }
            let p = if port_src == 255 {
                let i = ctx.add_constant(Value::from_i64(8080));
                let r = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: r, idx: i }, expr_span);
                r
            } else { port_src };
            let h = if host_src == 255 {
                let i = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(b"0.0.0.0".to_vec()))));
                let r = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: r, idx: i }, expr_span);
                r
            } else { host_src };
            let w = if workers_src == 255 {
                let i = ctx.add_constant(Value::from_i64(4));
                let r = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: r, idx: i }, expr_span);
                r
            } else { workers_src };
            let r = if routes_src == 255 {
                let i = ctx.add_constant(Value::from_bool(false));
                let reg = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: reg, idx: i }, expr_span);
                reg
            } else { routes_src };
            fc.emit(OpCode::HttpServe { func_idx: 0, port_src: p, host_src: h, workers_src: w, routes_src: r }, expr_span);
            return Some(p);
        }
        if method_name == "respond" && args.len() >= 2 {
            let status_src = compile_expr(fc, args[0].expr(), ctx);
            let body_src = compile_expr(fc, args[1].expr(), ctx);
            let headers_src = if let Some(arg) = args.get(2) {
                compile_expr(fc, arg.expr(), ctx)
            } else {
                let f = ctx.add_constant(Value::from_bool(false));
                let r = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: r, idx: f }, expr_span);
                r
            };
            let dst = fc.push_reg();
            fc.emit(OpCode::HttpRespond { dst, status_src, body_src, headers_src }, expr_span);
            return Some(dst);
        }
    }

    if is_date && method_name == "now" {
        let dst = fc.push_reg();
        fc.emit(OpCode::DateNow { dst }, expr_span);
        return Some(dst);
    }

    if is_perf {
        let dst = fc.push_reg();
        match method_name {
            "focus" | "ms" => fc.emit(OpCode::PerfMs { dst }, expr_span),
            "us" => fc.emit(OpCode::PerfUs { dst }, expr_span),
            "ns" => fc.emit(OpCode::PerfNs { dst }, expr_span),
            _ => {}
        }
        return Some(dst);
    }

    if is_json && method_name == "parse" && !args.is_empty() {
        let src = compile_expr(fc, args[0].expr(), ctx);
        let dst = src;
        fc.emit(OpCode::JsonParse { dst, src }, expr_span);
        return Some(dst);
    }

    if is_env {
        let dst = fc.push_reg();
        if method_name == "get" {
            if let Some(arg_node) = args.first() {
                let src = compile_expr(fc, arg_node.expr(), ctx);
                fc.emit(OpCode::EnvGet { dst, src }, expr_span);
                fc.pop_reg();
            }
        } else if method_name == "args" {
            fc.emit(OpCode::EnvArgs { dst }, expr_span);
        }
        return Some(dst);
    }

    if is_crypto {
        let base = fc.next_local as u8;
        let mut arg_count = 0u8;
        for arg in args {
            let arg_reg = base + arg_count;
            fc.next_local = arg_reg as usize;
            let src = compile_expr(fc, arg.expr(), ctx);
            if src != arg_reg { fc.emit(OpCode::Move { dst: arg_reg, src }, expr_span); }
            arg_count += 1;
        }
        let dst = base;
        match method_name {
            "hash"   => fc.emit(OpCode::CryptoHash { dst, pass_src: base, alg_src: base + 1 }, expr_span),
            "verify" => fc.emit(OpCode::CryptoVerify { dst, pass_src: base, hash_src: base + 1, alg_src: base + 2 }, expr_span),
            "token"  => fc.emit(OpCode::CryptoToken { dst, len_src: base }, expr_span),
            _ => {}
        }
        return Some(dst);
    }

    // 4. Sqlite Table query optimization: .where(...)
    if method_name == "where" {
        let base = fc.next_local as u8;
        let receiver_reg = compile_expr(fc, receiver, ctx);
        if receiver_reg != base {
            fc.emit(OpCode::Move { dst: base, src: receiver_reg }, expr_span);
        }
        fc.next_local = (base + 1) as usize;
        fc.sync_max_locals();
        if let Some(dst) = compile_query_where_hir(fc, expr_span, base, args, ctx) {
            return Some(dst);
        }
    }

    None
}

pub(crate) fn compile_query_where_hir(
    fc: &mut FunctionCompiler,
    expr_span: &Span,
    base: u8,
    args: &[HirArgument],
    ctx: &mut CompileContext,
) -> Option<u8> {
    if args.len() == 1 {
        let arg_expr = args[0].expr();
        if !matches!(arg_expr.kind, HirExprKind::Lambda { .. }) {
            let flat_locals = fc.convert_to_flat_locals();
            let mut local_idx_to_name = std::collections::HashMap::new();
            HIR_CAPTURE_MAP.with(|c| {
                local_idx_to_name = c.borrow().clone();
            });
            
            let mut captures = Vec::new();
            collect_captures_hir(arg_expr, &flat_locals, &local_idx_to_name, &mut captures);

            let mut sub = FunctionCompiler::new(false, Some(flat_locals));
            sub.is_table_lambda = true;
            
            for id in &captures {
                sub.lookup_local(id);
            }
            sub.next_local = 1 + captures.len(); 
            
            let res = compile_expr(&mut sub, arg_expr, ctx);
            sub.emit(OpCode::Return { src: res }, &arg_expr.span);
            
            let captures_to_pass = sub.captures.clone();
            
            let fid = ctx.functions.len();
            let has_loops = crate::vm::opcode::calculate_has_loops(&sub.bytecode);
            let chunk = Chunk::new(
                sub.bytecode,
                sub.spans,
                false,
                sub.max_locals_used.max(sub.next_local),
                has_loops,
                "query_where".to_string(),
                1,
            );
            ctx.functions.push(std::sync::Arc::new(chunk));
            
            let f_val = Value::from_function(fid as u32);
            let f_idx = ctx.add_constant(f_val);
            let f_reg = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst: f_reg, idx: f_idx }, &arg_expr.span);
            
            for &cap_id in &captures_to_pass {
                if let Some(slot) = fc.lookup_local(&cap_id) {
                    let r = fc.push_reg();
                    fc.emit(OpCode::Move { dst: r, src: slot as u8 }, &arg_expr.span);
                } else {
                    fc.push_reg(); 
                }
            }
            
            let dst = base;
            fc.emit(
                OpCode::MethodCall {
                    dst,
                    kind: MethodKind::Where,
                    base,
                    arg_count: (1 + captures_to_pass.len()) as u8,
                },
                expr_span,
            );
            fc.next_local = (base + 1) as usize;
            fc.sync_max_locals();
            return Some(dst);
        }
    }
    None
}
