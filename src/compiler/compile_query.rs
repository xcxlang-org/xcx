use crate::frontend::ast::{Expr, ExprKind};
use crate::vm::opcode::{OpCode, MethodKind};
use crate::vm::value::Value;
use crate::vm::opcode::Chunk;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};

impl FunctionCompiler {
    pub(crate) fn compile_query_where(&mut self, expr: &Expr, base: u8, args: &[crate::frontend::ast::Argument], ctx: &mut CompileContext) -> Option<u8> {
        if args.len() == 1 {
            if !matches!(args[0].expr().kind, ExprKind::Lambda { .. }) {
                let flat_locals = self.convert_to_flat_locals();
                let mut captures = Vec::new();
                self.collect_captures(args[0].expr(), &flat_locals, &mut captures);

                let mut sub = FunctionCompiler::new(false, Some(flat_locals));
                sub.is_table_lambda = true;
                
                for id in &captures { sub.lookup_local(id); }
                sub.next_local = 1 + captures.len(); 
                
                let res = sub.compile_expr(args[0].expr(), ctx);
                sub.emit(OpCode::Return { src: res }, &args[0].expr().span);
                
                let captures_to_pass = sub.captures.clone();
                
                let fid = ctx.functions.len();
                let has_loops = crate::vm::opcode::calculate_has_loops(&sub.bytecode);
                let chunk = Chunk::new(sub.bytecode, sub.spans, false, sub.max_locals_used.max(sub.next_local), has_loops, "query_where".to_string(), 1);
                ctx.functions.push(std::sync::Arc::new(chunk));
                
                let f_val = Value::from_function(fid as u32);
                let f_idx = ctx.add_constant(f_val);
                let f_reg = self.push_reg();
                self.emit(OpCode::LoadConst { dst: f_reg, idx: f_idx }, &args[0].expr().span);
                
                for &cap_id in &captures_to_pass {
                    if let Some(slot) = self.lookup_local(&cap_id) {
                        let r = self.push_reg();
                        self.emit(OpCode::Move { dst: r, src: slot as u8 }, &args[0].expr().span);
                    } else {
                        self.push_reg(); 
                    }
                }
                
                let dst = base;
                self.emit(OpCode::MethodCall { dst, kind: MethodKind::Where, base, arg_count: (1 + captures_to_pass.len()) as u8 }, &expr.span);
                self.next_local = (base + 1) as usize;
                self.sync_max_locals();
                return Some(dst);
            }
        }
        None
    }
}
