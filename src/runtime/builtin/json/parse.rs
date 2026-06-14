use crate::vm::value::Value;
use crate::vm::utils::json::json_val_to_value;

// Implementation of JSON parsing for the XCX runtime.
// Parses a JSON string into an XCX Value (Map, Array, or primitive).
pub fn handle_json_parse(json_str: &str) -> Value {
    // Try strict JSON first
    if let Ok(v) = serde_json::from_str(json_str) {
        let json_val = crate::vm::object::JsonVal::from_serde(v);
        let res = json_val_to_value(&json_val);
        unsafe { res.inc_ref(); }
        return res;
    }

    // Try relaxed parsing for XCX-style literals like {1, 2, 3}
    let relaxed = relaxed_preprocess(json_str);
    match serde_json::from_str(&relaxed) {
        Ok(v) => {
            let json_val = crate::vm::object::JsonVal::from_serde(v);
            let res = json_val_to_value(&json_val);
            unsafe { res.inc_ref(); }
            res
        }
        Err(_) => panic!("halt.fatal: Invalid JSON (R305)"),
    }
}

fn relaxed_preprocess(s: &str) -> String {
    let mut result = s.as_bytes().to_vec();
    let mut stack = Vec::new(); // (index, has_colon)
    let mut in_string = false;
    let mut escaped = false;

    for i in 0..result.len() {
        let b = result[i];
        if in_string {
            if escaped { escaped = false; }
            else if b == b'\\' { escaped = true; }
            else if b == b'"' { in_string = false; }
        } else {
            match b {
                b'"' => { in_string = true; }
                b'{' => { stack.push((i, false)); }
                b':' => {
                    if let Some(top) = stack.last_mut() {
                        top.1 = true;
                    }
                }
                b'}' => {
                    if let Some((start_idx, has_colon)) = stack.pop() {
                        if !has_colon {
                            result[start_idx] = b'[';
                            result[i] = b']';
                        }
                    }
                }
                _ => {}
            }
        }
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

pub fn json_parse_impl(src: Value) -> Value {
    let s = src.to_string();
    handle_json_parse(&s)
}
