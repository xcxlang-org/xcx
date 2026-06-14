use crate::frontend::ast::{Stmt, StmtKind, ExprKind, Type, HaltLevel};
use crate::vm::opcode::{OpCode, TypeTag};
use crate::vm::value::Value;
use crate::frontend::lexer::TokenKind;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};

impl FunctionCompiler {
    pub fn compile_stmt(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        match &stmt.kind {
            StmtKind::VarDecl { .. } => {
                self.compile_var_decl(stmt, ctx);
            }
            StmtKind::Print(expr) => {
                let src = self.compile_expr(expr, ctx);
                self.emit(OpCode::Print { src }, &stmt.span);
                self.pop_reg();
            }
            StmtKind::TerminalWrite(expr) => {
                let src = self.compile_expr(expr, ctx);
                let dst = self.push_reg();
                self.emit(OpCode::TerminalWrite { dst, src }, &stmt.span);
                self.pop_reg(); // pop dst
                self.pop_reg(); // pop src
            }
            StmtKind::FunctionCallStmt { name, args } => {
                let base = self.next_local as u8;
                for (i, arg) in args.iter().enumerate() {
                    let dst = (base as usize + i) as u8;
                    let src = self.compile_expr(arg.expr(), ctx);
                    if src != dst { self.emit(OpCode::Move { dst, src }, &stmt.span); }
                    self.next_local = dst as usize + 1;
                }
                if let Some(&func_id) = ctx.func_indices.get(name) {
                    let dst = base;
                    self.emit(OpCode::Call { dst, func_idx: func_id as u32, base, arg_count: args.len() as u8 }, &stmt.span);
                }
                self.next_local = base as usize;
            }
            StmtKind::Input(name, ty) => {
                let dst = if let Some(slot) = self.lookup_local(name) { slot as u8 } else {
                    let slot = self.push_reg() as usize;
                    self.define_local(*name, slot);
                    slot as u8
                };
                let type_tag = match **ty {
                    Type::Int => TypeTag::Int,
                    Type::Float => TypeTag::Float,
                    Type::String => TypeTag::String,
                    Type::Bool => TypeTag::Bool,
                    _ => TypeTag::Unknown,
                };
                self.emit(OpCode::Input { dst, ty: type_tag }, &stmt.span);
            }
            StmtKind::Assign { name, value } => {
                let mut optimized = false;
                if let ExprKind::Binary { left, op, right } = &value.kind {
                    if *op == TokenKind::Plus {
                        let is_inc = match (&left.kind, &right.kind) {
                            (ExprKind::Identifier(id), ExprKind::IntLiteral(1)) if id == name => true,
                            (ExprKind::IntLiteral(1), ExprKind::Identifier(id)) if id == name => true,
                            _ => false,
                        };
                        if is_inc {
                            if let Some(slot) = self.lookup_local(name) {
                                self.emit(OpCode::IncLocal { reg: slot as u8 }, &stmt.span);
                                optimized = true;
                            } else if let Some(&global_idx) = ctx.globals.get(name) {
                                self.emit(OpCode::IncVar { idx: global_idx as u32 }, &stmt.span);
                                optimized = true;
                            }
                        }
                    } else if *op == TokenKind::Minus {
                        let is_dec = match (&left.kind, &right.kind) {
                            (ExprKind::Identifier(id), ExprKind::IntLiteral(1)) if id == name => true,
                            _ => false,
                        };
                        if is_dec {
                            if let Some(slot) = self.lookup_local(name) {
                                self.emit(OpCode::DecLocal { reg: slot as u8 }, &stmt.span);
                                optimized = true;
                            } else if let Some(&global_idx) = ctx.globals.get(name) {
                                self.emit(OpCode::DecVar { idx: global_idx as u32 }, &stmt.span);
                                optimized = true;
                            }
                        }
                    }
                }
                if !optimized {
                    let src = self.compile_expr(value, ctx);
                    if let Some(slot) = self.lookup_local(name) { self.emit(OpCode::Move { dst: slot as u8, src }, &stmt.span); }
                    else if let Some(&global_idx) = ctx.globals.get(name) { self.emit(OpCode::SetVar { idx: global_idx as u32, src }, &stmt.span); }
                    self.pop_reg();
                }
            }
            StmtKind::If { .. } => {
                self.compile_if(stmt, ctx);
            }
            StmtKind::While { .. } => {
                self.compile_while(stmt, ctx);
            }
            StmtKind::For { .. } => {
                self.compile_for(stmt, ctx);
            }
            StmtKind::Break => {
                self.compile_break(stmt);
            }
            StmtKind::Continue => {
                self.compile_continue(stmt);
            }
            StmtKind::ExprStmt(expr) => {
                self.compile_expr(expr, ctx);
                self.pop_reg();
            }
            StmtKind::Halt { level, message } => {
                let src = self.compile_expr(message, ctx);
                match level {
                    HaltLevel::Alert => self.emit(OpCode::HaltAlert { src }, &stmt.span),
                    HaltLevel::Error => self.emit(OpCode::HaltError { src }, &stmt.span),
                    HaltLevel::Fatal => self.emit(OpCode::HaltFatal { src }, &stmt.span),
                }
                self.pop_reg();
            }
            StmtKind::Return(expr) => {
                if let Some(e) = expr {
                    let src = self.compile_expr(e, ctx);
                    self.emit(OpCode::Return { src }, &stmt.span);
                    self.pop_reg();
                } else {
                    self.emit(OpCode::ReturnVoid, &stmt.span);
                }
            }
            StmtKind::FunctionDef { name, params, body, .. } => {
                self.compile_fn_def(name, params, body, stmt, ctx);
            }
            StmtKind::FiberDef { .. } => {
                self.compile_fiber_def(stmt, ctx);
            }
            StmtKind::FiberDecl { .. } => {
                self.compile_fiber_decl(stmt, ctx);
            }
            StmtKind::JsonBind { json, path, target } => {
                let json_src = self.compile_expr(json, ctx);
                let path_src = self.compile_expr(path, ctx);
                if let Some(local_idx) = self.lookup_local(target) { self.emit(OpCode::JsonBindLocal { dst: local_idx as u8, json_src, path_src }, &stmt.span); }
                else {
                    let idx = ctx.globals.get(target).copied().unwrap_or(0);
                    self.emit(OpCode::JsonBind { idx: idx as u32, json_src, path_src }, &stmt.span);
                }
                self.next_local = json_src as usize;
            }
            StmtKind::JsonInject { json, mapping, table } => {
                let json_src = self.compile_expr(json, ctx);
                let mapping_src = self.compile_expr(mapping, ctx);
                if let Some(local_idx) = self.lookup_local(table) { self.emit(OpCode::JsonInjectLocal { table_reg: local_idx as u8, json_src, mapping_src }, &stmt.span); }
                else {
                    let idx = ctx.globals.get(table).copied().unwrap_or(0);
                    self.emit(OpCode::JsonInject { table_idx: idx as u32, json_src, mapping_src }, &stmt.span);
                }
                self.next_local = json_src as usize;
            }
            StmtKind::Yield { value: _, target: _ } => {
                self.compile_yield(stmt, ctx);
            }
            StmtKind::YieldFrom(..) => {
                self.compile_yield_from(stmt, ctx);
            }
            StmtKind::YieldVoid => { self.compile_yield_void(stmt); }
            StmtKind::DatabaseDecl { name, fields } => {
                self.compile_database_decl(name, fields, ctx, &stmt.span);
            }
            StmtKind::Wait(expr) => {
                let src = self.compile_expr(expr, ctx);
                self.emit(OpCode::Wait { src }, &stmt.span);
                self.pop_reg();
            }
            StmtKind::NetRequestStmt { method, url, headers, body, timeout, target } => {
                let mut elements = Vec::new();
                elements.push((crate::frontend::ast::Expr { kind: ExprKind::StringLiteral(ctx.interner.intern("method")), span: crate::error::Span::default() }, *method.clone()));
                elements.push((crate::frontend::ast::Expr { kind: ExprKind::StringLiteral(ctx.interner.intern("url")), span: crate::error::Span::default() }, *url.clone()));
                if let Some(h) = headers { elements.push((crate::frontend::ast::Expr { kind: ExprKind::StringLiteral(ctx.interner.intern("headers")), span: crate::error::Span::default() }, *h.clone())); }
                if let Some(b) = body { elements.push((crate::frontend::ast::Expr { kind: ExprKind::StringLiteral(ctx.interner.intern("body")), span: crate::error::Span::default() }, *b.clone())); }
                if let Some(t) = timeout { elements.push((crate::frontend::ast::Expr { kind: ExprKind::StringLiteral(ctx.interner.intern("timeout")), span: crate::error::Span::default() }, *t.clone())); }
                let map_expr = crate::frontend::ast::Expr { kind: ExprKind::MapLiteral { key_type: Box::new(Type::String), value_type: Box::new(Type::Json), elements }, span: crate::error::Span::default() };
                let arg_src = self.compile_expr(&map_expr, ctx);
                let dst = if let Some(slot) = self.lookup_local(target) { slot as u8 } else {
                    let s = self.next_local; self.define_local(*target, s); self.next_local += 1; s as u8
                };
                self.emit(OpCode::HttpRequest { dst, arg_src }, &stmt.span);
                if dst == arg_src + 1 {
                    self.next_local = (dst + 1) as usize;
                } else {
                    self.pop_reg();
                }
                self.sync_max_locals();
            }
            StmtKind::Serve { name, port, host, workers, routes } => {
                let port_src    = self.compile_expr(port, ctx);
                let host_src    = if let Some(h) = host { self.compile_expr(h, ctx) } 
                                  else { let i = ctx.add_constant(Value::from_bool(false)); let r = self.push_reg(); self.emit(OpCode::LoadConst { dst: r, idx: i }, &stmt.span); r };
                let workers_src = if let Some(w) = workers { self.compile_expr(w, ctx) } 
                                  else { let i = ctx.add_constant(Value::from_bool(false)); let r = self.push_reg(); self.emit(OpCode::LoadConst { dst: r, idx: i }, &stmt.span); r };
                let routes_src  = self.compile_expr(routes, ctx);
                let func_idx = ctx.func_indices.get(name).copied().unwrap_or(0);
                self.emit(OpCode::HttpServe { func_idx: func_idx as u32, port_src, host_src, workers_src, routes_src }, &stmt.span);
                self.next_local = port_src as usize; self.sync_max_locals();
            }
            _ => {}
        }
    }
}
