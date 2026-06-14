use parking_lot::Mutex;
use std::sync::atomic::AtomicBool;
use super::json_val::JsonVal;

// JSON object representation. 
// Uses the native JsonVal representation with Copy-On-Write logic instead of heavy locks.
pub struct JsonObj {
    pub root: JsonVal,
    pub cached_str: Mutex<Option<std::sync::Arc<super::string_obj::StringObj>>>,
    pub dirty: AtomicBool,
}

impl JsonObj {
    pub fn new(value: JsonVal) -> Self {
        Self { 
            root: value,
            cached_str: Mutex::new(None),
            dirty: AtomicBool::new(true),
        }
    }
}

impl Clone for JsonObj {
    fn clone(&self) -> Self {
        Self::new(self.root.clone())
    }
}
