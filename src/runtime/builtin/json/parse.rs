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

/// serde_json caps nesting at 128 levels by default; XCX accepts arbitrarily
/// deep trees, so every user-JSON decode goes through this depth-unlimited
/// deserializer. Runs on the 64MB executor stack, which sets the practical
/// depth ceiling. Deserializes straight into JsonVal — see the `Deserialize`
/// impl in json_val.rs.
fn from_str_unbounded(s: &str) -> Result<crate::vm::object::JsonVal, serde_json::Error> {
    use serde::Deserialize as _;
    let mut de = serde_json::Deserializer::from_str(s);
    de.disable_recursion_limit();
    crate::vm::object::JsonVal::deserialize(&mut de)
}

/// Parses `json_str` into an XCX Value (Map, Array, or primitive).
///
/// Ownership contract (VM builtin convention): the returned Value carries
/// exactly one strong reference, owned by the caller. `Value` has no Drop,
/// so any additional reference returned here is forgotten by every consumer
/// — the interpreter's JsonParse step and the JIT's json-parse FFI wrapper
/// both store or discard the result with a single dec_ref — and a leaked
/// json wrapper pins its entire parse tree.
pub fn handle_json_parse(json_str: &str) -> Value {
    thread_local! {
        static JSON_CACHE: std::cell::RefCell<JsonCache> = std::cell::RefCell::new(JsonCache::new());
    }

    // Only strings within the insert threshold can ever be cached; skipping
    // the lookup for longer payloads avoids hashing them for nothing.
    let cached_val = if json_str.len() <= 16384 {
        JSON_CACHE.with(|c| {
            c.borrow().get(json_str)
        })
    } else {
        None
    };

    if let Some(val) = cached_val {
        // Cache entries must never alias a returned json: every json Value is
        // mutable in place, so a shared tree would leak mutations between
        // independent parses of the same text. Hand out an owned deep copy.
        let fresh = val.deep_clone();
        return json_val_to_value(&fresh);
    }

    // Try strict JSON first
    let json_val = if let Ok(v) = from_str_unbounded(json_str) {
        v
    } else {
        // Try relaxed parsing for XCX-style literals like {1, 2, 3}
        let relaxed = relaxed_preprocess(json_str);
        match from_str_unbounded(&relaxed) {
            Ok(v) => v,
            // R305 is documented as halt.fatal (errors_halt.md): immediate
            // termination with an error exit code — identical to the
            // OpCode::HaltFatal arm, not a raw panic. The offending input
            // snippet is included (newlines flattened) for diagnosis.
            Err(_) => {
                let snippet: String = json_str
                    .chars()
                    .take(80)
                    .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                    .collect();
                eprintln!("XCX Fatal: Invalid JSON (R305): {}", snippet);
                std::process::exit(1);
            }
        }
    };

    if json_str.len() <= 16384 {
        JSON_CACHE.with(|c| {
            c.borrow_mut().insert(json_str.to_string(), json_val.deep_clone());
        });
    }

    json_val_to_value(&json_val)
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
    // A string argument is borrowed straight from its object instead of being
    // copied through to_string() first — parse payloads can be large.
    if src.tag == crate::vm::value::nan_boxing::TAG_STR {
        let arc = crate::vm::value::heap_object::as_string(&src);
        match std::str::from_utf8(&arc.data) {
            Ok(s) => handle_json_parse(s),
            Err(_) => {
                let s = String::from_utf8_lossy(&arc.data).into_owned();
                handle_json_parse(&s)
            }
        }
    } else {
        let s = src.to_string();
        handle_json_parse(&s)
    }
}

/// Reports whether `handle_json_parse` would accept the string (strict serde
/// decode or the relaxed XCX preprocessor). The compilers use this to decide
/// whether a `<<< >>>` literal emits a parsed json value; content that is not
/// valid JSON (e.g. literals containing embedded expressions) keeps the
/// legacy raw-string behavior.
pub fn is_parseable_json(s: &str) -> bool {
    if from_str_unbounded(s).is_ok() {
        return true;
    }
    let relaxed = relaxed_preprocess(s);
    from_str_unbounded(&relaxed).is_ok()
}
