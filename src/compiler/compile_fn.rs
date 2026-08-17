use crate::frontend::ast::Stmt;
use crate::intern::StringId;
use crate::vm::opcode::OpCode;
use crate::vm::opcode::Chunk;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};

impl FunctionCompiler {
    pub fn compile_fn_def(&mut self, name: &StringId, params: &[(crate::frontend::ast::Type, StringId)], body: &[Stmt], stmt: &Stmt, ctx: &mut CompileContext) {
        let mut fc = FunctionCompiler::new(false, None);
        for (i, (ty, pname)) in params.iter().enumerate() {
            fc.define_local(*pname, i);
            fc.local_types.insert(*pname, ty.clone());
        }
        fc.next_local = params.len();
        for s in body { fc.compile_stmt(s, ctx); }
        if fc.bytecode.is_empty() || !matches!(fc.bytecode.last(), Some(OpCode::Return { .. }) | Some(OpCode::ReturnVoid)) {
            fc.emit(OpCode::ReturnVoid, &stmt.span);
        }
        let name_str = ctx.interner.lookup(*name).to_string();
        let chunk = Chunk::new(fc.bytecode, fc.spans, false, fc.max_locals_used.max(fc.next_local), name_str, params.len());
        let fid = ctx.func_indices.get(name).copied().unwrap_or(0);
        ctx.functions[fid] = std::sync::Arc::new(chunk);
    }
}
