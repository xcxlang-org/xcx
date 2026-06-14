use std::sync::Arc;
use crate::frontend::ast::{Expr, ExprKind};
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use crate::vm::opcode::{OpCode, MethodKind};
use crate::vm::value::Value;
use crate::vm::object::StringObj;

pub fn compile(fc: &mut FunctionCompiler, expr: &Expr, ctx: &mut CompileContext) -> u8 {
    match &expr.kind {
        ExprKind::MemberAccess { receiver, member } => {
            let base = fc.next_local as u8;
            let receiver_reg = fc.compile_expr(receiver, ctx);
            if receiver_reg != base { fc.emit(OpCode::Move { dst: base, src: receiver_reg }, &expr.span); }
            fc.next_local = (base + 1) as usize;
            let method_name = ctx.interner.lookup(*member).to_string();
            let dst = fc.push_reg();

            let is_property = match method_name.as_str() {
                "length" | "year" | "month" | "day" | "hour" | "minute" | "second" | "affected" | "insertId" | "status" | "ok" | "error" => true,
                _ => false
            };

            if is_property {
                let name_idx = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(method_name.clone().into_bytes()))));
                fc.emit(OpCode::GetMember { dst, container: base, name_idx }, &expr.span);
            } else if let Some(kind) = fc.map_method_kind(&method_name) {
                fc.emit(OpCode::MethodCall { dst, kind, base, arg_count: 0 }, &expr.span);
            } else {
                let mi = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(method_name.into_bytes()))));
                fc.emit(OpCode::MethodCallCustom { dst, method_name_idx: mi, base, arg_count: 0 }, &expr.span);
            }
            fc.next_local = (dst + 1) as usize;
            dst
        }
        ExprKind::Index { receiver, index } => {
            let base = fc.next_local as u8;
            let r_src = fc.compile_expr(receiver, ctx);
            if r_src != base { fc.emit(OpCode::Move { dst: base, src: r_src }, &expr.span); }
            fc.next_local = (base + 1) as usize; 
            let i_src = fc.compile_expr(index, ctx);
            let i_dst = base + 1;
            if i_src != i_dst { fc.emit(OpCode::Move { dst: i_dst, src: i_src }, &expr.span); }
            fc.next_local = (i_dst + 1) as usize;
            let dst = fc.push_reg();
            fc.emit(OpCode::MethodCall { dst, kind: MethodKind::Get, base, arg_count: 1 }, &expr.span);
            fc.next_local = (dst + 1) as usize;
            dst
        }
        ExprKind::As { expr: e, name } => {
            let src = fc.compile_expr(e, ctx);
            if let Some(slot) = fc.lookup_local(name) {
                fc.emit(OpCode::Move { dst: slot as u8, src }, &expr.span);
                src
            } else if let Some(&idx) = ctx.globals.get(name) {
                fc.emit(OpCode::SetVar { idx: idx as u32, src }, &expr.span);
                src
            } else {
                let slot = src as usize;
                fc.define_local(*name, slot);
                if slot >= fc.next_local {
                    fc.next_local = slot + 1;
                    fc.sync_max_locals();
                }
                src
            }
        }
        ExprKind::Tuple(exprs) => {
            let base = fc.next_local as u8;
            for (i, e) in exprs.iter().enumerate() {
                let dst = base + i as u8;
                fc.next_local = dst as usize;
                let src = fc.compile_expr(e, ctx);
                if src != dst { fc.emit(OpCode::Move { dst, src }, &expr.span); }
                fc.next_local = (dst + 1) as usize;
                fc.sync_max_locals();
            }
            let dst = base;
            fc.emit(OpCode::ArrayInit { dst, base, count: exprs.len() as u32 }, &expr.span);
            fc.next_local = (base + 1) as usize;
            fc.sync_max_locals();
            dst
        }
        _ => 0,
    }
}
