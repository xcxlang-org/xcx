use crate::frontend::ast::{Stmt, StmtKind, Expr, ExprKind};
use crate::intern::StringId;
use crate::vm::opcode::OpCode;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};

impl FunctionCompiler {
    pub fn compile_var_decl(&mut self, stmt: &Stmt, ctx: &mut CompileContext) {
        // Const enforcement is handled by the semantic analysis pass;
        // the compile pass receives only valid programs.
        if let StmtKind::VarDecl { name, value, ty, is_const: _is_const } = &stmt.kind {
            let src = if let Some(v) = value {
                if let ExprKind::TableLiteral { .. } = &v.kind {
                    let name_str = ctx.interner.lookup(*name).to_string();
                    self.compile_table_literal(v, ctx, Some(name_str))
                } else {
                    self.compile_expr(v, ctx)
                }
            } else {
                let def = self.get_default_value(ty, ctx);
                let idx = ctx.add_constant(def);
                let dst = self.push_reg();
                self.emit(OpCode::LoadConst { dst, idx }, &stmt.span);
                dst
            };

            if self.is_main && self.scopes.len() == 1 && ctx.globals.contains_key(name) {
                let idx = *ctx.globals.get(name).unwrap();
                self.emit(OpCode::SetVar { idx: idx as u32, src }, &stmt.span);
                self.pop_reg();
            } else {
                let slot = if let Some(&s) = self.scopes.last().and_then(|scope| scope.get(name)) {
                    s 
                } else {
                    let s = src as usize;
                    self.define_local(*name, s);
                    if s >= self.next_local {
                        self.next_local = s + 1;
                        self.sync_max_locals();
                    }
                    s
                };
                self.local_types.insert(*name, ty.as_ref().clone());
                if (slot as u8) != src {
                    self.emit(OpCode::Move { dst: slot as u8, src }, &stmt.span);
                }
            }
        }
    }

    pub fn compile_database_decl(&mut self, name: &StringId, fields: &Vec<(StringId, Box<Expr>)>, ctx: &mut CompileContext, span: &crate::error::Span) {
        let mut engine_src = 0;
        let mut path_src = 0;
        
        let mut tables_base = self.next_local as u8;
        let mut table_count = 0;
        
        for (f_name, f_val) in fields {
            let n = ctx.interner.lookup(*f_name).to_string();
            if n == "engine" {
                engine_src = self.compile_expr(f_val, ctx);
            } else if n == "path" {
                path_src = self.compile_expr(f_val, ctx);
            } else {
                // Table
                let reg = self.compile_expr(f_val, ctx);
                if table_count == 0 { tables_base = reg; }
                table_count += 1;
            }
        }
        
        let dst = if let Some(slot) = self.lookup_local(name) { slot as u8 } else {
            let slot = self.push_reg();
            self.define_local(*name, slot as usize);
            slot
        };
        
        self.emit(OpCode::DatabaseInit { 
            dst, 
            engine_src, 
            path_src, 
            tables_base_reg: tables_base, 
            table_count 
        }, span);

        if self.is_main {
            if let Some(&idx) = ctx.globals.get(name) {
                self.emit(OpCode::SetVar { idx: idx as u32, src: dst }, span);
            }
        }
        
        self.next_local = (dst + 1) as usize;
        self.sync_max_locals();
    }
}
