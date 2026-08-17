use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::HashMap;
use std::cell::RefCell;

thread_local! {
    static KEY_CACHE: RefCell<HashMap<String, Arc<String>>> = RefCell::new(HashMap::with_capacity(64));
}

pub fn intern_key(k: String) -> Arc<String> {
    KEY_CACHE.with(|cache| {
        let mut map = cache.borrow_mut();
        if let Some(arc) = map.get(&k) {
            Arc::clone(arc)
        } else {
            let arc = Arc::new(k.clone());
            if map.len() < 1000 {
                map.insert(k, Arc::clone(&arc));
            }
            arc
        }
    })
}

#[derive(Clone)]
pub enum JsonVal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Arc<crate::vm::object::StringObj>),
    Array(Arc<RwLock<Vec<JsonVal>>>),
    Object(Arc<RwLock<Vec<(Arc<String>, JsonVal)>>>),
}

fn escape_str_to_buf(s: &str, buf: &mut String) {
    let bytes = s.as_bytes();
    let mut needs_escaping = false;
    for &b in bytes {
        if b == b'"' || b == b'\\' || b < 0x20 {
            needs_escaping = true;
            break;
        }
    }
    if !needs_escaping {
        let len = bytes.len();
        let current_len = buf.len();
        unsafe {
            let vec = buf.as_mut_vec();
            vec.reserve(len + 2);
            let ptr = vec.as_mut_ptr().add(current_len);
            *ptr = b'"';
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(1), len);
            *ptr.add(len + 1) = b'"';
            vec.set_len(current_len + len + 2);
        }
    } else {
        buf.push('"');
        let mut bytes = bytes;
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b'"' || b == b'\\' || b < 0x20 {
                unsafe { buf.as_mut_vec().extend_from_slice(&bytes[..i]) };
                match b {
                    b'"' => buf.push_str("\\\""),
                    b'\\' => buf.push_str("\\\\"),
                    0x08 => buf.push_str("\\b"),
                    0x0C => buf.push_str("\\f"),
                    b'\n' => buf.push_str("\\n"),
                    b'\r' => buf.push_str("\\r"),
                    b'\t' => buf.push_str("\\t"),
                    _ => {
                        use std::fmt::Write;
                        write!(buf, "\\u{:04x}", b as u32).unwrap();
                    }
                }
                bytes = &bytes[i + 1..];
                i = 0;
            } else {
                i += 1;
            }
        }
        unsafe { buf.as_mut_vec().extend_from_slice(bytes) };
        buf.push('"');
    }
}

