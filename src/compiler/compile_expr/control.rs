use crate::frontend::ast::{Expr, ExprKind};
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use crate::vm::opcode::{OpCode, Chunk};
use crate::vm::value::Value;

pub fn compile(fc: &mut FunctionCompiler, expr: &Expr, ctx: &mut CompileContext) -> u8 {
    match &expr.kind {
        ExprKind::Lambda { params, body, .. } => {
            let flat_locals = fc.convert_to_flat_locals();
            let mut captures = Vec::new();
            fc.collect_captures(body, &flat_locals, &mut captures);
            let mut sub = FunctionCompiler::new(false, Some(flat_locals));
            for (i, (_, param_name)) in params.iter().enumerate() {
                sub.define_local(*param_name, i);
            }
            for id in &captures { sub.lookup_local(id); }
            sub.next_local = params.len() + captures.len();
            let res = sub.compile_expr(body, ctx);
            sub.emit(OpCode::Return { src: res }, &expr.span);
            let fid = ctx.functions.len();
            ctx.functions.push(std::sync::Arc::new(Chunk::new(sub.bytecode, sub.spans, false, sub.max_locals_used.max(sub.next_local), "lambda".to_string(), params.len())));
            let f_val = Value::from_function(fid as u32);
            let f_idx = ctx.add_constant(f_val);
            let dst = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst, idx: f_idx }, &expr.span);
            dst
        }
        ExprKind::Yield(v) => {
            let src = fc.compile_expr(v, ctx);
            fc.emit(OpCode::Yield { src }, &expr.span);
            src
        }
        _ => 0,
    }
}
