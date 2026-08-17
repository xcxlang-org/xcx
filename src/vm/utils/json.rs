use crate::vm::value::{Value, TAG_STR, TAG_ARR, TAG_SET, TAG_MAP, TAG_TBL, TAG_ROW, TAG_JSON, TAG_DATE};
use crate::vm::object::{StringObj, JsonVal, JsonObj, json_val::intern_key};
use std::sync::Arc;
use parking_lot::RwLock;

pub fn value_to_json(v: &Value) -> JsonVal {
    if v.is_int() { return JsonVal::Int(v.as_i64()); }
    if v.is_float() { return JsonVal::Float(v.as_f64()); }
    if v.is_bool() { return JsonVal::Bool(v.as_bool()); }
    if !v.is_ptr() { return JsonVal::Null; }
    
    let tag = v.tag;
    match tag {
        TAG_STR => {
            JsonVal::String(v.as_string())
        }
        TAG_ARR => {
            let a_rc = v.as_array();
            let a = a_rc.read();
            JsonVal::Array(Arc::new(RwLock::new(a.iter().map(value_to_json).collect())))
        }
        TAG_SET => {
            let s_rc = v.as_set();
            let s = s_rc.read();
            JsonVal::Array(Arc::new(RwLock::new(s.elements.iter().map(value_to_json).collect())))
        }
        TAG_MAP => {
            let b_rc = v.as_map();
            let b = b_rc.read();
            let mut obj = Vec::new();
            for (k, val) in b.iter() { obj.push((intern_key(k.to_string()), value_to_json(val))); }
            JsonVal::Object(Arc::new(RwLock::new(obj)))
        }
        TAG_TBL => {
            let tbl_arc = v.as_table();
            let t = tbl_arc.read();
            let mut rows = Vec::new();
            let col_keys: Vec<Arc<String>> = t.columns.iter().map(|col| intern_key(col.name.clone())).collect();
            let num_cols = t.columns.len();
            let num_rows = t.len();
            for r_idx in 0..num_rows {
                let mut obj = Vec::with_capacity(num_cols);
                for c_idx in 0..num_cols {
                    let cell_idx = r_idx * num_cols + c_idx;
                    obj.push((Arc::clone(&col_keys[c_idx]), value_to_json(&t.rows[cell_idx])));
                }
                rows.push(JsonVal::Object(Arc::new(RwLock::new(obj))));
            }
            JsonVal::Array(Arc::new(RwLock::new(rows)))
        }
        TAG_ROW => {
            let r = v.as_row();
            let t = r.table.read();
            let row_idx = r.row_idx as usize;
            if row_idx < t.len() {
                let mut obj = Vec::with_capacity(t.columns.len());
                let col_keys: Vec<Arc<String>> = t.columns.iter().map(|col| intern_key(col.name.clone())).collect();
                let num_cols = t.columns.len();
                let start_idx = row_idx * num_cols;
                for c_idx in 0..num_cols {
                    let cell_idx = start_idx + c_idx;
                    obj.push((Arc::clone(&col_keys[c_idx]), value_to_json(&t.rows[cell_idx])));
                }
                JsonVal::Object(Arc::new(RwLock::new(obj)))
            } else {
                JsonVal::Null
            }
        }
        TAG_JSON => {
            let json_ptr = v.unpack_ptr::<crate::vm::object::JsonObj>();
            let json_obj = unsafe { &*json_ptr };
            json_obj.root.clone()
        }
        TAG_DATE => {
            let ts = v.as_date();
            let dt = chrono::DateTime::from_timestamp_millis(ts).unwrap().naive_utc();
            JsonVal::String(Arc::new(StringObj::new(dt.format("%Y-%m-%d %H:%M:%S").to_string().into_bytes())))
        },
        _ => JsonVal::Null,
    }
}

pub fn json_val_to_value(v: &JsonVal) -> Value {
    match v {
        JsonVal::Null    => Value::from_bool(false),
        JsonVal::Bool(b) => Value::from_bool(*b),
        JsonVal::Int(i) => Value::from_i64(*i),
        JsonVal::Float(f) => Value::from_f64(*f),
        JsonVal::String(s) => Value::from_string(s.clone()),
        JsonVal::Array(_) | JsonVal::Object(_) => {
            Value::from_json(Arc::new(JsonObj::new(v.clone())))
        }
    }
}

