use crate::frontend::ast::{Expr, ExprKind};
use crate::vm::opcode::OpCode;
use crate::vm::value::Value;
use crate::vm::object::StringObj;
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use std::sync::Arc;
use parking_lot::RwLock;

impl FunctionCompiler {
    pub(crate) fn compile_table_literal(&mut self, expr: &Expr, ctx: &mut CompileContext, table_name: Option<String>) -> u8 {
        if let ExprKind::TableLiteral { columns, rows } = &expr.kind {
            let vm_cols = columns.iter().map(|c| crate::vm::object::VMColumn {
                name: ctx.interner.lookup(c.name).to_string(),
                ty: c.ty.clone(), is_auto: c.is_auto(), is_pk: c.is_pk(), is_unique: c.is_unique(),
            }).collect();
            let t_name = table_name.unwrap_or_default();
            let skeleton = Value::from_table(Arc::new(RwLock::new(
                crate::vm::object::TableObj { table_name: t_name, columns: vm_cols, rows: Vec::new(), sql_binding: None, sql_where: None, pending_op: None }
            )));
            let ncol = columns.iter().filter(|c| !c.is_auto()).count();
            let ci = ctx.add_constant(skeleton);
            let base = self.next_local as u8;

            if rows.len() * ncol > 200 {
                self.emit(OpCode::TableBegin { dst: base, skeleton_idx: ci }, &expr.span);
                let row_base = base + 1;
                for row in rows {
                    let mut col_idx = 0;
                    self.next_local = row_base as usize;
                    for val in row {
                        let dst = row_base + col_idx as u8;
                        let src = self.compile_expr(val, ctx);
                        if src != dst { self.emit(OpCode::Move { dst, src }, &expr.span); }
                        self.next_local = (dst + 1) as usize; self.sync_max_locals();
                        col_idx += 1;
                    }
                    self.emit(OpCode::TableInitRow { tbl_dst: base, base: row_base, col_count: ncol as u8 }, &expr.span);
                }
                self.next_local = (base + 1) as usize;
                self.sync_max_locals();
                return base;
            } else {
                let mut current_idx = 0;
                for row in rows {
                    for val in row {
                        let dst = base + current_idx as u8;
                        let src = self.compile_expr(val, ctx);
                        if src != dst { self.emit(OpCode::Move { dst, src }, &expr.span); }
                        self.next_local = (dst + 1) as usize; self.sync_max_locals();
                        current_idx += 1;
                    }
                }
                self.emit(OpCode::TableInit { dst: base, skeleton_idx: ci, base, row_count: rows.len() as u32, col_count: ncol as u32 }, &expr.span);
                self.next_local = (base + 1) as usize;
                self.sync_max_locals();
                return base;
            }
        }
        0
    }

    pub(crate) fn compile_database_literal(&mut self, expr: &Expr, ctx: &mut CompileContext) -> u8 {
        if let ExprKind::DatabaseLiteral(fields) = &expr.kind {
            let mut engine_reg = 0; let mut path_reg = 0;
            let mut found_engine = false; let mut found_path = false;
            for (name, expr) in fields {
                let name_str = ctx.interner.lookup(*name);
                if name_str == "engine" { engine_reg = self.compile_expr(expr, ctx); found_engine = true; }
                else if name_str == "path" { path_reg = self.compile_expr(expr, ctx); found_path = true; }
            }
            if !found_engine || !found_path { return self.push_reg(); }
            let base = self.next_local as u8;
            let mut table_count = 0;
            for (name, expr) in fields {
                let name_str = ctx.interner.lookup(*name);
                if name_str != "engine" && name_str != "path" {
                    let name_idx = ctx.add_constant(Value::from_string(Arc::new(StringObj::new(name_str.to_string().into_bytes()))));
                    let nr = base + (table_count as u8 * 2);
                    self.emit(OpCode::LoadConst { dst: nr, idx: name_idx }, &expr.span);
                    self.next_local = (nr + 1) as usize; self.sync_max_locals();
                    let tr = nr + 1;
                    let src = self.compile_expr(expr, ctx);
                    if src != tr { self.emit(OpCode::Move { dst: tr, src }, &expr.span); }
                    self.next_local = (tr + 1) as usize; self.sync_max_locals();
                    table_count += 1;
                }
            }
            let dst = engine_reg;
            self.emit(OpCode::DatabaseInit { dst, engine_src: engine_reg, path_src: path_reg, tables_base_reg: base, table_count }, &expr.span);
            self.next_local = (dst + 1) as usize; self.sync_max_locals();
            return dst;
        }
        0
    }
}
