use parking_lot::Mutex;
use std::sync::atomic::AtomicU64;
use super::json_val::JsonVal;

// JSON object representation.
// In-place mutation is shared through `Arc<RwLock<…>>` storage, so a
// serialized-string cache keyed on this wrapper's version would go stale the
// moment a fetched child view mutates the tree — `to_str` therefore always
// serializes fresh. The legacy cache fields below are no longer used by the
// runtime; they remain because tests/e2e/json_concurrency.rs exercises them.
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
