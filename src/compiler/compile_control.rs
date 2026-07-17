use crate::frontend::ast::{Stmt, StmtKind, ForIterType};
use crate::vm::value::Value;
use crate::vm::opcode::{OpCode, MethodKind};
use crate::compiler::compiler::{FunctionCompiler, CompileContext, LoopFrame};

impl FunctionCompiler {
    pub(crate) fn compile_if(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        if let StmtKind::If { condition, then_branch, else_ifs, else_branch } = &stmt.kind {
            let mut end_jumps = Vec::new();
            let cond_reg = self.compile_expr(condition, ctx);
            let jmp_idx = self.bytecode.len();
            self.emit(OpCode::JumpIfFalse { src: cond_reg, target: 0 }, &stmt.span);
            self.pop_reg(); 
            
            let saved_nl = self.enter_scope();
            for s in then_branch { self.compile_stmt(s, ctx); }
            self.exit_scope(saved_nl);
            
            if !else_ifs.is_empty() || else_branch.is_some() {
                end_jumps.push(self.bytecode.len());
                self.emit(OpCode::Jump { target: 0 }, &stmt.span);
            }
            let jump_target = self.bytecode.len() as u32;
            if let OpCode::JumpIfFalse { ref mut target, .. } = self.bytecode[jmp_idx] { *target = jump_target; }

            for (elif_cond, elif_branch) in else_ifs {
                let elif_cond_reg = self.compile_expr(elif_cond, ctx);
                let elif_jmp = self.bytecode.len();
                self.emit(OpCode::JumpIfFalse { src: elif_cond_reg, target: 0 }, &stmt.span);
                self.pop_reg();
                let saved_elif = self.enter_scope();
                for s in elif_branch { self.compile_stmt(s, ctx); }
                self.exit_scope(saved_elif);
                end_jumps.push(self.bytecode.len());
                self.emit(OpCode::Jump { target: 0 }, &stmt.span);
                let elif_target = self.bytecode.len() as u32;
                if let OpCode::JumpIfFalse { ref mut target, .. } = self.bytecode[elif_jmp] { *target = elif_target; }
            }
            if let Some(branch) = else_branch {
                let saved_else = self.enter_scope();
                for s in branch { self.compile_stmt(s, ctx); }
                self.exit_scope(saved_else);
            }
            let final_idx = self.bytecode.len() as u32;
            for idx in end_jumps {
                if let OpCode::Jump { ref mut target } = self.bytecode[idx] { *target = final_idx; }
            }
        }
    }

    pub(crate) fn compile_while(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        use crate::frontend::ast::ExprKind;
        use crate::frontend::lexer::TokenKind;

        if let StmtKind::While { condition, body } = &stmt.kind {
            let mut is_opt_candidate = false;
            let mut loop_counter_id = None;
            let mut limit_expr = None;
            let mut is_less = false;
            let mut is_greater = false;
            let mut is_greater_equal = false;

            if let ExprKind::Binary { left, op, right } = &condition.kind {
                if *op == TokenKind::Less || *op == TokenKind::LessEqual {
                    if let ExprKind::Identifier(id) = &left.kind {
                        if self.lookup_local(id).is_some() {
                            loop_counter_id = Some(*id);
                            limit_expr = Some(right.as_ref());
                            is_less = *op == TokenKind::Less;
                            is_opt_candidate = true;
                        }
                    }
                } else if *op == TokenKind::Greater || *op == TokenKind::GreaterEqual {
                    if let ExprKind::Identifier(id) = &left.kind {
                        if self.lookup_local(id).is_some() {
                            loop_counter_id = Some(*id);
                            limit_expr = Some(right.as_ref());
                            is_greater = *op == TokenKind::Greater;
                            is_greater_equal = *op == TokenKind::GreaterEqual;
                            is_opt_candidate = true;
                        }
                    }
                }
            }

            self.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: None });
            let is_downward = is_greater || is_greater_equal;

