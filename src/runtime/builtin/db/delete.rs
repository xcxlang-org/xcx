use std::sync::Arc;
use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use crate::vm::value::Value;
use crate::vm::object::DatabaseObj;

impl Executor {
    pub fn handle_database_delete(&mut self, dst: u8, db_rc: Arc<DatabaseObj>, args: &[Value], ip: usize, locals: &mut [Value]) -> OpResult {
        if args.is_empty() { return OpResult::Halt; }
        
        if args[0].is_table() {
            let table_rc = args[0].as_table();
            {
                let mut table = table_rc.write();
                table.pending_op = Some(crate::vm::opcode::MethodKind::Remove);
                table.sql_binding = Some(crate::vm::object::SqlBinding {
                    table_name: table.table_name.clone(),
                    db_conn: db_rc.conn.clone(),
                });
            }
            // Return the table so .where() can be called
            let res = args[0];
            unsafe { res.inc_ref(); }
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            return OpResult::Continue;
        }

        let sql = args[0].to_string();
        let conn = db_rc.conn.lock();
        match conn.execute(&sql, []) {
            Ok(affected) => {
                let mut obj = Vec::new();
                obj.push((std::sync::Arc::new("affected".to_string()), crate::vm::object::JsonVal::Int(affected as i64)));
                obj.push((std::sync::Arc::new("insertId".to_string()), crate::vm::object::JsonVal::Int(0)));
                let res = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(obj))))));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            Err(e) => {
                eprintln!("R404: DB delete error: {}{}", e, self.current_span_info(ip));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(false);
            }
        }
        OpResult::Continue
    }
}
