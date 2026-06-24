use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::OpResult;
use crate::vm::Executor;
use crate::vm::value::Value;
use crate::vm::opcode::MethodKind;
use crate::vm::object::{DatabaseObj, TableObj, VMColumn};

impl Executor {
    pub fn handle_database_ddl(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, name: String, ip: usize, locals: &mut [Value]) -> OpResult {
        if let Some(t_val) = db_rc.tables.read().get(&name) {
            let t_rc = t_val.as_table();
            let res = Value::from_table(t_rc);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        } else {
            let conn_clone = db_rc.conn.clone();
            let mut cols = Vec::new();
            {
                let conn = conn_clone.lock();
                let sql = format!("PRAGMA table_info([{}])", name);
                if let Ok(mut stmt) = conn.prepare(&sql) {
                    if let Ok(rows) = stmt.query_map([], |row| {
                        let cname: String = row.get(1)?;
                        let ctype: String = row.get(2)?;
                        let is_pk: i32 = row.get(5)?;
                        let ty = if ctype.to_uppercase().contains("INT") { crate::frontend::ast::Type::Int }
                                else if ctype.to_uppercase().contains("FLOAT") || ctype.to_uppercase().contains("REAL") { crate::frontend::ast::Type::Float }
                                else if ctype.to_uppercase().contains("BOOL") { crate::frontend::ast::Type::Bool }
                                else { crate::frontend::ast::Type::String };
                        Ok(VMColumn { name: cname, ty, is_pk: is_pk != 0, is_auto: is_pk != 0 && ctype.to_uppercase().contains("INTEGER"), is_unique: false })
                    }) {
                        for r in rows { if let Ok(info) = r { cols.push(info); } }
                    }
                }
            }
            if !cols.is_empty() {
                let t_rc = Arc::new(RwLock::new(TableObj {
                    table_name: name.clone(),
                    columns: cols,
                    rows: Vec::new(),
                    sql_binding: Some(crate::vm::object::SqlBinding { table_name: name.clone(), db_conn: conn_clone }),
                    sql_where: None,
                    pending_op: None,
                }));
                db_rc.tables.write().insert(name, Value::from_table(t_rc.clone()));
                let res = Value::from_table(t_rc);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            } else {
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                eprintln!("R401: Table not found in database: {}{}", name, self.current_span_info(ip));
                return OpResult::Halt;
            }
        }
        OpResult::Continue
    }

    pub fn handle_database_member_access(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, name: &[u8], ip: usize, locals: &mut [Value], _vm_arc: &Arc<crate::vm::core::vm::VM>, _glbs: &mut Option<parking_lot::RwLockWriteGuard<Vec<Value>>>) -> OpResult {
        let name_str = String::from_utf8_lossy(name).to_string();
        self.handle_database_ddl(dst, db_rc, name_str, ip, locals)
    }

    pub fn handle_database_sync(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, args: &[Value], ip: usize, locals: &mut [Value]) -> OpResult {
        if args.is_empty() { return OpResult::Continue; }
        let table_val = &args[0];
        if !table_val.is_table() {
            self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            eprintln!("R405: Sync expects a Table object as first argument{}", self.current_span_info(ip));
            return OpResult::Halt;
        }
        
        let table_rc = table_val.as_table();
        let mut table = table_rc.write();
        
        let mut sql = format!("CREATE TABLE IF NOT EXISTS [{}] (", table.table_name);
        for (i, col) in table.columns.iter().enumerate() {
            if i > 0 { sql.push_str(", "); }
            let ty_str = match col.ty {
                crate::frontend::ast::Type::Int => "INTEGER",
                crate::frontend::ast::Type::Float => "REAL",
                crate::frontend::ast::Type::Bool => "INTEGER",
                _ => "TEXT",
            };
            sql.push_str(&format!("[{}] {}", col.name, ty_str));
            if col.is_pk {
                sql.push_str(" PRIMARY KEY");
                if col.is_auto && col.ty == crate::frontend::ast::Type::Int {
                    sql.push_str(" AUTOINCREMENT");
                }
            }
            if col.is_unique {
                sql.push_str(" UNIQUE");
            }
        }
        sql.push(')');
        
        let conn = db_rc.conn.lock();
        match conn.execute(&sql, []) {
            Ok(_) => {
                table.sql_binding = Some(crate::vm::object::SqlBinding {
                    table_name: table.table_name.clone(),
                    db_conn: db_rc.conn.clone(),
                });
                let tbl_name = table.table_name.clone();
                drop(table);
                db_rc.tables.write().insert(tbl_name, table_val.clone());
                
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(true);
                OpResult::Continue
            }
            Err(e) => {
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                eprintln!("R406: Failed to sync table {}: {}{}", table.table_name, e, self.current_span_info(ip));
                OpResult::Halt
            }
        }
    }

    pub fn handle_database_maintenance_ddl(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, kind: MethodKind, args: &[Value], ip: usize, locals: &mut [Value]) -> OpResult {
        if args.is_empty() { return OpResult::Continue; }
        let table_name = if args[0].is_string() {
            String::from_utf8_lossy(&args[0].as_string()).into_owned()
        } else if args[0].is_table() {
            args[0].as_table().read().table_name.clone()
        } else {
            self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            eprintln!("R407: Method {:?} expects a table name or table object{}", kind, self.current_span_info(ip));
            return OpResult::Halt;
        };

        let conn = db_rc.conn.lock();
        match kind {
            MethodKind::Drop => {
                let sql = format!("DROP TABLE IF EXISTS [{}]", table_name);
                match conn.execute(&sql, []) {
                    Ok(_) => {
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = Value::from_bool(true);
                        OpResult::Continue
                    }
                    Err(e) => {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!("R408: Failed to drop table {}: {}{}", table_name, e, self.current_span_info(ip));
                        OpResult::Halt
                    }
                }
            }
            MethodKind::Has => {
                let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";
                let mut stmt = conn.prepare(sql).unwrap();
                let exists = stmt.exists([&table_name]).unwrap_or(false);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(exists);
                OpResult::Continue
            }
            _ => OpResult::Continue
        }
    }
}