pub fn set_json_value_at_path(target: &mut JsonVal, path: &str, value: JsonVal) {
    let pointer = super::path::normalize_json_path(path);
    let parts: Vec<&str> = pointer.split('/').filter(|s| !s.is_empty()).collect();
    set_json_val_cow(target, &parts, value);
}

fn set_json_val_cow(node: &mut JsonVal, parts: &[&str], value: JsonVal) {
    if parts.is_empty() {
        *node = value;
        return;
    }
    
    node.make_mutable();
    
    let part = parts[0];
    let rest = &parts[1..];
    
    match node {
        JsonVal::Object(o) => {
            let mut obj_write = o.write();
            if let Some(pos) = obj_write.iter().position(|(k, _)| k.as_str() == part) {
                if rest.is_empty() {
                    obj_write[pos] = (Arc::new(part.to_string()), value);
                } else {
                    let mut child = obj_write[pos].1.clone();
                    set_json_val_cow(&mut child, rest, value);
                    obj_write[pos].1 = child;
                }
            } else {
                let mut new_node = if !rest.is_empty() && rest[0].parse::<usize>().is_ok() {
                    JsonVal::Array(Arc::new(RwLock::new(Vec::new())))
                } else {
                    JsonVal::Object(Arc::new(RwLock::new(Vec::new())))
                };
                set_json_val_cow(&mut new_node, rest, value);
                obj_write.push((Arc::new(part.to_string()), new_node));
            }
        }
        JsonVal::Array(a) => {
            let mut arr_write = a.write();
            if let Ok(idx) = part.parse::<usize>() {
                if idx < arr_write.len() {
                    if rest.is_empty() {
                        arr_write[idx] = value;
                    } else {
                        let mut child = arr_write[idx].clone();
                        set_json_val_cow(&mut child, rest, value);
                        arr_write[idx] = child;
                    }
                } else {
                    while arr_write.len() < idx {
                        arr_write.push(JsonVal::Null);
                    }
                    let mut new_node = if !rest.is_empty() && rest[0].parse::<usize>().is_ok() {
                        JsonVal::Array(Arc::new(RwLock::new(Vec::new())))
                    } else {
                        JsonVal::Object(Arc::new(RwLock::new(Vec::new())))
                    };
                    set_json_val_cow(&mut new_node, rest, value);
                    arr_write.push(new_node);
                }
            }
        }
        _ => {
            let next_is_array = part.parse::<usize>().is_ok();
            *node = if next_is_array {
                JsonVal::Array(Arc::new(RwLock::new(Vec::new())))
            } else {
                JsonVal::Object(Arc::new(RwLock::new(Vec::new())))
            };
            set_json_val_cow(node, parts, value);
        }
    }
}


