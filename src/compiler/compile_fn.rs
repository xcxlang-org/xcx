use std::sync::Arc;
use crate::frontend::ast::Stmt;
use crate::frontend::ast::Type;
use crate::intern::StringId;
use crate::vm::opcode::OpCode;
use crate::vm::opcode::Chunk;
use crate::vm::value::Value;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};

/// The default value a recovered call yields for a declared return type,
/// mirroring the language's default-value semantics (types.md). Complex
/// types have no single default — `None` leaves the call result unset,
/// as before.
pub(crate) fn return_default_for_type(ty: &Type) -> Option<Value> {
    match ty {
        Type::Int => Some(Value::from_i64(0)),
        Type::Float => Some(Value::from_f64(0.0)),
        Type::Bool => Some(Value::from_bool(false)),
        Type::String => Some(Value::from_string(Arc::new(crate::vm::object::StringObj::new(Vec::new())))),
        Type::Date => Some(Value::from_date(0)),
        _ => None,
    }
}

impl FunctionCompiler {
    pub fn compile_fn_def(&mut self, name: &StringId, params: &[(Type, StringId)], return_type: Option<&Type>, body: &[Stmt], stmt: &Stmt, ctx: &mut CompileContext) {
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
        let mut chunk = Chunk::new(fc.bytecode, fc.spans, false, fc.max_locals_used.max(fc.next_local), name_str, params.len());
        chunk.ret_default = return_type.and_then(return_default_for_type);
        let fid = ctx.func_indices.get(name).copied().unwrap_or(0);
        ctx.functions[fid] = std::sync::Arc::new(chunk);
    }
}
