use crate::vm::value::{Value, TAG_STR, TAG_ARR, TAG_SET, TAG_MAP, TAG_TBL, TAG_ROW, TAG_JSON, TAG_DATE};
use crate::vm::object::{StringObj, JsonVal, JsonObj};
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
            let b = v.as_string();
            // XCX strings are always valid UTF-8 — the parser and all string
            // construction paths guarantee this invariant.
            let s = unsafe { String::from_utf8_unchecked(b.data.clone()) };
            JsonVal::String(Arc::new(s))
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
            for (k, val) in b.iter() { obj.push((Arc::new(k.to_string()), value_to_json(val))); }
            JsonVal::Object(Arc::new(RwLock::new(obj)))
        }
        TAG_TBL => {
            let tbl_arc = v.as_table();
            let t = tbl_arc.read();
            let mut rows = Vec::new();
            for rt in t.rows.iter() {
                let mut obj = Vec::new();
                for (i, col) in t.columns.iter().enumerate() {
                    if i < rt.len() {
                        obj.push((Arc::new(col.name.clone()), value_to_json(&rt[i])));
                    }
                }
                rows.push(JsonVal::Object(Arc::new(RwLock::new(obj))));
            }
            JsonVal::Array(Arc::new(RwLock::new(rows)))
        }
        TAG_ROW => {
            let r = v.as_row();
            let t = r.table.read();
            let row_idx = r.row_idx as usize;
            if row_idx < t.rows.len() {
                let mut obj = Vec::new();
                for (i, col) in t.columns.iter().enumerate() {
                    if i < t.rows[row_idx].len() {
                        obj.push((Arc::new(col.name.clone()), value_to_json(&t.rows[row_idx][i])));
                    }
                }
                JsonVal::Object(Arc::new(RwLock::new(obj)))
            } else {
                JsonVal::Null
            }
        }
        TAG_JSON => {
            let json_ptr = if v.tag == crate::vm::value::nan_boxing::TAG_ARENA {
                crate::vm::value::heap_object::arena_ptr::<crate::vm::object::JsonObj>(v)
            } else {
                v.unpack_ptr::<crate::vm::object::JsonObj>()
            };
            let json_obj = unsafe { &*json_ptr };
            json_obj.root.clone()
        }
        TAG_DATE => {
            let ts = v.as_date();
            let dt = chrono::DateTime::from_timestamp_millis(ts).unwrap().naive_utc();
            JsonVal::String(Arc::new(dt.format("%Y-%m-%d %H:%M:%S").to_string()))
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
        JsonVal::String(s) => Value::from_string(Arc::new(StringObj::new(s.as_ref().clone().into_bytes()))),
        JsonVal::Array(_) | JsonVal::Object(_) => {
            Value::from_json(Arc::new(JsonObj::new(v.clone())))
        }
    }
}

pub fn set_json_value_at_path(target: &mut JsonVal, path: &str, value: JsonVal) {
    let pointer = super::path::normalize_json_path(path);
    let parts: Vec<&str> = pointer.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        *target = value;
        return;
    }
    
    let mut current_node = target.clone();
    
    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;

        if is_last {
            match &current_node {
                JsonVal::Object(obj) => {
                    let mut obj_write = obj.write();
                    if let Some(pos) = obj_write.iter().position(|(k, _)| k.as_str() == *part) {
                        obj_write[pos] = (Arc::new(part.to_string()), value.clone());
                    } else {
                        obj_write.push((Arc::new(part.to_string()), value.clone()));
                    }
                }
                JsonVal::Array(arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        let mut arr_write = arr.write();
                        if idx < arr_write.len() {
                            arr_write[idx] = value.clone();
                        } else {
                            while arr_write.len() < idx {
                                arr_write.push(JsonVal::Null);
                            }
                            arr_write.push(value.clone());
                        }
                    }
                }
                _ => {}
            }
        } else {
            let next_part = parts[i + 1];
            let next_is_array = next_part.parse::<usize>().is_ok();
            
            let next_node = match &current_node {
                JsonVal::Object(obj) => {
                    let mut obj_write = obj.write();
                    if let Some(pos) = obj_write.iter().position(|(k, _)| k.as_str() == *part) {
                        obj_write[pos].1.clone()
                    } else {
                        let new_node = if next_is_array {
                            JsonVal::Array(Arc::new(RwLock::new(Vec::new())))
                        } else {
                            JsonVal::Object(Arc::new(RwLock::new(Vec::new())))
                        };
                        obj_write.push((Arc::new(part.to_string()), new_node.clone()));
                        new_node
                    }
                }
                JsonVal::Array(arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        let mut arr_write = arr.write();
                        if idx < arr_write.len() {
                            arr_write[idx].clone()
                        } else {
                            while arr_write.len() <= idx {
                                arr_write.push(JsonVal::Null);
                            }
                            let new_node = if next_is_array {
                                JsonVal::Array(Arc::new(RwLock::new(Vec::new())))
                            } else {
                                JsonVal::Object(Arc::new(RwLock::new(Vec::new())))
                            };
                            arr_write[idx] = new_node.clone();
                            new_node
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            };
            current_node = next_node;
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
                    h_map.push((Arc::new(name), JsonVal::String(Arc::new(val.to_string()))));
                }
            }
            let text = resp.into_string().unwrap_or_default();
            if text.len() > 10 * 1024 * 1024 {
                let mut res = Vec::new();
                res.push((Arc::new("status".to_string()), JsonVal::Int(413)));
                res.push((Arc::new("ok".to_string()),     JsonVal::Bool(false)));
                res.push((Arc::new("error".to_string()),  JsonVal::String(Arc::new("Body too large".to_string()))));
                JsonVal::Object(Arc::new(RwLock::new(res)))
            } else {
                let body_val = if let Ok(serde_res) = serde_json::from_str::<serde_json::Value>(&text) {
                    JsonVal::from_serde(serde_res)
                } else {
                    JsonVal::String(Arc::new(text))
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
                    h_map.push((Arc::new(name), JsonVal::String(Arc::new(val.to_string()))));
                }
            }
            let text = resp.into_string().unwrap_or_default();
            let body_val = if let Ok(serde_res) = serde_json::from_str::<serde_json::Value>(&text) {
                JsonVal::from_serde(serde_res)
            } else {
                JsonVal::String(Arc::new(text))
            };
            let mut res = Vec::new();
            res.push((Arc::new("status".to_string()),  JsonVal::Int(code as i64)));
            res.push((Arc::new("ok".to_string()),      JsonVal::Bool(false)));
            res.push((Arc::new("error".to_string()),   JsonVal::String(Arc::new(format!("Status code {}", code)))));
            res.push((Arc::new("body".to_string()),    body_val));
            res.push((Arc::new("headers".to_string()), JsonVal::Object(Arc::new(RwLock::new(h_map)))));
            JsonVal::Object(Arc::new(RwLock::new(res)))
        }
        Err(e) => {
            let mut res = Vec::new();
            res.push((Arc::new("status".to_string()), JsonVal::Int(0)));
            res.push((Arc::new("ok".to_string()),     JsonVal::Bool(false)));
            res.push((Arc::new("error".to_string()),  JsonVal::String(Arc::new(e.to_string()))));
            JsonVal::Object(Arc::new(RwLock::new(res)))
        }
    }
}
