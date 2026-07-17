use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::OpResult;
use crate::vm::Executor;
use crate::vm::value::Value;
use crate::vm::object::{DatabaseObj, TableObj, StringObj};

impl Executor {
    pub fn handle_database_read(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, args: &[Value], ip: usize, locals: &mut [Value]) -> OpResult {
        if args.is_empty() { return OpResult::Continue; }
        
        let mut sql: String;
        let mut params_vals: Vec<Value> = Vec::new();
        let mut target_table: Option<Arc<RwLock<TableObj>>> = None;
        let mut is_raw = false;

        match args.len() {
            1 => {
                if args[0].is_table() {
                    let table_rc = args[0].as_table();
                    let table = table_rc.read();
                    sql = format!("SELECT * FROM [{}]", table.table_name);
                    if let Some(w) = &table.sql_where {
                        sql.push_str(" WHERE ");
                        sql.push_str(w);
                    }
                    target_table = Some(table_rc.clone());
                } else {
                    sql = args[0].to_string();
                    is_raw = true;
                }
            }
            2 | 3 => {
                if args[0].is_table() {
                    target_table = Some(args[0].as_table().clone());
                    sql = args[1].to_string();
                    if args.len() == 3 && args[2].is_array() {
                        let arr_rc = args[2].as_array();
                        params_vals = arr_rc.read().elements.clone();
                    }
                } else {
                    sql = args[0].to_string();
                    if args[1].is_array() {
                        let arr_rc = args[1].as_array();
                        params_vals = arr_rc.read().elements.clone();
                    }
                    is_raw = true;
                }
            }
            _ => { return OpResult::Halt; }
        }

        let conn = db_rc.conn.lock();
        match conn.prepare(&sql) {
            Ok(mut stmt) => {
                let col_count = stmt.column_count();
                let mut xrows = Vec::new();
                let mut jrows = Vec::new();

                let col_names: Vec<String> = (0..col_count).map(|i| stmt.column_name(i).unwrap_or("unknown").to_string()).collect();
                let params: Vec<Box<dyn rusqlite::ToSql>> = params_vals.iter().map(|v| Box::new(v.to_sql_value()) as Box<dyn rusqlite::ToSql>).collect();
                let rusql_params = rusqlite::params_from_iter(params);

                let mut rows_iter = match stmt.query(rusql_params) {
                    Ok(i) => i,
                    Err(e) => {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!("R403: DB query error: {}{}", e, self.current_span_info(ip));
                        return OpResult::Halt;
                    }
                };

                while let Ok(Some(row)) = rows_iter.next() {
                    if is_raw {
                        let mut jrow = Vec::new();
                        for i in 0..col_count {
                            let name = Arc::new(col_names[i].clone());
                            if let Ok(v) = row.get::<_, i64>(i) { jrow.push((name, crate::vm::object::JsonVal::Int(v))); }
                            else if let Ok(v) = row.get::<_, f64>(i) { jrow.push((name, crate::vm::object::JsonVal::Float(v))); }
                            else if let Ok(v) = row.get::<_, String>(i) { jrow.push((name, crate::vm::object::JsonVal::String(Arc::new(v)))); }
                            else if let Ok(v) = row.get::<_, bool>(i) { jrow.push((name, crate::vm::object::JsonVal::Bool(v))); }
                            else { jrow.push((name, crate::vm::object::JsonVal::Null)); }
                        }
                        jrows.push(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(jrow))));
                    } else {
                        let mut xrow = Vec::new();
                        for i in 0..col_count {
                            if let Ok(v) = row.get::<_, i64>(i) { xrow.push(Value::from_i64(v)); }
                            else if let Ok(v) = row.get::<_, f64>(i) { xrow.push(Value::from_f64(v)); }
                            else if let Ok(v) = row.get::<_, String>(i) { xrow.push(Value::from_string(Arc::new(StringObj::new(v.into_bytes())))); }
                            else { xrow.push(Value::from_bool(false)); }
                        }
                        xrows.push(xrow);
                    }
                }

                if is_raw {
                    let res = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(crate::vm::object::JsonVal::Array(Arc::new(parking_lot::RwLock::new(jrows))))));
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = res;
                } else if let Some(tt) = target_table {
                    let mut t = tt.read().clone();
                    t.rows = xrows;
                    let res = Value::from_table(Arc::new(RwLock::new(t)));
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = res;
                }
            }
            Err(e) => {
                eprintln!("R403: DB query prepare error: {}{}", e, self.current_span_info(ip));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(false);
                return OpResult::Continue;
            }
        }
        OpResult::Continue
    }
}
