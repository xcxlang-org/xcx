use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::value::Value;

#[derive(Clone)]
pub struct UpvalueCell {
    pub value: Arc<RwLock<Value>>,
}

impl UpvalueCell {
    pub fn new(val: Value) -> Self {
        Self {
            value: Arc::new(RwLock::new(val)),
        }
    }

    pub fn get(&self) -> Value {
        let v = *self.value.read();
        unsafe { v.inc_ref(); }
        v
    }

    pub fn set(&self, val: Value) {
        let mut w = self.value.write();
        unsafe { w.dec_ref(); }
        *w = val;
        unsafe { val.inc_ref(); }
    }
}

impl Drop for UpvalueCell {
    fn drop(&mut self) {
        // Since it's an Arc, only the last one decrefs the inner value
        if Arc::strong_count(&self.value) == 1 {
            let v = *self.value.read();
            unsafe { v.dec_ref(); }
        }
    }
}
