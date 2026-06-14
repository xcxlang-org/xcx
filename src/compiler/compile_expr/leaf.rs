use crate::frontend::ast::{Expr, ExprKind};
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::object::StringObj;
use std::sync::Arc;

pub fn compile(fc: &mut FunctionCompiler, expr: &Expr, ctx: &mut CompileContext) -> u8 {
    match &expr.kind {
        ExprKind::Tag(id) => {
            let s = ctx.interner.lookup(*id).to_string();
            let i = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(s.into_bytes()))));
            let dst = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
            dst
        }
        ExprKind::IntLiteral(v) => {
            let i = ctx.add_constant(Value::from_i64(*v));
            let dst = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
            dst
        }
        ExprKind::FloatLiteral(v) => {
            let i = ctx.add_constant(Value::from_f64(*v));
            let dst = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
            dst
        }
        ExprKind::StringLiteral(id) => {
            let s = ctx.interner.lookup(*id).to_string();
            let i = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(s.into_bytes()))));
            let dst = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
            dst
        }
        ExprKind::BoolLiteral(v) => {
            let i = ctx.add_constant(Value::from_bool(*v));
            let dst = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
            dst
        }
        ExprKind::Identifier(id) => {
            if let Some(slot) = fc.lookup_local(id) {
                let dst = fc.push_reg();
                fc.emit(OpCode::Move { dst, src: slot as u8 }, &expr.span);
                dst
            } else if let Some(&idx) = ctx.globals.get(id) {
                let dst = fc.push_reg();
                fc.emit(OpCode::GetVar { dst, idx: idx as u32 }, &expr.span);
                dst
            } else if fc.is_table_lambda {
                let dst = fc.push_reg();
                let mi = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(ctx.interner.lookup(*id).to_string().into_bytes()))));
                fc.emit(OpCode::MethodCallCustom { dst, method_name_idx: mi, base: 0, arg_count: 0 }, &expr.span);
                dst
            } else if let Some(&fid) = ctx.func_indices.get(id) {
                let dst = fc.push_reg();
                let i = ctx.add_constant(Value::from_function(fid as u32));
                fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
                dst
            } else {
                let dst = fc.push_reg();
                let name = ctx.interner.lookup(*id).to_string();
                let i = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(name.into_bytes()))));
                fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
                dst
            }
        }
        ExprKind::RawBlock(id) => {
            let s = ctx.interner.lookup(*id).to_string();
            let i = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(s.into_bytes()))));
            let dst = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
            dst
        }
        _ => 0,
    }
}
