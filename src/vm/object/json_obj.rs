use parking_lot::Mutex;
use std::sync::atomic::AtomicU64;
use super::json_val::JsonVal;

// JSON object representation. 
// Uses the native JsonVal representation with Copy-On-Write logic instead of heavy locks.
pub struct JsonObj {
    pub root: JsonVal,
    pub cached_str: Mutex<Option<std::sync::Arc<super::string_obj::StringObj>>>,
    pub version: AtomicU64,
    pub cached_version: AtomicU64,
}

impl JsonObj {
    pub fn new(value: JsonVal) -> Self {
        Self { 
            root: value,
            cached_str: Mutex::new(None),
            version: AtomicU64::new(0),
            cached_version: AtomicU64::new(1),
        }
    }
}

impl Clone for JsonObj {
    fn clone(&self) -> Self {
        Self::new(self.root.clone())
    }
}
