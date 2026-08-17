use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;
use crate::vm::opcode::MethodKind;
use crate::vm::core::executor::Executor;

impl Executor {
    pub fn handle_table_insert<'a>(&mut self, dst: u8, t_rc: Arc<RwLock<crate::vm::object::TableObj>>, kind: MethodKind, args: &[Value], names: Option<&[String]>, locals: &mut [Value]) -> OpResult {
        let mut t_mut = t_rc.write();
        let key = Arc::as_ptr(&t_rc) as usize;
        if let Some(cache_vec) = self.row_cache.remove(&key) {
            for v in cache_vec {
                if v.is_row() {
                    unsafe { v.dec_ref(); }
                }
            }
        }
        let cols = t_mut.columns.clone();
        let mut row = Vec::with_capacity(cols.len());
        
        let mut pk_val = None;
        let mut pk_idx = None;
        let mut mapped_row = vec![None; cols.len()];
        let mut pos_vals = Vec::new();

        if let Some(ns) = names {
            for (i, n) in ns.iter().enumerate() {
                if n.is_empty() { pos_vals.push(args[i]); }
                else if let Some(ci) = cols.iter().position(|c| &c.name == n) {
                    mapped_row[ci] = Some(args[i]);
                }
            }
        } else { pos_vals.extend_from_slice(args); }

        let mut pos_idx = 0;
        for (ci, col) in cols.iter().enumerate() {
            if col.is_auto {
                let num_cols = cols.len();
                let num_rows = t_mut.len();
                let mut max = 0;
                for r_idx in 0..num_rows {
                    let cell = t_mut.rows[r_idx * num_cols + ci];
                    if cell.is_int() {
                        max = max.max(cell.as_i64());
                    }
                }
                row.push(Value::from_i64(max + 1));
            } else {
                let val = if let Some(v) = mapped_row[ci] { v }
                          else if pos_idx < pos_vals.len() { let v = pos_vals[pos_idx]; pos_idx += 1; v }
                          else { Value::from_bool(false) };
                unsafe { val.inc_ref(); }
                row.push(val);
                if col.is_pk { pk_val = Some(val); pk_idx = Some(ci); }
            }
        }

        let mut replaced = false;
        if kind == MethodKind::Save && pk_idx.is_some() {
            let pki = pk_idx.unwrap();
            let pkv = pk_val.unwrap();
            let num_cols = cols.len();
            let num_rows = t_mut.len();
            let mut existing_idx = None;
            for r_idx in 0..num_rows {
                if t_mut.rows[r_idx * num_cols + pki] == pkv {
                    existing_idx = Some(r_idx);
                    break;
                }
            }
            if let Some(row_idx) = existing_idx {
                let start_idx = row_idx * num_cols;
                let mut old_row = Vec::with_capacity(num_cols);
                for c_idx in 0..num_cols {
                    let cell_idx = start_idx + c_idx;
                    let old_val = t_mut.rows[cell_idx];
                    old_row.push(old_val);
                    t_mut.rows[cell_idx] = row[c_idx];
                }
                for v in old_row { unsafe { v.dec_ref(); } }
                replaced = true;
                
                if let Some(binding) = &t_mut.sql_binding {
                    let mut sql = format!("UPDATE [{}] SET ", binding.table_name);
                    let mut first = true;
                    let mut pieces: Vec<Value> = Vec::new();
                    for (ci, col) in cols.iter().enumerate() {
                        if ci == pki { continue; }
                        if !first { sql.push_str(", "); }
                        first = false;
                        sql.push_str(&format!("[{}] = ?", col.name));
                        pieces.push(row[ci]);
                    }
                    sql.push_str(&format!(" WHERE [{}] = ?", cols[pki].name));
                    pieces.push(row[pki]);
                    
                    let conn = binding.db_conn.lock();
                    if let Ok(mut stmt) = conn.prepare(&sql) {
                        for (i, v) in pieces.iter().enumerate() {
                            match () {
                                _ if v.is_int() => { let _ = stmt.raw_bind_parameter(i + 1, v.as_i64() as i64); }
                                _ if v.is_float() => { let _ = stmt.raw_bind_parameter(i + 1, v.as_f64() as f64); }
                                _ if v.is_bool() => { let _ = stmt.raw_bind_parameter(i + 1, if v.as_bool() { 1i64 } else { 0i64 }); }
                                _ => { let _ = stmt.raw_bind_parameter(i + 1, v.to_string()); }
                            }
                        }
                        let _ = stmt.raw_execute();
                    }
                }
            }
        }

        if !replaced {
            t_mut.rows.extend(row.clone());
            if let Some(binding) = &t_mut.sql_binding {
                let mut sql = format!("INSERT INTO [{}] (", binding.table_name);
                let mut vals_sql = String::from("VALUES (");
                let mut pieces: Vec<Value> = Vec::new();
                let mut first = true;
                for (ci, col) in cols.iter().enumerate() {
                    if !first { sql.push_str(", "); vals_sql.push_str(", "); }
                    first = false;
                    sql.push_str(&format!("[{}]", col.name));
                    vals_sql.push('?');
                    pieces.push(row[ci]);
                }
                sql.push_str(") "); sql.push_str(&vals_sql); sql.push(')');
                
                let conn = binding.db_conn.lock();
                if let Ok(mut stmt) = conn.prepare(&sql) {
                    for (i, v) in pieces.iter().enumerate() {
                        match () {
                            _ if v.is_int() => { let _ = stmt.raw_bind_parameter(i + 1, v.as_i64()); }
                            _ if v.is_float() => { let _ = stmt.raw_bind_parameter(i + 1, v.as_f64()); }
                            _ if v.is_bool() => { let _ = stmt.raw_bind_parameter(i + 1, if v.as_bool() { 1 } else { 0 }); }
                            _ => { let _ = stmt.raw_bind_parameter(i + 1, v.to_string()); }
                        }
                    }
                    let _ = stmt.raw_execute();
                }
            }
        }

        let mut insert_id = 0;
        if let Some(binding) = &t_mut.sql_binding {
            insert_id = binding.db_conn.lock().last_insert_rowid();
        }

        fn make_insert_result(affected: i64, insert_id: i64) -> Value {
            let mut obj = Vec::new();
            obj.push((std::sync::Arc::new("affected".to_string()), crate::vm::object::JsonVal::Int(affected)));
            obj.push((std::sync::Arc::new("insertId".to_string()), crate::vm::object::JsonVal::Int(insert_id)));
            let jv = crate::vm::object::JsonVal::Object(Arc::new(RwLock::new(obj)));
            Value::from_json(Arc::new(crate::vm::object::JsonObj::new(jv)))
        }
        let res = make_insert_result(1, insert_id);
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = res;
        
        OpResult::Continue
    }
}
