use crate::frontend::ast::{Stmt, StmtKind};
use crate::vm::opcode::{OpCode, MethodKind};
use crate::vm::opcode::Chunk;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};

impl FunctionCompiler {
    pub(crate) fn compile_fiber_def(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        if let StmtKind::FiberDef { name, params, body, .. } = &stmt.kind {
            let mut fc = FunctionCompiler::new(false, None);
            for (i, (_, pname)) in params.iter().enumerate() {
                fc.define_local(*pname, i);
            }
            fc.next_local = params.len();
            for s in body { fc.compile_stmt(s, ctx); }
            if fc.bytecode.is_empty() || !matches!(fc.bytecode.last(), Some(OpCode::Return { .. }) | Some(OpCode::ReturnVoid)) {
                fc.emit(OpCode::ReturnVoid, &stmt.span);
            }
            let has_loops = crate::vm::opcode::calculate_has_loops(&fc.bytecode);
            let name_str = ctx.interner.lookup(*name).to_string();
            let chunk = Chunk::new(fc.bytecode, fc.spans, true, fc.max_locals_used.max(fc.next_local), has_loops, name_str, params.len());
            let fid = ctx.func_indices.get(name).copied().unwrap_or(0);
            ctx.functions[fid] = std::sync::Arc::new(chunk);
        }
    }

    pub(crate) fn compile_fiber_decl(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        if let StmtKind::FiberDecl { name, fiber_name, args, .. } = &stmt.kind {
            let base = self.next_local as u8;
            for (i, arg) in args.iter().enumerate() {
                let dst = base + i as u8;
                let src = self.compile_expr(arg.expr(), ctx);
                if src != dst { self.emit(OpCode::Move { dst, src }, &stmt.span); }
                self.next_local = (dst + 1) as usize;
            }
            let f_idx = ctx.func_indices.get(fiber_name).copied().unwrap_or(0);
            let dst = if let Some(s) = self.lookup_local(name) { s as u8 } else {
                let s = self.push_reg();
                self.define_local(*name, s as usize);
                s
            };
            self.emit(OpCode::FiberCreate { dst, func_idx: f_idx as u32, base, arg_count: args.len() as u8 }, &stmt.span);
            self.next_local = (dst + 1) as usize; self.sync_max_locals();
        }
    }

    pub(crate) fn compile_yield(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        if let StmtKind::Yield { value, target } = &stmt.kind {
            let src = self.compile_expr(value, ctx);
            if let Some(t_id) = target {
                let dst = if let Some(slot) = self.lookup_local(t_id) { slot as u8 } else {
                    let slot = self.push_reg();
                    self.define_local(*t_id, slot as usize);
                    slot
                };
                self.emit(OpCode::YieldWithTarget { dst, src }, &stmt.span);
                self.next_local = (dst + 1) as usize;
            } else {
                self.emit(OpCode::Yield { src }, &stmt.span);
                self.pop_reg();
            }
        }
    }

    pub(crate) fn compile_yield_from(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        if let StmtKind::YieldFrom(expr) = &stmt.kind {
            let fiber_reg = self.compile_expr(expr, ctx);
            let start_label = self.bytecode.len();
            let test_reg = self.push_reg();
            self.emit(OpCode::MethodCall { dst: test_reg, kind: MethodKind::IsDone, base: fiber_reg, arg_count: 0 }, &stmt.span);
            let exit_jmp = self.bytecode.len();
            self.emit(OpCode::JumpIfTrue { src: test_reg, target: 0 }, &stmt.span);
            self.pop_reg();
            let val_reg = self.push_reg();
            self.emit(OpCode::MethodCall { dst: val_reg, kind: MethodKind::Next, base: fiber_reg, arg_count: 0 }, &stmt.span);
            let test_reg2 = self.push_reg();
            self.emit(OpCode::MethodCall { dst: test_reg2, kind: MethodKind::IsDone, base: fiber_reg, arg_count: 0 }, &stmt.span);
            let skip_jmp = self.bytecode.len();
            self.emit(OpCode::JumpIfTrue { src: test_reg2, target: 0 }, &stmt.span);
            self.pop_reg(); 
            self.emit(OpCode::Yield { src: val_reg }, &stmt.span);
            let skip_target = self.bytecode.len() as u32;
            if let OpCode::JumpIfTrue { ref mut target, .. } = self.bytecode[skip_jmp] { *target = skip_target; }
            self.pop_reg(); 
            self.emit(OpCode::Jump { target: start_label as u32 }, &stmt.span);
            let end_label = self.bytecode.len() as u32;
            if let OpCode::JumpIfTrue { ref mut target, .. } = self.bytecode[exit_jmp] { *target = end_label; }
            self.next_local = fiber_reg as usize;
        }
    }

    pub(crate) fn compile_yield_void(&mut self, stmt: &Stmt) {
        self.emit(OpCode::YieldVoid, &stmt.span);
    }
}
