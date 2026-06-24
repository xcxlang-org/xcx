use std::sync::Arc;
use crate::vm::core::vm::OpResult;
use crate::vm::Executor;
use crate::vm::value::Value;
use crate::vm::object::{DatabaseObj, JsonObj};

impl Executor {
    pub fn handle_database_write(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, kind: crate::vm::opcode::MethodKind, args: &[Value], ip: usize, locals: &mut [Value]) -> OpResult {
        if args.is_empty() { return OpResult::Continue; }
        
        let mut sqls = Vec::new();
        let mut params_list = Vec::new();
        
        match kind {
            crate::vm::opcode::MethodKind::Insert => {
                if !args[0].is_table() {
                    self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    eprintln!("R409: Insert expects a table as first argument{}", self.current_span_info(ip));
                    return OpResult::Halt;
                }
                let table_rc = args[0].as_table();
                let table = table_rc.read();
                
                let mut col_names = Vec::new();
                let mut placeholders = Vec::new();
                let mut params = Vec::new();
                
                let mut col_idx = 0;
                let mut arg_idx = 1;
                while arg_idx < args.len() && col_idx < table.columns.len() {
                    let col = &table.columns[col_idx];
                    if col.is_auto {
                        col_idx += 1;
                        continue;
                    }
                    col_names.push(format!("[{}]", col.name));
                    placeholders.push("?".to_string());
                    params.push(Box::new(args[arg_idx].to_sql_value()) as Box<dyn rusqlite::ToSql>);
                    col_idx += 1;
                    arg_idx += 1;
                }
                

                
                let sql = format!("INSERT INTO [{}] ({}) VALUES ({})", table.table_name, col_names.join(", "), placeholders.join(", "));
                sqls.push(sql);
                params_list.push(params);
            }
            crate::vm::opcode::MethodKind::Truncate => {
                let table_name = if args[0].is_table() {
                    args[0].as_table().read().table_name.clone()
                } else {
                    args[0].to_string()
                };
                sqls.push(format!("DELETE FROM [{}]", table_name));
                params_list.push(Vec::new());
            }
            crate::vm::opcode::MethodKind::Push => {
                if !args[0].is_table() { return OpResult::Halt; }
                let table_rc = args[0].as_table();
                let table = table_rc.read();
                
                let mut col_names = Vec::new();
                let mut placeholders = Vec::new();
                let mut col_indices = Vec::new();
                for (i, col) in table.columns.iter().enumerate() {
                    if col.is_auto { continue; }
                    col_names.push(format!("[{}]", col.name));
                    placeholders.push("?".to_string());
                    col_indices.push(i);
                }
                
                let sql = format!("INSERT INTO [{}] ({}) VALUES ({})", table.table_name, col_names.join(", "), placeholders.join(", "));
                for row in &table.rows {
                    let mut params = Vec::new();
                    for &idx in &col_indices {
                        params.push(Box::new(row[idx].to_sql_value()) as Box<dyn rusqlite::ToSql>);
                    }
                    sqls.push(sql.clone());
                    params_list.push(params);
                }
            }
            crate::vm::opcode::MethodKind::Save => {
                // Simplified UPSERT for SQLite
                if !args[0].is_table() { return OpResult::Halt; }
                let table_rc = args[0].as_table();
                let table = table_rc.read();
                
                let pk_col = table.columns.iter().find(|c| c.is_pk);
                if pk_col.is_none() {
                    self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    eprintln!("R410: Save (upsert) requires a primary key in table schema{}", self.current_span_info(ip));
                    return OpResult::Halt;
                }
                let pk_name = pk_col.unwrap().name.clone();
                
                let mut col_names = Vec::new();
                let mut placeholders = Vec::new();
                let mut col_indices = Vec::new();
                let mut updates = Vec::new();
                for (i, col) in table.columns.iter().enumerate() {
                    if col.is_auto { continue; }
                    col_names.push(format!("[{}]", col.name));
                    placeholders.push("?".to_string());
                    col_indices.push(i);
                    if !col.is_pk {
                        updates.push(format!("[{}] = excluded.[{}]", col.name, col.name));
                    }
                }
                
                let sql = format!("INSERT INTO [{}] ({}) VALUES ({}) ON CONFLICT([{}]) DO UPDATE SET {}", 
                    table.table_name, col_names.join(", "), placeholders.join(", "), pk_name, updates.join(", "));
                
                for row in &table.rows {
                    let mut params = Vec::new();
                    for &idx in &col_indices {
                        params.push(Box::new(row[idx].to_sql_value()) as Box<dyn rusqlite::ToSql>);
                    }
                    sqls.push(sql.clone());
                    params_list.push(params);
                }
            }
            _ => {
                // Direct SQL (Execute/Exec)
                sqls.push(args[0].to_string());
                let mut params = Vec::new();
                if args.len() > 1 && args[1].is_array() {
                    let arr_rc = args[1].as_array();
                    for v in &arr_rc.read().elements {
                        params.push(Box::new(v.to_sql_value()) as Box<dyn rusqlite::ToSql>);
                    }
                }
                params_list.push(params);
            }
        }

        let mut affected = 0;
        let mut last_id = 0;
        let conn = db_rc.conn.lock();
        
        for (sql, params) in sqls.into_iter().zip(params_list.into_iter()) {
            match conn.execute(&sql, rusqlite::params_from_iter(params)) {
                Ok(n) => {
                    affected += n;
                    last_id = conn.last_insert_rowid();
                }
                Err(e) => {
                    eprintln!("R403: DB write error: {} in SQL: {}{}", e, sql, self.current_span_info(ip));
                    let _ = conn.execute("ROLLBACK", []);
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = Value::from_bool(false);
                    return OpResult::Continue;
                }
            }
        }

        let mut obj = Vec::new();
        obj.push((std::sync::Arc::new("affected".to_string()), crate::vm::object::JsonVal::Int(affected as i64)));
        obj.push((std::sync::Arc::new("insertId".to_string()), crate::vm::object::JsonVal::Int(last_id as i64)));
        let res = Value::from_json(Arc::new(JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(obj))))));
        
        
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = res;
        OpResult::Continue
    }
}
