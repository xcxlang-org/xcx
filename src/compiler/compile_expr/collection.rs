use crate::frontend::ast::{Expr, ExprKind};
use crate::compiler::compiler::{FunctionCompiler, CompileContext};
use crate::vm::opcode::OpCode;
use crate::vm::value::Value;

pub fn compile(fc: &mut FunctionCompiler, expr: &Expr, ctx: &mut CompileContext) -> u8 {
    match &expr.kind {
        ExprKind::ArrayLiteral { elements } => {
            let base = fc.next_local as u8;
            for (i, e) in elements.iter().enumerate() {
                let dst = base + i as u8;
                fc.next_local = dst as usize;
                let src = fc.compile_expr(e, ctx);
                if src != dst { fc.emit(OpCode::Move { dst, src }, &expr.span); }
                fc.next_local = (dst + 1) as usize;
                fc.sync_max_locals();
            }
            let dst = base;
            fc.emit(OpCode::ArrayInit { dst, base, count: elements.len() as u32 }, &expr.span);
            fc.next_local = (base + 1) as usize;
            fc.sync_max_locals();
            dst
        }
        ExprKind::SetLiteral { elements, range, .. } => {
            if let Some(r) = range {
                let start = fc.compile_expr(&r.start, ctx);
                let end   = fc.compile_expr(&r.end, ctx);
                let (step, has_step_reg) = if let Some(s) = &r.step {
                    let step_reg = fc.compile_expr(s, ctx);
                    let h_idx = ctx.add_constant(Value::from_bool(true));
                    let h_reg = fc.push_reg();
                    fc.emit(OpCode::LoadConst { dst: h_reg, idx: h_idx }, &expr.span);
                    (step_reg, h_reg)
                } else {
                    let dummy_idx = ctx.add_constant(Value::from_bool(false));
                    let dummy = fc.push_reg();
                    fc.emit(OpCode::LoadConst { dst: dummy, idx: dummy_idx }, &expr.span);
                    let f_idx = ctx.add_constant(Value::from_bool(false));
                    let f_reg = fc.push_reg();
                    fc.emit(OpCode::LoadConst { dst: f_reg, idx: f_idx }, &expr.span);
                    (dummy, f_reg)
                };
                let dst = start;
                fc.emit(OpCode::SetRange { dst, start, end, step, has_step: has_step_reg }, &expr.span);
                fc.next_local = (dst + 1) as usize; 
                fc.sync_max_locals();
                dst
            } else {
                let base = fc.next_local as u8;
                for (i, e) in elements.iter().enumerate() {
                    let dst = base + i as u8;
                    fc.next_local = dst as usize;
                    let src = fc.compile_expr(e, ctx);
                    if src != dst { fc.emit(OpCode::Move { dst, src }, &expr.span); }
                    fc.next_local = (dst + 1) as usize;
                    fc.sync_max_locals();
                }
                let dst = base;
                fc.emit(OpCode::SetInit { dst, base, count: elements.len() as u32 }, &expr.span);
                fc.next_local = (base + 1) as usize;
                fc.sync_max_locals();
                dst
            }
        }
        ExprKind::MapLiteral { elements, .. } => {
            let base = fc.next_local as u8;
            for (i, (k, v)) in elements.iter().enumerate() {
                let k_dst = base + (i as u8 * 2);
                fc.next_local = k_dst as usize;
                let k_src = fc.compile_expr(k, ctx);
                if k_src != k_dst { fc.emit(OpCode::Move { dst: k_dst, src: k_src }, &expr.span); }
                fc.next_local = (k_dst + 1) as usize;
                fc.sync_max_locals();

                let v_dst = k_dst + 1;
                fc.next_local = v_dst as usize;
                let v_src = fc.compile_expr(v, ctx);
                if v_src != v_dst { fc.emit(OpCode::Move { dst: v_dst, src: v_src }, &expr.span); }
                fc.next_local = (v_dst + 1) as usize;
                fc.sync_max_locals();
            }
            let dst = base;
            fc.emit(OpCode::MapInit { dst, base, count: elements.len() as u32 }, &expr.span);
            fc.next_local = (base + 1) as usize;
            fc.sync_max_locals();
            dst
        }
        ExprKind::ArrayOrSetLiteral { elements: exprs } => {
            let base = fc.next_local as u8;
            for (i, e) in exprs.iter().enumerate() {
                let dst = base + i as u8;
                fc.next_local = dst as usize;
                let src = fc.compile_expr(e, ctx);
                if src != dst { fc.emit(OpCode::Move { dst, src }, &expr.span); }
                fc.next_local = (dst + 1) as usize;
                fc.sync_max_locals();
            }
            let dst = base;
            fc.emit(OpCode::ArrayInit { dst, base, count: exprs.len() as u32 }, &expr.span);
            fc.next_local = (base + 1) as usize;
            fc.sync_max_locals();
            dst
        }
        ExprKind::TableLiteral { .. } => fc.compile_table_literal(expr, ctx, None),
        ExprKind::DatabaseLiteral(..) => fc.compile_database_literal(expr, ctx),
        ExprKind::RandomChoice { set } => {
            let src = fc.compile_expr(set, ctx);
            let dst = src;
            fc.emit(OpCode::RandomChoice { dst, src }, &expr.span);
            dst
        }
        ExprKind::RandomInt { min, max, step } => {
            let min_reg = fc.compile_expr(min, ctx);
            let max_reg = fc.compile_expr(max, ctx);
            let (step_reg, has_step_reg) = if let Some(s) = step {
                let s_reg = fc.compile_expr(s, ctx);
                let h_idx = ctx.add_constant(Value::from_bool(true));
                let h_reg = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: h_reg, idx: h_idx }, &expr.span);
                (s_reg, h_reg)
            } else {
                let dummy_idx = ctx.add_constant(Value::from_bool(false));
                let dummy = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: dummy, idx: dummy_idx }, &expr.span);
                let f_idx = ctx.add_constant(Value::from_bool(false));
                let f_reg = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: f_reg, idx: f_idx }, &expr.span);
                (dummy, f_reg)
            };
            let dst = min_reg;
            fc.emit(OpCode::RandomInt { dst, min: min_reg, max: max_reg, step: step_reg, has_step: has_step_reg }, &expr.span);
            fc.next_local = (dst + 1) as usize;
            fc.sync_max_locals();
            dst
        }
        ExprKind::RandomFloat { min, max, step } => {
            let min_reg = fc.compile_expr(min, ctx);
            let max_reg = fc.compile_expr(max, ctx);
            let (step_reg, has_step_reg) = if let Some(s) = step {
                let s_reg = fc.compile_expr(s, ctx);
                let h_idx = ctx.add_constant(Value::from_bool(true));
                let h_reg = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: h_reg, idx: h_idx }, &expr.span);
                (s_reg, h_reg)
            } else {
                let dummy_idx = ctx.add_constant(Value::from_bool(false));
                let dummy = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: dummy, idx: dummy_idx }, &expr.span);
                let f_idx = ctx.add_constant(Value::from_bool(false));
                let f_reg = fc.push_reg();
                fc.emit(OpCode::LoadConst { dst: f_reg, idx: f_idx }, &expr.span);
                (dummy, f_reg)
            };
            let dst = min_reg;
            fc.emit(OpCode::RandomFloat { dst, min: min_reg, max: max_reg, step: step_reg, has_step: has_step_reg }, &expr.span);
            fc.next_local = (dst + 1) as usize;
            fc.sync_max_locals();
            dst
        }
        ExprKind::DateLiteral { date_string, format } => {
            let date_str = ctx.interner.lookup(*date_string).to_string();
            let date = if let Some(fmt_id) = format {
                let fmt_str = ctx.interner.lookup(*fmt_id).to_string();
                let chrono_fmt = fmt_str
                    .replace("YYYY", "%Y").replace("MM", "%m").replace("DD", "%d")
                    .replace("M", "%-m").replace("D", "%-d");
                chrono::NaiveDate::parse_from_str(&date_str, &chrono_fmt)
                    .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            } else {
                chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            };
            let dt = date.and_hms_opt(0, 0, 0).unwrap();
            let i = ctx.add_constant(Value::from_date(dt.and_utc().timestamp_millis()));
            let dst = fc.push_reg();
            fc.emit(OpCode::LoadConst { dst, idx: i }, &expr.span);
            dst
        }
        _ => 0,
    }
}
