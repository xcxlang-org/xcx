use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::value::Value;
use crate::vm::object::{ArrayObj, SetObj, MapObj};

pub struct RuntimeOps;

impl RuntimeOps {
    #[inline]
    pub fn array_init(elements: &[Value]) -> Value {
        let mut elems = Vec::with_capacity(elements.len());
        for &v in elements {
            unsafe { v.inc_ref(); }
            elems.push(v);
        }
        Value::from_array(Arc::new(RwLock::new(ArrayObj::new(elems))))
    }

    #[inline]
    pub fn set_init(elements: &[Value]) -> Value {
        let mut set_elements = std::collections::BTreeSet::new();
        for &v in elements {
            unsafe { v.inc_ref(); }
            set_elements.insert(v);
        }
        Value::from_set(Arc::new(RwLock::new(SetObj::new(set_elements))))
    }

    #[inline]
    pub fn map_init(elements: &[Value]) -> Value {
        let count = elements.len() / 2;
        let mut map = Vec::with_capacity(count);
        for i in 0..count {
            let k = elements[i * 2];
            let v = elements[i * 2 + 1];
            unsafe { k.inc_ref(); v.inc_ref(); }
            map.push((k, v));
        }
        Value::from_map(Arc::new(RwLock::new(MapObj::new(map))))
    }

    pub fn matches_type(val: &Value, ty: &crate::frontend::ast::Type) -> bool {
        use crate::frontend::ast::Type;
        match ty {
            Type::Int => val.is_int(),
            Type::Float => val.is_float(),
            Type::String => val.is_string(),
            Type::Bool => val.is_bool(),
            Type::Date => val.is_date(),
            Type::Json => val.is_json(),
            Type::Array(_) => val.is_array(),
            Type::Map(_, _) => val.is_map(),
            Type::Set(_) => val.is_set(),
            Type::Table(_) => val.is_table(),
            Type::Fiber(_) => val.is_fiber(),
            Type::Database => val.is_db(),
            _ => true,
        }
    }

    pub fn table_init(
        skeleton_val: Value,
        row_count: u32,
        locals: &[Value],
    ) -> Result<Value, String> {
        if !skeleton_val.is_table() {
            return Err("Skeleton is not a table".to_string());
        }

        let skeleton = skeleton_val.as_table();
        let skeleton_rd = skeleton.read();
        let cols = skeleton_rd.columns.clone();
        let ncol = cols.len();

        let mut rows = Vec::with_capacity(row_count as usize);
        let mut current_offset = 0;
        for row_idx in 0..row_count {
            let mut row = Vec::with_capacity(ncol);
            for c in 0..ncol {
                if cols[c].is_auto {
                    row.push(Value::from_i64(row_idx as i64 + 1));
                } else {
                    if current_offset >= locals.len() {
                        return Err("Not enough values for table initialization".to_string());
                    }
                    let val = locals[current_offset];
                    if !Self::matches_type(&val, &cols[c].ty) {
                        return Err(format!(
                            "XCX Table Error: Column '{}' expects type {:?}, but got {}",
                            cols[c].name, cols[c].ty, val.type_name()
                        ));
                    }
                    unsafe { val.inc_ref(); }
                    row.push(val);
                    current_offset += 1;
                }
            }
            rows.push(row);
        }

        let table_name = skeleton_rd.table_name.clone();

        Ok(Value::from_table(Arc::new(RwLock::new(crate::vm::object::TableObj {
            table_name,
            columns: cols,
            rows,
            sql_binding: None,
            sql_where: None,
            pending_op: None,
        }))))
    }

