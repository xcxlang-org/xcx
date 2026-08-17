use crate::vm::value::Value;
use crate::vm::utils::json::json_val_to_value;

struct JsonCache {
    map: std::collections::HashMap<String, crate::vm::object::JsonVal>,
    order: std::collections::VecDeque<String>,
    total_len: usize,
}

impl JsonCache {
    fn new() -> Self {
        Self {
            map: std::collections::HashMap::with_capacity(16),
            order: std::collections::VecDeque::with_capacity(16),
            total_len: 0,
        }
    }

    fn get(&self, key: &str) -> Option<crate::vm::object::JsonVal> {
        self.map.get(key).cloned()
    }

    fn insert(&mut self, key: String, val: crate::vm::object::JsonVal) {
        let key_len = key.len();
        
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), val);
            if let Some(pos) = self.order.iter().position(|x| x == &key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
            return;
        }

        while (self.order.len() >= 16 || self.total_len + key_len > 512_000) && !self.order.is_empty() {
            if let Some(oldest) = self.order.pop_front() {
                self.total_len = self.total_len.saturating_sub(oldest.len());
                self.map.remove(&oldest);
            }
        }

        self.total_len += key_len;
        self.order.push_back(key.clone());
        self.map.insert(key, val);
    }
}

// Implementation of JSON parsing for the XCX runtime.
// Parses a JSON string into an XCX Value (Map, Array, or primitive).
pub fn handle_json_parse(json_str: &str) -> Value {
    thread_local! {
        static JSON_CACHE: std::cell::RefCell<JsonCache> = std::cell::RefCell::new(JsonCache::new());
    }

    let cached_val = JSON_CACHE.with(|c| {
        c.borrow().get(json_str)
    });

    if let Some(val) = cached_val {
        let res = json_val_to_value(&val);
        unsafe { res.inc_ref(); }
        return res;
    }

    // Try strict JSON first
    let json_val = if let Ok(v) = serde_json::from_str(json_str) {
        crate::vm::object::JsonVal::from_serde(v)
    } else {
        // Try relaxed parsing for XCX-style literals like {1, 2, 3}
        let relaxed = relaxed_preprocess(json_str);
        match serde_json::from_str(&relaxed) {
            Ok(v) => crate::vm::object::JsonVal::from_serde(v),
            Err(_) => panic!("halt.fatal: Invalid JSON (R305)"),
        }
    };

    if json_str.len() <= 16384 {
        JSON_CACHE.with(|c| {
            c.borrow_mut().insert(json_str.to_string(), json_val.clone());
        });
    }

    let res = json_val_to_value(&json_val);
    unsafe { res.inc_ref(); }
    res
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