            if is_opt_candidate {
                let counter_name = loop_counter_id.unwrap();
                let counter_reg = self.lookup_local(&counter_name).unwrap() as u8;
                let original_limit_reg = self.compile_expr(limit_expr.unwrap(), ctx);

                let limit_reg = if is_less {
                    let one_idx = ctx.add_constant(Value::from_i64(1));
                    let one_reg = self.push_reg();
                    self.emit(OpCode::LoadConst { dst: one_reg, idx: one_idx }, &stmt.span);
                    self.emit(OpCode::Sub { dst: original_limit_reg, src1: original_limit_reg, src2: one_reg }, &stmt.span);
                    self.pop_reg(); // free one_reg
                    original_limit_reg
                } else if is_greater {
                    let one_idx = ctx.add_constant(Value::from_i64(1));
                    let one_reg = self.push_reg();
                    self.emit(OpCode::LoadConst { dst: one_reg, idx: one_idx }, &stmt.span);
                    self.emit(OpCode::Add { dst: original_limit_reg, src1: original_limit_reg, src2: one_reg }, &stmt.span);
                    self.pop_reg(); // free one_reg
                    original_limit_reg
                } else {
                    original_limit_reg
                };

                let start_p = self.bytecode.len();
                if let Some(l) = self.loop_stack.last_mut() { l.start_pc = start_p; }

                let test_reg = self.push_reg();
                if is_downward {
                    self.emit(OpCode::GreaterEqual { dst: test_reg, src1: counter_reg, src2: limit_reg }, &stmt.span);
                } else {
                    self.emit(OpCode::LessEqual { dst: test_reg, src1: counter_reg, src2: limit_reg }, &stmt.span);
                }
                let exit_jmp = self.bytecode.len();
                self.emit(OpCode::JumpIfFalse { src: test_reg, target: 0 }, &stmt.span);
                self.pop_reg(); // free test_reg

                let body_p = self.bytecode.len();
                let saved_while = self.enter_scope();
                for s in body { self.compile_stmt(s, ctx); }
                self.exit_scope(saved_while);

                let len = self.bytecode.len();
                let mut fused = false;
                if len > 0 {
                    if !is_downward {
                        if let OpCode::IncLocal { reg } = self.bytecode[len - 1] {
                            if reg == counter_reg {
                                self.bytecode.pop();
                                self.spans.pop();
                                self.emit(OpCode::LoopNext { reg: counter_reg, limit_reg, target: body_p as u32 }, &stmt.span);
                                fused = true;
                            }
                        }
                    } else {
                        if let OpCode::DecLocal { reg } = self.bytecode[len - 1] {
                            if reg == counter_reg {
                                self.bytecode.pop();
                                self.spans.pop();
                                self.emit(OpCode::LoopPrev { reg: counter_reg, limit_reg, target: body_p as u32 }, &stmt.span);
                                fused = true;
                            }
                        }
                    }
                }

                if !fused {
                    self.emit(OpCode::Jump { target: start_p as u32 }, &stmt.span);
                }

                let exit_target = self.bytecode.len() as u32;
                if let OpCode::JumpIfFalse { ref mut target, .. } = self.bytecode[exit_jmp] { *target = exit_target; }

                let frame = self.loop_stack.pop().unwrap();
                let breaks = frame.breaks;
                let continues = frame.continues;
                let end_label = self.bytecode.len() as u32;
                for b in breaks { if let OpCode::Jump { ref mut target } = self.bytecode[b] { *target = end_label; } }
                for c in continues { if let OpCode::Jump { ref mut target } = self.bytecode[c] { *target = start_p as u32; } }

                self.pop_reg(); // free limit_reg
            } else {
                let start_p = self.bytecode.len();
                if let Some(l) = self.loop_stack.last_mut() { l.start_pc = start_p; }
                let cond_reg = self.compile_expr(condition, ctx);
                let exit_jmp = self.bytecode.len();
                self.emit(OpCode::JumpIfFalse { src: cond_reg, target: 0 }, &stmt.span);
                self.pop_reg();
                let saved_while = self.enter_scope();
                for s in body { self.compile_stmt(s, ctx); }
                self.exit_scope(saved_while);
                self.emit(OpCode::Jump { target: start_p as u32 }, &stmt.span);
                let exit_target = self.bytecode.len() as u32;
                if let OpCode::JumpIfFalse { ref mut target, .. } = self.bytecode[exit_jmp] { *target = exit_target; }
                let frame = self.loop_stack.pop().unwrap();
                let breaks = frame.breaks;
                let continues = frame.continues;
                let end_label = self.bytecode.len() as u32;
                for b in breaks { if let OpCode::Jump { ref mut target } = self.bytecode[b] { *target = end_label; } }
                for c in continues { if let OpCode::Jump { ref mut target } = self.bytecode[c] { *target = start_p as u32; } }
            }
        }
    }

    pub(crate) fn compile_for(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        if let StmtKind::For { var_name, start, end, step, body, iter_type } = &stmt.kind {
            match iter_type {
                ForIterType::Array | ForIterType::Set => {
                    let source_reg_raw = self.compile_expr(start, ctx);
                    self.pop_reg();
                    let receiver_reg = self.push_reg();
                    if *iter_type == ForIterType::Set {
                        self.emit(OpCode::MethodCall { dst: receiver_reg, kind: MethodKind::Values, base: source_reg_raw, arg_count: 0 }, &stmt.span);
                    } else {
                        self.emit(OpCode::Move { dst: receiver_reg, src: source_reg_raw }, &stmt.span);
                    }
                    let arg_reg = self.push_reg();
                    let size_reg = self.push_reg();
                    let index_reg = self.push_reg();
                    self.emit(OpCode::MethodCall { dst: size_reg, kind: MethodKind::Size, base: receiver_reg, arg_count: 0 }, &stmt.span);
                    let zero_idx = ctx.add_constant(Value::from_i64(0));
                    self.emit(OpCode::LoadConst { dst: index_reg, idx: zero_idx }, &stmt.span);
                    let loop_var_reg = if let Some(s) = self.lookup_local(var_name) {
                        let r = s as u8; if s >= self.next_local { self.next_local = s + 1; } r
                    } else {
                        let s = self.push_reg(); self.define_local(*var_name, s as usize); s
                    };
                    let saved_for = self.enter_scope();
                    self.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: None });
                    let start_label = self.bytecode.len();
                    if let Some(l) = self.loop_stack.last_mut() { l.start_pc = start_label; }
                    let saved_next_local = self.next_local;
                    let test_reg = self.push_reg();
                    self.emit(OpCode::Less { dst: test_reg, src1: index_reg, src2: size_reg }, &stmt.span);
                    let exit_jmp = self.bytecode.len();
                    self.emit(OpCode::JumpIfFalse { src: test_reg, target: 0 }, &stmt.span);
                    self.pop_reg(); 
                    let body_start = self.bytecode.len();
                    self.emit(OpCode::Move { dst: arg_reg, src: index_reg }, &stmt.span);
                    if *iter_type == ForIterType::Set {
                        self.emit(OpCode::GetIndex { dst: loop_var_reg, container: receiver_reg, index: arg_reg }, &stmt.span);
                    } else {
                        self.emit(OpCode::MethodCall { dst: loop_var_reg, kind: MethodKind::Get, base: receiver_reg, arg_count: 1 }, &stmt.span);
                    }
                    for s in body { self.compile_stmt(s, ctx); }
                    self.next_local = saved_next_local as usize;
                    let cont_label = self.bytecode.len();
                    self.emit(OpCode::ArrayLoopNext { idx_reg: index_reg, size_reg, target: body_start as u32 }, &stmt.span);
                    let end_label = self.bytecode.len() as u32;
                    if let OpCode::JumpIfFalse { ref mut target, .. } = self.bytecode[exit_jmp] { *target = end_label; }
                    let frame = self.loop_stack.pop().unwrap();
                    let breaks = frame.breaks;
                    let continues = frame.continues;
                    self.exit_scope(saved_for);
                    for b in breaks { if let OpCode::Jump { ref mut target } = self.bytecode[b] { *target = end_label; } }
                    for c in continues { if let OpCode::Jump { ref mut target } = self.bytecode[c] { *target = cont_label as u32; } }
                    self.next_local = receiver_reg as usize;
                }
                ForIterType::Range => {
                    let start_reg_raw = self.compile_expr(start, ctx);
                    self.pop_reg();
                    let loop_var_reg = self.push_reg(); 
                    self.emit(OpCode::Move { dst: loop_var_reg, src: start_reg_raw }, &stmt.span);
                    self.define_local(*var_name, loop_var_reg as usize);
                    let limit_reg = self.compile_expr(end, ctx);
                    let saved_for = self.enter_scope();
                    self.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: None });
                    let start_p = self.bytecode.len();
                    if let Some(l) = self.loop_stack.last_mut() { l.start_pc = start_p; }
                    let saved_next_local = self.next_local;
                    let test_reg = self.push_reg();
                    self.emit(OpCode::LessEqual { dst: test_reg, src1: loop_var_reg, src2: limit_reg }, &stmt.span);
                    let exit_jmp = self.bytecode.len();
                    self.emit(OpCode::JumpIfFalse { src: test_reg, target: 0 }, &stmt.span);
                    self.pop_reg();
                    let body_p = self.bytecode.len();
                    let mut last_stmt_start = body_p;
                    let body_len = body.len();
                    for (i, s) in body.iter().enumerate() {
                        if i == body_len - 1 { last_stmt_start = self.bytecode.len(); }
                        self.compile_stmt(s, ctx);
                    }
                    self.next_local = saved_next_local;
                    let mut cont_label = self.bytecode.len();
                    if step.is_none() {
                        let len = self.bytecode.len();
                        let mut fused = false;
                        let has_continues = if let Some(l) = self.loop_stack.last() {
                            !l.continues.is_empty()
                        } else {
                            false
                        };
                        if len == last_stmt_start + 1 && !has_continues {
                            match self.bytecode[len - 1] {
                                OpCode::IncVar { idx } => {
                                    self.bytecode.pop(); self.spans.pop();
                                    cont_label = self.bytecode.len();
                                    self.emit(OpCode::IncVarLoopNext { g_idx: idx, reg: loop_var_reg, limit_reg, target: body_p as u32 }, &stmt.span);
                                    fused = true;
                                }
                                OpCode::IncLocal { reg } => {
                                    self.bytecode.pop(); self.spans.pop();
                                    cont_label = self.bytecode.len();
                                    self.emit(OpCode::IncLocalLoopNext { inc_reg: reg, reg: loop_var_reg, limit_reg, target: body_p as u32 }, &stmt.span);
                                    fused = true;
                                }
                                _ => {}
                            }
                        }
                        if !fused { self.emit(OpCode::LoopNext { reg: loop_var_reg, limit_reg, target: body_p as u32 }, &stmt.span); }
                    } else {
                        let step_reg = self.compile_expr(step.as_ref().unwrap(), ctx);
                        self.emit(OpCode::Add { dst: loop_var_reg, src1: loop_var_reg, src2: step_reg }, &stmt.span);
                        self.emit(OpCode::Jump { target: start_p as u32 }, &stmt.span);
                        self.pop_reg();
                    }
                    let end_label = self.bytecode.len() as u32;
                    if let OpCode::JumpIfFalse { ref mut target, .. } = self.bytecode[exit_jmp] { *target = end_label; }
                    let frame = self.loop_stack.pop().unwrap();
                    let breaks = frame.breaks;
                    let continues = frame.continues;
                    self.exit_scope(saved_for);
                    for b in breaks { if let OpCode::Jump { ref mut target } = self.bytecode[b] { *target = end_label; } }
                    for c in continues { if let OpCode::Jump { ref mut target } = self.bytecode[c] { *target = cont_label as u32; } }
                    self.next_local = loop_var_reg as usize;
                }
                ForIterType::Fiber => {
                    let fiber_reg = self.compile_expr(start, ctx);
                    let loop_var_reg = if let Some(s) = self.lookup_local(var_name) {
                        let r = s as u8; if s >= self.next_local { self.next_local = s + 1; } r
                    } else {
                        let s = self.push_reg(); self.define_local(*var_name, s as usize); s
                    };
                    let saved_for = self.enter_scope();
                    self.loop_stack.push(LoopFrame { start_pc: 0, breaks: Vec::new(), continues: Vec::new(), fiber_reg: Some(fiber_reg as usize) });
                    let start_label = self.bytecode.len();
                    if let Some(l) = self.loop_stack.last_mut() { l.start_pc = start_label; }
                    let saved_next_local = self.next_local;
                    let test_reg = self.push_reg();
                    self.emit(OpCode::MethodCall { dst: test_reg, kind: MethodKind::IsDone, base: fiber_reg, arg_count: 0 }, &stmt.span);
                    let exit_jmp = self.bytecode.len();
                    self.emit(OpCode::JumpIfTrue { src: test_reg, target: 0 }, &stmt.span);
                    self.pop_reg();
                    self.emit(OpCode::MethodCall { dst: loop_var_reg, kind: MethodKind::Next, base: fiber_reg, arg_count: 0 }, &stmt.span);
                    for s in body { self.compile_stmt(s, ctx); }
                    self.next_local = saved_next_local;
                    let cont_label = self.bytecode.len();
                    self.emit(OpCode::Jump { target: start_label as u32 }, &stmt.span);
                    let end_label = self.bytecode.len() as u32;
                    if let OpCode::JumpIfTrue { ref mut target, .. } = self.bytecode[exit_jmp] { *target = end_label; }
                    let frame = self.loop_stack.pop().unwrap();
                    let breaks = frame.breaks;
                    let continues = frame.continues;
                    self.exit_scope(saved_for);
                    for b in breaks { if let OpCode::Jump { ref mut target } = self.bytecode[b] { *target = end_label; } }
                    for c in continues { if let OpCode::Jump { ref mut target } = self.bytecode[c] { *target = cont_label as u32; } }
                    self.next_local = fiber_reg as usize;
                }
            }
        }
    }

    pub(crate) fn compile_break(&mut self, stmt: &Stmt) {
        if let Some(&LoopFrame { fiber_reg: Some(fiber_reg_idx), .. }) = self.loop_stack.last() {
            let tmp = self.push_reg();
            self.emit(OpCode::MethodCall { dst: tmp, kind: MethodKind::Close, base: fiber_reg_idx as u8, arg_count: 0 }, &stmt.span);
            self.pop_reg();
        }
        let jmp = self.bytecode.len();
        self.emit(OpCode::Jump { target: 0 }, &stmt.span);
        if let Some(l) = self.loop_stack.last_mut() { l.breaks.push(jmp); }
    }

    pub(crate) fn compile_continue(&mut self, stmt: &Stmt) {
        let jmp = self.bytecode.len();
        self.emit(OpCode::Jump { target: 0 }, &stmt.span);
        if let Some(l) = self.loop_stack.last_mut() { l.continues.push(jmp); }
    }
}