    pub fn get_member(c: Value, name: &str) -> Value {
        let mut res = Value::from_bool(false);
        match c.tag() {
            crate::vm::value::Tag::String => {
                if name == "length" {
                    let s = c.as_string();
                    let s_str = String::from_utf8_lossy(&s.data);
                    res = Value::from_i64(s_str.chars().count() as i64);
                }
            }
            crate::vm::value::Tag::Array => {
                let arr = c.as_array();
                let len = arr.read().len() as i64;
                if name == "size" || name == "count" || name == "length" {
                    res = Value::from_i64(len);
                }
            }
            crate::vm::value::Tag::Set => {
                let set = c.as_set();
                let len = set.read().elements.len() as i64;
                if name == "size" || name == "count" {
                    res = Value::from_i64(len);
                }
            }
            crate::vm::value::Tag::Map => {
                let map = c.as_map();
                let len = map.read().elements.len() as i64;
                if name == "size" || name == "count" {
                    res = Value::from_i64(len);
                }
            }
            crate::vm::value::Tag::Json => {
                let j = c.as_json();
                match &j.root {
                    crate::vm::object::JsonVal::Object(o) => {
                        let o_read = o.read();
                        if let Some((_, val)) = o_read.iter().find(|(k, _)| k.as_str() == name) {
                            res = crate::vm::utils::json_val_to_value(val);
                        }
                        if name == "size" || name == "count" || name == "length" {
                            res = Value::from_i64(o_read.len() as i64);
                        }
                    }
                    crate::vm::object::JsonVal::Array(a) => {
                        let a_read = a.read();
                        if name == "size" || name == "count" || name == "length" {
                            res = Value::from_i64(a_read.len() as i64);
                        }
                    }
                    _ => {}
                }
            }
            crate::vm::value::Tag::Table => {
                let tbl = c.as_table();
                let tbl_rd = tbl.read();
                if name == "count" || name == "size" {
                    res = Value::from_i64(tbl_rd.rows.len() as i64);
                } else if name == "name" {
                    res = Value::from_string(Arc::new(crate::vm::object::StringObj::new(tbl_rd.table_name.as_bytes().to_vec())));
                }
            }
            crate::vm::value::Tag::Date => {
                let ts = c.as_date();
                use chrono::{Datelike, Timelike};
                if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts) {
                    let utc = dt.naive_utc();
                    match name {
                        "year" => res = Value::from_i64(utc.year() as i64),
                        "month" => res = Value::from_i64(utc.month() as i64),
                        "day" => res = Value::from_i64(utc.day() as i64),
                        "hour" => res = Value::from_i64(utc.hour() as i64),
                        "minute" => res = Value::from_i64(utc.minute() as i64),
                        "second" => res = Value::from_i64(utc.second() as i64),
                        "ms" => res = Value::from_i64(dt.timestamp_subsec_millis() as i64),
                        _ => {}
                    }
                }
            }
            crate::vm::value::Tag::Row => {
                let row_ref = c.as_row();
                let tbl = row_ref.table.read();
                let row_idx = row_ref.row_idx as usize;
                if row_idx < tbl.rows.len() {
                    let values = &tbl.rows[row_idx];
                    for i in 0..tbl.columns.len() {
                        if tbl.columns[i].name == name {
                            res = values[i];
                            unsafe { res.inc_ref(); }
                            break;
                        }
                    }
                }
            }
            crate::vm::value::Tag::Database => {
                let db = c.as_database();
                let tables = db.tables.read();
                if let Some(val) = tables.get(name) {
                    res = *val;
                    if res.is_table() {
                        let tbl_rc = res.as_table();
                        let tbl_read = tbl_rc.read();
                        let binding_opt = tbl_read.sql_binding.clone();
                        let cols = tbl_read.columns.clone();
                        drop(tbl_read);

                        if let Some(binding) = binding_opt {
                            let table_name = binding.table_name.clone();
                            let conn = binding.db_conn.lock();
                            let sql = format!("SELECT * FROM [{}]", table_name);
                            if let Ok(mut stmt) = conn.prepare(&sql) {
                                let mut new_rows = Vec::new();
                                if let Ok(rows_iter) = stmt.query_map(rusqlite::params![], |row| {
                                    let mut row_vals = Vec::with_capacity(cols.len());
                                    for (i, col) in cols.iter().enumerate() {
                                        let v = match col.ty {
                                            crate::frontend::ast::Type::Int => {
                                                Value::from_i64(row.get::<_, i64>(i).unwrap_or(0))
                                            }
                                            crate::frontend::ast::Type::Float => {
                                                Value::from_f64(row.get::<_, f64>(i).unwrap_or(0.0))
                                            }
                                            crate::frontend::ast::Type::Bool => {
                                                Value::from_bool(row.get::<_, i32>(i).unwrap_or(0) != 0)
                                            }
                                            crate::frontend::ast::Type::String => {
                                                let s_val = row.get::<_, String>(i).unwrap_or_default();
                                                Value::from_string(Arc::new(crate::vm::object::StringObj::new(s_val.into_bytes())))
                                            }
                                            _ => Value::from_bool(false),
                                        };
                                        row_vals.push(v);
                                    }
                                    Ok(row_vals)
                                }) {
                                    for r in rows_iter {
                                        if let Ok(row) = r {
                                            new_rows.push(row);
                                        }
                                    }
                                }
                                let mut tbl_write = tbl_rc.write();
                                for old_row in std::mem::replace(&mut tbl_write.rows, new_rows) {
                                    for v in old_row {
                                        unsafe { v.dec_ref(); }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        if res.is_ptr() { unsafe { res.inc_ref(); } }
        res
    }

    pub fn set_member(c: Value, name: &str, val: Value) {
        match c.tag() {
            crate::vm::value::Tag::Json => {
                let j = c.as_json();
                if let crate::vm::object::JsonVal::Object(o) = &j.root {
                    let mut obj = o.write();
                    if let Some(pos) = obj.iter().position(|(k, _)| k.as_str() == name) {
                        obj[pos].1 = crate::vm::utils::value_to_json(&val);
                    } else {
                        obj.push((std::sync::Arc::new(name.to_string()), crate::vm::utils::value_to_json(&val)));
                    }
                }
            }
            _ => {}
        }
    }
}
