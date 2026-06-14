use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::value::Value;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::object::{TableObj, RowObj, JoinPred};
use crate::vm::opcode::MethodKind;
use crate::vm::core::executor::Executor;
use crate::vm::utils;

impl Executor {
    pub fn handle_table_select(&mut self, dst: u8, t_rc: Arc<RwLock<TableObj>>, kind: MethodKind, args: &[Value], _ip: usize, locals: &mut [Value], vm_arc: &Arc<VM>) -> OpResult {
        let t = t_rc.read();
        match kind {
            MethodKind::Where => {
                let (filter_func, captures) = if args[0].is_func() { (args[0].as_function_idx(), vec![]) }
                else if args[0].is_fiber() {
                    let f_arc = args[0].as_fiber();
                    let f = f_arc.read();
                    (f.func_id as u32, f.locals.clone())
                }
                else { return OpResult::Halt; };

                let sql_where = if t.sql_binding.is_some() { utils::table::translate_filter_to_sql(self, filter_func as usize, &t.columns, &captures) } else { None };

                if let Some(MethodKind::Remove) = t.pending_op {
                    if let Some(binding) = &t.sql_binding {
                        let conn = binding.db_conn.lock();
                        let mut sql = format!("DELETE FROM [{}]", binding.table_name);
                        if let Some(w) = &sql_where { sql.push_str(" WHERE "); sql.push_str(w); }
                        let affected = conn.execute(&sql, []).unwrap_or(0);
                        let mut obj = Vec::new();
                        obj.push((std::sync::Arc::new("affected".to_string()), crate::vm::object::JsonVal::Int(affected as i64)));
                        obj.push((std::sync::Arc::new("insertId".to_string()), crate::vm::object::JsonVal::Int(0)));
                        let res = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(obj))))));
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                        return OpResult::Continue;
                    }
                }

                let row_count = t.rows.len();
                drop(t);
                let mut filtered = Vec::new();
                for i in 0..row_count {
                    let row_val = Value::from_row(Arc::new(RowObj { table: t_rc.clone(), row_idx: i as u32 }));
                    let mut run_args = vec![row_val];
                    for a in &args[1..] { unsafe { a.inc_ref(); } run_args.push(*a); }
                    if let Some(res) = self.run_frame(self.ctx.functions[filter_func as usize].clone(), &run_args, vm_arc, filter_func as usize) {
                        let res: Value = res;
                        if res.is_bool() && res.as_bool() {
                            let mut row_copy = Vec::new();
                            let guard = t_rc.read();
                            for v in &(*guard).rows[i] { unsafe { (v as &Value).inc_ref(); } row_copy.push(*v); }
                            filtered.push(row_copy);
                        }
                        unsafe { res.dec_ref(); }
                    }
                    unsafe { row_val.dec_ref(); }
                    for a in run_args.into_iter().skip(1) { unsafe { a.dec_ref(); } }
                }
                
                let t_read = t_rc.read();
                let res = Value::from_table(Arc::new(RwLock::new(TableObj {
                    table_name: t_read.table_name.clone(), columns: t_read.columns.clone(), rows: filtered, sql_binding: t_read.sql_binding.clone(), sql_where, pending_op: None,
                })));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Join => {
                if args.is_empty() { return OpResult::Halt; }
                let right_rc = args[0].as_table();
                let pred = if args.len() >= 3 {
                    let lk = String::from_utf8_lossy(&args[1].as_string()).into_owned();
                    let rk = String::from_utf8_lossy(&args[2].as_string()).into_owned();
                    JoinPred::Keys(lk, rk)
                } else if args.len() == 2 {
                    if args[1].is_func() { JoinPred::Lambda(args[1].as_function_idx() as usize) }
                    else {
                        let f_arc = args[1].as_fiber();
                        let f = f_arc.read();
                        JoinPred::Closure(f.func_id as usize, f.locals.clone())
                    }
                } else { return OpResult::Halt; };
                let left_data = t.clone();
                let right_data = right_rc.read().clone();
                drop(t);
                let result = utils::table::join_tables(&left_data, &right_data, &pred, "b", self, vm_arc);
                let res = Value::from_table(Arc::new(RwLock::new(result)));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Show => {
                // println!("{}", t.to_formatted_grid());
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Count | MethodKind::Len | MethodKind::Size => {
                let res = Value::from_i64(t.rows.len() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Find => {
                if args.is_empty() { 
                    // eprintln!("Table.find: missing predicate argument{}", self.current_span_info(ip));
                    return OpResult::Halt; 
                }
                let filter_func = if args[0].is_func() {
                    args[0].as_function_idx()
                } else if args[0].is_fiber() {
                    args[0].as_fiber().read().func_id as u32
                } else {
                    // eprintln!("Table.find: first argument must be a function or fiber{}", self.current_span_info(ip));
                    return OpResult::Halt;
                };
                let row_count = t.rows.len();
                drop(t);
                let mut found_idx: i64 = -1;
                for i in 0..row_count {
                    let row_ref = Arc::new(RowObj { table: t_rc.clone(), row_idx: i as u32 });
                    let row_val = Value::from_row(row_ref);
                    let mut run_args = vec![row_val];
                    for a in &args[1..] { unsafe { a.inc_ref(); } run_args.push(*a); }
                    if let Some(res) = self.run_frame(self.ctx.functions[filter_func as usize].clone(), &run_args, vm_arc, filter_func as usize) {
                        if res.is_bool() && res.as_bool() {
                            found_idx = i as i64;
                            unsafe { res.dec_ref(); }
                            unsafe { row_val.dec_ref(); }
                            for a in run_args.into_iter().skip(1) { unsafe { a.dec_ref(); } }
                            break;
                        }
                        unsafe { res.dec_ref(); }
                    }
                    unsafe { row_val.dec_ref(); }
                    for a in run_args.into_iter().skip(1) { unsafe { a.dec_ref(); } }
                }
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_i64(found_idx);
            }
            _ => { return OpResult::Halt; }
        }
        OpResult::Continue
    }

    pub fn handle_table_query<'a>(
        &mut self,
        dst: u8,
        t_rc: Arc<RwLock<TableObj>>,
        kind: MethodKind,
        args: &[Value],
        _ip: usize,
        locals: &mut [Value],
    ) -> OpResult {
        let t = t_rc.read();
        let sql = if kind == MethodKind::Query && !args.is_empty() {
            args[0].to_string()
        } else {
            let table_name = if let Some(binding) = &t.sql_binding {
                binding.table_name.clone()
            } else { "unknown".to_string() };
            let mut s = format!("SELECT * FROM [{}]", table_name);
            if let Some(w) = &t.sql_where {
                s.push_str(" WHERE ");
                s.push_str(w);
            }
            s
        };
        
        let binding_opt = t.sql_binding.clone();
        let cols = t.columns.clone();

        if let Some(binding) = binding_opt {
            let mut new_rows = Vec::new();
            let mut ok = false;
            {
                let conn = binding.db_conn.lock();
                if let Ok(mut stmt) = conn.prepare(&sql) {
                    if let Ok(rows_iter) = stmt.query_map(rusqlite::params![], |row: &rusqlite::Row<'_>| -> rusqlite::Result<Vec<Value>> {
                        let mut row_vals = Vec::with_capacity(cols.len());
                        for (i, col) in cols.iter().enumerate() {
                            let v = match col.ty {
                                crate::frontend::ast::Type::Int => Value::from_i64(row.get::<_, i64>(i).unwrap_or(0)),
                                crate::frontend::ast::Type::Float => Value::from_f64(row.get::<_, f64>(i).unwrap_or(0.0)),
                                crate::frontend::ast::Type::Bool => Value::from_bool(row.get::<_, i32>(i).unwrap_or(0) != 0),
                                crate::frontend::ast::Type::String => Value::from_string(Arc::new(crate::vm::object::StringObj::new(row.get::<_, String>(i).unwrap_or_default().into_bytes()))),
                                _ => Value::from_bool(false),
                            };
                            row_vals.push(v);
                        }
                        Ok(row_vals)
                    }) {
                        for r in rows_iter { if let Ok(row) = r { new_rows.push(row); } }
                        ok = true;
                    }
                }
            }

            if ok {
                drop(t);
                let res = Value::from_table(Arc::new(RwLock::new(TableObj {
                    table_name: String::new(),
                    columns: cols,
                    rows: new_rows,
                    sql_binding: Some(binding),
                    sql_where: None,
                    pending_op: None,
                })));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            } else {
                drop(t);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(false);
            }
        } else {
            let rows_copy = t.rows.iter().map(|r: &Vec<Value>| {
                r.iter().map(|v: &Value| { unsafe { (v as &Value).inc_ref(); } *v }).collect::<Vec<Value>>()
            }).collect::<Vec<Vec<Value>>>();
            drop(t);
            let res = Value::from_table(Arc::new(RwLock::new(TableObj {
                table_name: String::new(),
                columns: cols,
                rows: rows_copy,
                sql_binding: None,
                sql_where: None,
                pending_op: None,
            })));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpResult::Continue
    }

    pub fn handle_table_to_json<'a>(
        &mut self,
        dst: u8,
        t_rc: Arc<RwLock<TableObj>>,
        locals: &mut [Value],
    ) -> OpResult {
        let json = t_rc.read().to_json();
        let res = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(json)));
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = res;
        OpResult::Continue
    }
}