impl JsonVal {
    pub fn from_serde(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => JsonVal::Null,
            serde_json::Value::Bool(b) => JsonVal::Bool(b),
            serde_json::Value::Number(num) => {
                if let Some(i) = num.as_i64() {
                    JsonVal::Int(i)
                } else if let Some(f) = num.as_f64() {
                    JsonVal::Float(f)
                } else {
                    JsonVal::Null
                }
            }
            serde_json::Value::String(s) => JsonVal::String(Arc::new(crate::vm::object::StringObj::new(s.into_bytes()))),
            serde_json::Value::Array(arr) => {
                let vec: Vec<JsonVal> = arr.into_iter().map(JsonVal::from_serde).collect();
                JsonVal::Array(Arc::new(RwLock::new(vec)))
            }
            serde_json::Value::Object(o) => {
                let vec: Vec<(Arc<String>, JsonVal)> = o.into_iter().map(|(k, v)| (intern_key(k), JsonVal::from_serde(v))).collect();
                JsonVal::Object(Arc::new(RwLock::new(vec)))
            }
        }
    }

    pub fn to_serde(&self) -> serde_json::Value {
        match self {
            JsonVal::Null => serde_json::Value::Null,
            JsonVal::Bool(b) => serde_json::Value::Bool(*b),
            JsonVal::Int(i) => serde_json::Value::Number((*i).into()),
            JsonVal::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or_else(|| serde_json::Number::from(0))),
            JsonVal::String(s) => serde_json::Value::String(String::from_utf8_lossy(&s.data).into_owned()),
            JsonVal::Array(a) => {
                let mut arr = vec![];
                for v in a.read().iter() {
                    arr.push(v.to_serde());
                }
                serde_json::Value::Array(arr)
            }
            JsonVal::Object(o) => {
                let mut map = serde_json::Map::new();
                for (k, v) in o.read().iter() {
                    map.insert(k.to_string(), v.to_serde());
                }
                serde_json::Value::Object(map)
            }
        }
    }

    pub fn to_string_buf(&self, buf: &mut String) {
        match self {
            JsonVal::Null => buf.push_str("null"),
            JsonVal::Bool(b) => buf.push_str(if *b { "true" } else { "false" }),
            JsonVal::Int(i) => {
                let mut n = *i;
                if n == 0 {
                    buf.push('0');
                } else {
                    let mut is_neg = false;
                    if n < 0 {
                        is_neg = true;
                        if n == std::i64::MIN {
                            buf.push_str("-9223372036854775808");
                            return;
                        } else {
                            n = -n;
                        }
                    }
                    let mut temp = [b'0'; 20];
                    let mut idx = 20;
                    while n > 0 {
                        idx -= 1;
                        temp[idx] = b'0' + (n % 10) as u8;
                        n /= 10;
                    }
                    let digits = &temp[idx..];
                    let digits_len = digits.len();
                    let current_len = buf.len();
                    unsafe {
                        let vec = buf.as_mut_vec();
                        let extra = if is_neg { 1 } else { 0 };
                        vec.reserve(digits_len + extra);
                        let ptr = vec.as_mut_ptr().add(current_len);
                        if is_neg {
                            *ptr = b'-';
                            std::ptr::copy_nonoverlapping(digits.as_ptr(), ptr.add(1), digits_len);
                        } else {
                            std::ptr::copy_nonoverlapping(digits.as_ptr(), ptr, digits_len);
                        }
                        vec.set_len(current_len + digits_len + extra);
                    }
                }
            }
            JsonVal::Float(f) => {
                use std::fmt::Write;
                write!(buf, "{}", f).unwrap();
            }
            JsonVal::String(s) => {
                if let Ok(s_str) = std::str::from_utf8(&s.data) {
                    escape_str_to_buf(s_str, buf);
                } else {
                    escape_str_to_buf(&String::from_utf8_lossy(&s.data), buf);
                }
            }
            JsonVal::Array(a) => {
                buf.push('[');
                let vec = unsafe { &*(*a).data_ptr() };
                for (i, v) in vec.iter().enumerate() {
                    if i > 0 { buf.push(','); }
                    v.to_string_buf(buf);
                }
                buf.push(']');
            }
            JsonVal::Object(o) => {
                buf.push('{');
                let vec = unsafe { &*(*o).data_ptr() };
                for (idx_enum, (k, v)) in vec.iter().enumerate() {
                    if idx_enum > 0 { buf.push(','); }
                    escape_str_to_buf(k.as_str(), buf);
                    buf.push(':');
                    v.to_string_buf(buf);
                }
                buf.push('}');
            }
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, JsonVal::Object(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, JsonVal::Array(_))
    }

    pub fn pointer(&self, path: &str) -> Option<JsonVal> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current = self.clone();
        for part in parts {
            let mut next = None;
            match &current {
                JsonVal::Object(o) => {
                    let o_read = unsafe { &*(*o).data_ptr() };
                    if let Some((_, val)) = o_read.iter().find(|(k, _)| k.as_str() == part) {
                        next = Some(val.clone());
                    }
                }
                JsonVal::Array(a) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        let a_read = unsafe { &*(*a).data_ptr() };
                        if idx < a_read.len() {
                            next = Some(a_read[idx].clone());
                        }
                    }
                }
                _ => return None,
            }
            if let Some(n) = next {
                current = n;
            } else {
                return None;
            }
        }
        Some(current)
    }

    pub fn make_mutable(&mut self) {
        match self {
            JsonVal::Array(a) => {
                if Arc::strong_count(a) > 1 {
                    let vec = unsafe { &*(*a).data_ptr() };
                    *self = JsonVal::Array(Arc::new(RwLock::new(vec.clone())));
                }
            }
            JsonVal::Object(o) => {
                if Arc::strong_count(o) > 1 {
                    let vec = unsafe { &*(*o).data_ptr() };
                    *self = JsonVal::Object(Arc::new(RwLock::new(vec.clone())));
                }
            }
            _ => {}
        }
    }

    pub fn deep_clone(&self) -> Self {
        match self {
            JsonVal::Array(a) => {
                let vec = unsafe { &*(*a).data_ptr() };
                let cloned_vec: Vec<JsonVal> = vec.iter().map(|v| v.deep_clone()).collect();
                JsonVal::Array(Arc::new(RwLock::new(cloned_vec)))
            }
            JsonVal::Object(o) => {
                let vec = unsafe { &*(*o).data_ptr() };
                let cloned_vec: Vec<(Arc<String>, JsonVal)> = vec.iter().map(|(k, v)| (Arc::clone(k), v.deep_clone())).collect();
                JsonVal::Object(Arc::new(RwLock::new(cloned_vec)))
            }
            other => other.clone(),
        }
    }
}

