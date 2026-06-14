use std::sync::Arc;
use crate::frontend::ast::{Expr, ExprKind, Argument};
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use crate::vm::opcode::{OpCode, MethodKind, TypeTag};
use crate::vm::value::Value;
use crate::vm::object::StringObj;

pub fn compile(fc: &mut FunctionCompiler, expr: &Expr, ctx: &mut CompileContext) -> u8 {
    match &expr.kind {
        ExprKind::FunctionCall { name, args } => {
            let n = ctx.interner.lookup(*name);
            if n == "terminal.input" {
                let dst = fc.push_reg();
                fc.emit(OpCode::Input { dst, ty: TypeTag::Unknown }, &expr.span);
                dst
            } else if n == "i" && args.len() == 1 {
                let src = fc.compile_expr(args[0].expr(), ctx);
                let dst = src;
                fc.emit(OpCode::CastInt { dst, src }, &expr.span);
                dst
            } else if n == "f" && args.len() == 1 {
                let src = fc.compile_expr(args[0].expr(), ctx);
                let dst = src;
                fc.emit(OpCode::CastFloat { dst, src }, &expr.span);
                dst
            } else if n == "s" && args.len() == 1 {
                let src = fc.compile_expr(args[0].expr(), ctx);
                let dst = src;
                fc.emit(OpCode::CastString { dst, src }, &expr.span);
                dst
            } else if n == "b" && args.len() == 1 {
                let src = fc.compile_expr(args[0].expr(), ctx);
                let dst = src;
                fc.emit(OpCode::CastBool { dst, src }, &expr.span);
                dst
            } else {
                let base = fc.next_local as u8;
                let mut arg_count = 0u8;
                for arg in args {
                    let arg_reg = base + arg_count;
                    fc.next_local = arg_reg as usize;
                    let src = fc.compile_expr(arg.expr(), ctx);
                    if src != arg_reg {
                        fc.emit(OpCode::Move { dst: arg_reg, src }, &expr.span);
                    }
                    arg_count += 1;
                }

                let dst = base;
                if let Some(&fid) = ctx.func_indices.get(name) {
                    if ctx.functions[fid].is_fiber {
                        fc.emit(OpCode::FiberCreate { dst, func_idx: fid as u32, base, arg_count }, &expr.span);
                    } else {
                        fc.emit(OpCode::Call { dst, func_idx: fid as u32, base, arg_count }, &expr.span);
                    }
                } else {
                    fc.emit(OpCode::Halt, &expr.span);
                }
                fc.next_local = (base + 1) as usize;
                fc.sync_max_locals();
                dst
            }
        }
        ExprKind::MethodCall { receiver, method, args, wait_after } => {
            let method_name = ctx.interner.lookup(*method).to_string();
            let mut is_date = false;
            let mut is_json = false;
            let mut is_env = false;
            let mut is_crypto = false;
            let mut is_net = false;
            let mut is_perf = false;
            
            if let ExprKind::Identifier(rid) = &receiver.kind {
                let rname = ctx.interner.lookup(*rid);
                if rname == "date" { is_date = true; }
                if rname == "json" { is_json = true; }
                if rname == "env" { is_env = true; }
                if rname == "crypto" { is_crypto = true; }
                if rname == "net" { is_net = true; }
                if rname == "perf" { is_perf = true; }
                if rname == "input" {
                    if method_name == "key" {
                        let wait = *wait_after || (if let Some(arg) = args.get(0) {
                            if let ExprKind::Tag(t) = &arg.expr().kind {
                                ctx.interner.lookup(*t) == "wait"
                            } else { false }
                        } else { false });
                        let dst = fc.push_reg();
                        if wait {
                            fc.emit(OpCode::InputKeyWait { dst }, &expr.span);
                        } else {
                            fc.emit(OpCode::InputKey { dst }, &expr.span);
                        }
                        return dst;
                    }
                    if method_name == "ready" {
                        let dst = fc.push_reg();
                        fc.emit(OpCode::InputReady { dst }, &expr.span);
                        return dst;
                    }
                }
            }
            
            if is_net {
                match method_name.as_str() {
                    "serve" => {
                        let mut port_src = 255u8;
                        let mut host_src = 255u8;
                        let mut workers_src = 255u8;
                        let mut routes_src = 255u8;
                        for arg in args {
                            let arg_name = match arg {
                                Argument::Named(id, _) => Some(ctx.interner.lookup(*id)),
                                Argument::Positional(_) => None,
                            };
                            match arg_name {
                                Some("port") => port_src = fc.compile_expr(arg.expr(), ctx),
                                Some("host") => host_src = fc.compile_expr(arg.expr(), ctx),
                                Some("workers") => workers_src = fc.compile_expr(arg.expr(), ctx),
                                Some("routes") => routes_src = fc.compile_expr(arg.expr(), ctx),
                                _ => {}
                            }
                        }
                        let p = if port_src == 255 {
                            let i = ctx.add_constant(Value::from_i64(8080));
                            let r = fc.push_reg();
                            fc.emit(OpCode::LoadConst { dst: r, idx: i }, &expr.span);
                            r
                        } else { port_src };
                        let h = if host_src == 255 {
                            let i = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(b"0.0.0.0".to_vec()))));
                            let r = fc.push_reg();
                            fc.emit(OpCode::LoadConst { dst: r, idx: i }, &expr.span);
                            r
                        } else { host_src };
                        let w = if workers_src == 255 {
                            let i = ctx.add_constant(Value::from_i64(4));
                            let r = fc.push_reg();
                            fc.emit(OpCode::LoadConst { dst: r, idx: i }, &expr.span);
                            r
                        } else { workers_src };
                        let r = if routes_src == 255 {
                            let i = ctx.add_constant(Value::from_bool(false));
                            let reg = fc.push_reg();
                            fc.emit(OpCode::LoadConst { dst: reg, idx: i }, &expr.span);
                            reg
                        } else { routes_src };
                        fc.emit(OpCode::HttpServe { func_idx: 0, port_src: p, host_src: h, workers_src: w, routes_src: r }, &expr.span);
                        return p;
                    }
                    "respond" => {
                        if args.len() >= 2 {
                            let status_src = fc.compile_expr(args[0].expr(), ctx);
                            let body_src = fc.compile_expr(args[1].expr(), ctx);
                            let headers_src = if let Some(arg) = args.get(2) {
                                fc.compile_expr(arg.expr(), ctx)
                            } else {
                                let f = ctx.add_constant(Value::from_bool(false));
                                let r = fc.push_reg();
                                fc.emit(OpCode::LoadConst { dst: r, idx: f }, &expr.span);
                                r
                            };
                            let dst = fc.push_reg();
                            fc.emit(OpCode::HttpRespond { dst, status_src, body_src, headers_src }, &expr.span);
                            return dst;
                        }
                    }
                    _ => {}
                }
            }
            if is_date && method_name == "now" {
                let dst = fc.push_reg();
                fc.emit(OpCode::DateNow { dst }, &expr.span);
                return dst;
            }
            if is_perf {
                let dst = fc.push_reg();
                match method_name.as_str() {
                    "ms" => fc.emit(OpCode::PerfMs { dst }, &expr.span),
                    "us" => fc.emit(OpCode::PerfUs { dst }, &expr.span),
                    "ns" => fc.emit(OpCode::PerfNs { dst }, &expr.span),
                    _ => {}
                }
                return dst;
            }
            if is_json && method_name == "parse" {
                let src = fc.compile_expr(args[0].expr(), ctx);
                let dst = src;
                fc.emit(OpCode::JsonParse { dst, src }, &expr.span);
                return dst;
            }
            if is_env {
                let dst = fc.push_reg();
                if method_name == "get" {
                    if let Some(arg_node) = args.first() {
                        let src = fc.compile_expr(arg_node.expr(), ctx);
                        fc.emit(OpCode::EnvGet { dst, src }, &expr.span);
                        fc.pop_reg();
                    }
                } else if method_name == "args" {
                    fc.emit(OpCode::EnvArgs { dst }, &expr.span);
                }
                return dst;
            }
            if is_crypto {
                let base = fc.next_local as u8;
                let mut arg_count = 0u8;
                for arg in args {
                    let arg_reg = base + arg_count;
                    fc.next_local = arg_reg as usize;
                    let src = fc.compile_expr(arg.expr(), ctx);
                    if src != arg_reg { fc.emit(OpCode::Move { dst: arg_reg, src }, &expr.span); }
                    arg_count += 1;
                }
                let dst = base;
                match method_name.as_str() {
                    "hash"   => fc.emit(OpCode::CryptoHash { dst, pass_src: base, alg_src: base + 1 }, &expr.span),
                    "verify" => fc.emit(OpCode::CryptoVerify { dst, pass_src: base, hash_src: base + 1, alg_src: base + 2 }, &expr.span),
                    "token"  => fc.emit(OpCode::CryptoToken { dst, len_src: base }, &expr.span),
                    _ => {}
                };
                fc.next_local = (base + 1) as usize;
                return dst;
            }

            if method_name == "push" && args.len() == 1 {
                if let ExprKind::MethodCall { receiver: get_recv, method: get_method, args: get_args, .. } = &receiver.kind {
                    if ctx.interner.lookup(*get_method) == "get" && get_args.len() == 1 {
                        let json_src = fc.compile_expr(get_recv, ctx);
                        let path_src = fc.compile_expr(get_args[0].expr(), ctx);
                        let val_src = fc.compile_expr(args[0].expr(), ctx);
                        
                        let dst = fc.push_reg();
                        fc.emit(OpCode::JsonFastGetPush { json_src, path_src, val_src }, &expr.span);
                        fc.pop_reg(); // Push returns void
                        return dst;
                    }
                }
            }

            let base = fc.compile_expr(receiver, ctx);

            if method_name == "where" {
                if let Some(dst) = fc.compile_query_where(expr, base, args, ctx) {
                    return dst;
                }
            }

            let mut arg_names = Vec::new();
            let mut has_named = false;
            for arg in args {
                if let Argument::Named(id, _) = arg {
                    has_named = true;
                    arg_names.push(ctx.interner.lookup(*id).to_string());
                } else {
                    arg_names.push(String::new());
                }
            }

            let mut arg_count = 0u8;
            for arg in args {
                let arg_reg = base + 1 + arg_count;
                fc.next_local = arg_reg as usize;
                let src = fc.compile_expr(arg.expr(), ctx);
                if src != arg_reg { fc.emit(OpCode::Move { dst: arg_reg, src }, &arg.expr().span); }
                arg_count += 1;
                
                if let ExprKind::Lambda { body, .. } = &arg.expr().kind {
                    let flat_locals = fc.convert_to_flat_locals();
                    let mut captures = Vec::new();
                    fc.collect_captures(body, &flat_locals, &mut captures);
                    for id in &captures {
                        if let Some(slot) = fc.lookup_local(id) {
                            let r_cap = base + 1 + arg_count;
                            fc.next_local = r_cap as usize;
                            fc.emit(OpCode::Move { dst: r_cap, src: slot as u8 }, &arg.expr().span);
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

            if is_builtin_property && arg_count == 0 {
                let name_idx = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(method_name.clone().into_bytes()))));
                fc.emit(OpCode::GetMember { dst, container: base, name_idx }, &expr.span);
            } else if let Some(kind) = fc.map_method_kind(&method_name) {
                if has_named {
                    let names_val = Value::from_string_array(Arc::new(arg_names));
                    let names_idx = ctx.add_constant(names_val);
                    fc.emit(OpCode::MethodCallNamed { dst, kind, base, arg_count, names_idx }, &expr.span);
                } else {
                    fc.emit(OpCode::MethodCall { dst, kind, base, arg_count }, &expr.span);
                }
            } else {
                let mi = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(method_name.clone().into_bytes()))));
                fc.emit(OpCode::MethodCallCustom { dst, method_name_idx: mi, base, arg_count }, &expr.span);
            }
            
            if method_name == "bind" && args.len() == 2 {
                if let ExprKind::Identifier(target_id) = &args[1].expr().kind {
                    if let Some(slot) = fc.lookup_local(target_id) {
                        let dst_slot = slot as u8;
                        let src_slot = base + 2; 
                        fc.emit(OpCode::Move { dst: dst_slot, src: src_slot }, &expr.span);
                    }
                }
            }
            fc.next_local = base as usize + 1;
            fc.sync_max_locals();
            dst
        }
        ExprKind::ModuleCall { module, method, args } => {
            let method_name = ctx.interner.lookup(*method).to_string();
            match module {
                crate::frontend::lexer::TokenKind::Net => {
                    match method_name.as_str() {
                        "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
                            let url_src = fc.compile_expr(args[0].expr(), ctx);
                            let body_src = if let Some(arg) = args.get(1) {
                                fc.compile_expr(arg.expr(), ctx)
                            } else {
                                let f = ctx.add_constant(Value::from_bool(false));
                                let r = fc.push_reg();
                                fc.emit(OpCode::LoadConst { dst: r, idx: f }, &expr.span);
                                r
                            };
                            let method_idx = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(method_name.into_bytes()))));
                            let dst = url_src;
                            fc.emit(OpCode::HttpCall { dst, method_idx, url_src, body_src }, &expr.span);
                            fc.next_local = (dst + 1) as usize; fc.sync_max_locals();
                            dst
                        }
                        "respond" => {
                            let status_src = fc.compile_expr(args[0].expr(), ctx);
                            let body_src   = fc.compile_expr(args[1].expr(), ctx);
                            let headers_src = if let Some(arg) = args.get(2) {
                                fc.compile_expr(arg.expr(), ctx)
                            } else {
                                let f = ctx.add_constant(Value::from_bool(false));
                                let r = fc.push_reg();
                                fc.emit(OpCode::LoadConst { dst: r, idx: f }, &expr.span);
                                r
                            };
                            let dst = status_src;
                            fc.emit(OpCode::HttpRespond { dst, status_src, body_src, headers_src }, &expr.span);
                            fc.next_local = (dst + 1) as usize;
                            dst
                        }
                        _ => fc.push_reg()
                    }
                }
                crate::frontend::lexer::TokenKind::Json => {
                    match method_name.as_str() {
                        "parse" => {
                            let src = fc.compile_expr(args[0].expr(), ctx);
                            let dst = src;
                            fc.emit(OpCode::JsonParse { dst, src }, &expr.span);
                            fc.next_local = (dst + 1) as usize;
                            dst
                        }
                        "toStr" | "stringify" => {
                            let base = fc.next_local as u8;
                            let receiver_reg = fc.compile_expr(args[0].expr(), ctx);
                            if receiver_reg != base { fc.emit(OpCode::Move { dst: base, src: receiver_reg }, &expr.span); }
                            fc.next_local = (base + 1) as usize;
                            let dst = fc.push_reg();
                            fc.emit(OpCode::MethodCall { dst, kind: MethodKind::ToStr, base, arg_count: 0 }, &expr.span);
                            fc.next_local = (dst + 1) as usize;
                            dst
                        }
                        _ => fc.push_reg()
                    }
                }
                crate::frontend::lexer::TokenKind::Crypto => {
                    match method_name.as_str() {
                        "hash" => {
                            let pass_src = fc.compile_expr(args[0].expr(), ctx);
                            let alg_src = if let Some(arg) = args.get(1) {
                                fc.compile_expr(arg.expr(), ctx)
                            } else {
                                let s = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(b"sha256".to_vec()))));
                                let r = fc.push_reg();
                                fc.emit(OpCode::LoadConst { dst: r, idx: s }, &expr.span);
                                r
                            };
                            let dst = pass_src;
                            fc.emit(OpCode::CryptoHash { dst, pass_src, alg_src }, &expr.span);
                            fc.next_local = (dst + 1) as usize;
                            fc.sync_max_locals();
                            dst
                        }
                        "verify" => {
                            let pass_src = fc.compile_expr(args[0].expr(), ctx);
                            let hash_src = fc.compile_expr(args[1].expr(), ctx);
                            let alg_src = if let Some(arg) = args.get(2) {
                                fc.compile_expr(arg.expr(), ctx)
                            } else {
                                let s = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(b"sha256".to_vec()))));
                                let r = fc.push_reg();
                                fc.emit(OpCode::LoadConst { dst: r, idx: s }, &expr.span);
                                r
                            };
                            let dst = pass_src;
                            fc.emit(OpCode::CryptoVerify { dst, pass_src, hash_src, alg_src }, &expr.span);
                            fc.next_local = (dst + 1) as usize;
                            fc.sync_max_locals();
                            dst
                        }
                        "token" => {
                            let len_src = if let Some(arg) = args.get(0) {
                                fc.compile_expr(arg.expr(), ctx)
                            } else {
                                let i = ctx.add_constant(Value::from_i64(32));
                                let r = fc.push_reg();
                                fc.emit(OpCode::LoadConst { dst: r, idx: i }, &expr.span);
                                r
                            };
                            let dst = len_src;
                            fc.emit(OpCode::CryptoToken { dst, len_src }, &expr.span);
                            fc.next_local = (dst + 1) as usize;
                            fc.sync_max_locals();
                            dst
                        }
                        _ => fc.push_reg()
                    }
                }
                crate::frontend::lexer::TokenKind::Env => {
                    match method_name.as_str() {
                        "get" => {
                            let src = fc.compile_expr(args[0].expr(), ctx);
                            let dst = src;
                            fc.emit(OpCode::EnvGet { dst, src }, &expr.span);
                            fc.next_local = (dst + 1) as usize;
                            dst
                        }
                        "args" => {
                            let dst = fc.push_reg();
                            fc.emit(OpCode::EnvArgs { dst }, &expr.span);
                            dst
                        }
                        _ => fc.push_reg()
                    }
                }
                crate::frontend::lexer::TokenKind::Date => {
                    if method_name == "now" {
                        let dst = fc.push_reg();
                        fc.emit(OpCode::DateNow { dst }, &expr.span);
                        dst
                    } else { fc.push_reg() }
                }
                crate::frontend::lexer::TokenKind::Perf => {
                    let dst = fc.push_reg();
                    match method_name.as_str() {
                        "ms" => fc.emit(OpCode::PerfMs { dst }, &expr.span),
                        "us" => fc.emit(OpCode::PerfUs { dst }, &expr.span),
                        "ns" => fc.emit(OpCode::PerfNs { dst }, &expr.span),
                        _ => {}
                    }
                    dst
                }
                crate::frontend::lexer::TokenKind::Store => {
                    let base = fc.next_local as u8;
                    let mut arg_count = 0u8;
                    for arg in args {
                        let arg_reg = base + arg_count;
                        fc.next_local = arg_reg as usize;
                        let src = fc.compile_expr(arg.expr(), ctx);
                        if src != arg_reg { fc.emit(OpCode::Move { dst: arg_reg, src }, &expr.span); }
                        arg_count += 1;
                    }
                    fc.next_local = (base + arg_count) as usize;
                    fc.sync_max_locals();
                    let dst = base;
                match method_name.as_str() {
                        "write"  => fc.emit(OpCode::StoreWrite { dst, base }, &expr.span),
                        "read"   => fc.emit(OpCode::StoreRead { dst, base }, &expr.span),
                        "append" => fc.emit(OpCode::StoreAppend { dst, base }, &expr.span),
                        "exists" => fc.emit(OpCode::StoreExists { dst, base }, &expr.span),
                        "delete" => fc.emit(OpCode::StoreDelete { dst, base }, &expr.span),
                        "list"   => fc.emit(OpCode::StoreList { dst, base }, &expr.span),
                        "isDir"  => fc.emit(OpCode::StoreIsDir { dst, base }, &expr.span),
                        "size"   => fc.emit(OpCode::StoreSize { dst, base }, &expr.span),
                        "mkdir"  => fc.emit(OpCode::StoreMkdir { dst, base }, &expr.span),
                        "glob"   => fc.emit(OpCode::StoreGlob { dst, base }, &expr.span),
                        "zip"    => fc.emit(OpCode::StoreZip { dst, base }, &expr.span),
                        "unzip"  => fc.emit(OpCode::StoreUnzip { dst, base }, &expr.span),
                        _ => { 
                            return fc.push_reg();
                        }
                };
                fc.next_local = (dst + 1) as usize;
                dst
                }
                _ => fc.push_reg(),
            }
        }
        ExprKind::TerminalCommand(cmd_id, args) => {
            let cmd = ctx.interner.lookup(*cmd_id);
            let dst = fc.push_reg();
            if cmd == "exit" { fc.emit(OpCode::TerminalExit { dst }, &expr.span); }
            else if cmd == "clear" { fc.emit(OpCode::TerminalClear { dst }, &expr.span); }
            else if cmd == "run" {
                if let Some(a) = args.get(0) {
                    let cmd_src = fc.compile_expr(a, ctx);
                    fc.emit(OpCode::TerminalRun { dst, cmd_src }, &expr.span);
                    fc.next_local = (dst + 1) as usize; 
                }
            } else if cmd == "raw" { fc.emit(OpCode::TerminalRaw { dst }, &expr.span); }
            else if cmd == "normal" || cmd == "cooked" { fc.emit(OpCode::TerminalNormal { dst }, &expr.span); }
            else if cmd == "cursor" {
                if let Some(a) = args.get(0) {
                    let val_str = match &a.kind {
                        ExprKind::Identifier(id) => ctx.interner.lookup(*id),
                        _ => "",
                    };
                    if val_str == "on" { fc.emit(OpCode::TerminalCursor { dst, on: true }, &expr.span); }
                    else if val_str == "off" { fc.emit(OpCode::TerminalCursor { dst, on: false }, &expr.span); }
                }
            } else if cmd == "move" {
                if args.len() >= 2 {
                    let x_src = fc.compile_expr(&args[0], ctx);
                    let y_src = fc.compile_expr(&args[1], ctx);
                    fc.emit(OpCode::TerminalMove { dst, x_src, y_src }, &expr.span);
                    fc.next_local = (dst + 1) as usize;
                }
            }
            fc.sync_max_locals();
            dst
        }
        _ => 0,
    }
}
