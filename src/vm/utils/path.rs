use crate::vm::value::Value;
use std::sync::Arc;

pub fn normalize_json_path(path: &str) -> String {
    if path.is_empty() { return String::new(); }
    let needs_replace = path.contains('.') || path.contains('[') || path.contains(']');
    if !needs_replace {
        if path.starts_with('/') {
            return path.to_string();
        } else {
            let mut p = String::with_capacity(path.len() + 1);
            p.push('/');
            p.push_str(path);
            return p;
        }
    }
    
    let mut p = String::with_capacity(path.len() + 1);
    if !path.starts_with('/') {
        p.push('/');
    }
    for c in path.chars() {
        match c {
            '.' | '[' => p.push('/'),
            ']' => {},
            _ => p.push(c),
        }
    }
    p
}

pub fn get_path_value_xcx(root: Value, path: &str) -> Option<Value> {
    let pp = normalize_json_path(path);
    if pp.is_empty() || pp == "/" {
        unsafe { root.inc_ref(); }
        return Some(root);
    }
    let parts: Vec<&str> = pp.split('/').filter(|s| !s.is_empty()).collect();
    let mut current = root;
    unsafe { current.inc_ref(); }

    for part in parts {
        let tag = current.tag;
        let next = if current.is_array() {
            let idx = part.parse::<usize>().unwrap_or(u32::MAX as usize);
            let arr_rc = current.as_array();
            let arr = arr_rc.read();
            if idx < arr.elements.len() {
                let v = arr.elements[idx];
                unsafe { v.inc_ref(); }
                Some(v)
            } else { None }
        } else if current.is_map() {
            let map_rc = current.as_map();
            let map = map_rc.read();
            if let Some((_, v)) = map.elements.iter().find(|(k, _)| k.matches_str(part)) {
                unsafe { (v as &Value).inc_ref(); }
                Some(*v)
            } else { None }
        } else if tag == crate::vm::value::TAG_JSON {
            let json_rc = current.as_json();
            let pointer = format!("/{}", part);
            let v = json_rc.root.pointer(&pointer);
            match v {
                Some(v) => Some(crate::vm::utils::json::json_val_to_value(&v)),
                None => None,
            }
        } else {
            None
        };
        
        unsafe { current.dec_ref(); }
        if let Some(n) = next {
            current = n;
            if !current.is_ptr() && !current.is_int() && !current.is_float() && !current.is_bool() { break; }
        } else {
            return None;
        }
    }
    Some(current)
}

pub fn set_path_value_xcx(root: Value, path: &str, value: Value) {
    let pp = normalize_json_path(path);
    let parts: Vec<&str> = pp.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() { return; }
    
    let mut current = root;
    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        if current.is_array() {
            let idx = part.parse::<usize>().unwrap_or(u32::MAX as usize);
            let arr_rc = current.as_array();
            if is_last {
                let mut arr = arr_rc.write();
                if idx < arr.elements.len() {
                    let old = arr.elements[idx];
                    unsafe { value.inc_ref(); }
                    arr.elements[idx] = value;
                    unsafe { old.dec_ref(); }
                } else if idx == arr.elements.len() {
                    unsafe { value.inc_ref(); }
                    arr.elements.push(value);
                }
                return;
            }
            let arr = arr_rc.read();
            if idx < arr.elements.len() && (arr.elements[idx].is_array() || arr.elements[idx].is_map()) {
                current = arr.elements[idx];
            } else { return; }
        } else if current.is_map() {
            let map_rc = current.as_map();
            if is_last {
                let mut map = map_rc.write();
                if let Some(e) = map.elements.iter_mut().find(|(k, _)| k.to_string() == *part) {
                    let old_v = e.1;
                    unsafe { value.inc_ref(); }
                    e.1 = value;
                    unsafe { old_v.dec_ref(); }
                } else {
                    let key = Value::from_string(Arc::new(crate::vm::object::StringObj::new(part.to_string().into_bytes())));
                    unsafe { key.inc_ref(); value.inc_ref(); }
                    map.elements.push((key, value));
                }
                return;
            }
            let map = map_rc.read();
            if let Some((_, v)) = map.elements.iter().find(|(k, _)| k.to_string() == *part) {
                if v.is_map() || v.is_array() {
                    current = *v;
                } else { return; }
            } else { return; }
        } else { return; }
    }
}

pub fn validate_path_safety(path: &str) {
    if path.contains("..") || path.starts_with('/') || (path.len() > 1 && path.as_bytes()[1] == b':') {
        eprintln!("HALT.FATAL: Security violation - illegal path access: {}", path);
        std::process::exit(1);
    }
}