pub fn build_response_json(result: Result<ureq::Response, ureq::Error>) -> crate::vm::object::JsonVal {
    use crate::vm::object::JsonVal;
    use parking_lot::RwLock;

    match result {
        Ok(resp) => {
            let status = resp.status();
            let mut h_map = Vec::new();
            for name in resp.headers_names() {
                if let Some(val) = resp.header(&name) {
                    h_map.push((Arc::new(name), JsonVal::String(Arc::new(StringObj::new(val.to_string().into_bytes())))));
                }
            }
            use std::io::Read as _;
            let mut buf = Vec::new();
            let text = if resp.into_reader().read_to_end(&mut buf).is_ok() {
                unsafe { String::from_utf8_unchecked(buf) }
            } else {
                String::new()
            };
            if text.len() > 50 * 1024 * 1024 {
                let mut res = Vec::new();
                res.push((Arc::new("status".to_string()), JsonVal::Int(413)));
                res.push((Arc::new("ok".to_string()),     JsonVal::Bool(false)));
                res.push((Arc::new("error".to_string()),  JsonVal::String(Arc::new(StringObj::new("Body too large".to_string().into_bytes())))));
                JsonVal::Object(Arc::new(RwLock::new(res)))
            } else {
                let body_val = if let Ok(serde_res) = serde_json::from_str::<serde_json::Value>(&text) {
                    JsonVal::from_serde(serde_res)
                } else {
                    JsonVal::String(Arc::new(StringObj::new(text.into_bytes())))
                };
                let mut res = Vec::new();
                res.push((Arc::new("status".to_string()),  JsonVal::Int(status as i64)));
                res.push((Arc::new("ok".to_string()),      JsonVal::Bool(status >= 200 && status < 300)));
                res.push((Arc::new("body".to_string()),    body_val));
                res.push((Arc::new("headers".to_string()), JsonVal::Object(Arc::new(RwLock::new(h_map)))));
                JsonVal::Object(Arc::new(RwLock::new(res)))
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            let mut h_map = Vec::new();
            for name in resp.headers_names() {
                if let Some(val) = resp.header(&name) {
                    h_map.push((Arc::new(name), JsonVal::String(Arc::new(StringObj::new(val.to_string().into_bytes())))));
                }
            }
            use std::io::Read as _;
            let mut buf = Vec::new();
            let text = if resp.into_reader().read_to_end(&mut buf).is_ok() {
                unsafe { String::from_utf8_unchecked(buf) }
            } else {
                String::new()
            };
            let body_val = if let Ok(serde_res) = serde_json::from_str::<serde_json::Value>(&text) {
                JsonVal::from_serde(serde_res)
            } else {
                JsonVal::String(Arc::new(StringObj::new(text.into_bytes())))
            };
            let mut res = Vec::new();
            res.push((Arc::new("status".to_string()),  JsonVal::Int(code as i64)));
            res.push((Arc::new("ok".to_string()),      JsonVal::Bool(false)));
            res.push((Arc::new("error".to_string()),   JsonVal::String(Arc::new(StringObj::new(format!("Status code {}", code).into_bytes())))));
            res.push((Arc::new("body".to_string()),    body_val));
            res.push((Arc::new("headers".to_string()), JsonVal::Object(Arc::new(RwLock::new(h_map)))));
            JsonVal::Object(Arc::new(RwLock::new(res)))
        }
        Err(e) => {
            let mut res = Vec::new();
            res.push((Arc::new("status".to_string()), JsonVal::Int(0)));
            res.push((Arc::new("ok".to_string()),     JsonVal::Bool(false)));
            res.push((Arc::new("error".to_string()),  JsonVal::String(Arc::new(StringObj::new(e.to_string().into_bytes())))));
            JsonVal::Object(Arc::new(RwLock::new(res)))
        }
    }
}

pub fn push_json_value_at_path(target: &mut JsonVal, path: &str, value: JsonVal) {
    let pointer = super::path::normalize_json_path(path);
    let parts: Vec<&str> = pointer.split('/').filter(|s| !s.is_empty()).collect();
    push_json_val_cow(target, &parts, value);
}

fn push_json_val_cow(node: &mut JsonVal, parts: &[&str], value: JsonVal) {
    node.make_mutable();
    if parts.is_empty() {
        if let JsonVal::Array(a) = node {
            a.write().push(value);
        } else {
            panic!("halt.error: Cannot push to a non-array JSON object (R306)");
        }
        return;
    }
    
    let part = parts[0];
    let rest = &parts[1..];
    
    match node {
        JsonVal::Object(o) => {
            let mut obj_write = o.write();
            if let Some(pos) = obj_write.iter().position(|(k, _)| k.as_str() == part) {
                let mut child = obj_write[pos].1.clone();
                push_json_val_cow(&mut child, rest, value);
                obj_write[pos].1 = child;
            } else {
                panic!("halt.error: Target array not found in JSON (R306)");
            }
        }
        JsonVal::Array(a) => {
            let mut arr_write = a.write();
            if let Ok(idx) = part.parse::<usize>() {
                if idx < arr_write.len() {
                    let mut child = arr_write[idx].clone();
                    push_json_val_cow(&mut child, rest, value);
                    arr_write[idx] = child;
                } else {
                    panic!("halt.error: Target array not found in JSON (R306)");
                }
            } else {
                panic!("halt.error: Target array not found in JSON (R306)");
            }
        }
        _ => {
            panic!("halt.error: Target array not found in JSON (R306)");
        }
    }
}

