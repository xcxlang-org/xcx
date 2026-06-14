use std::ops::{Deref, DerefMut};
use crate::vm::value::Value;

// Map object representation.
pub struct MapObj {
    pub elements: Vec<(Value, Value)>,
}

impl PartialEq for MapObj {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl MapObj {
    pub fn new(elements: Vec<(Value, Value)>) -> Self {
        Self { elements }
    }
}

impl Deref for MapObj {
    type Target = Vec<(Value, Value)>;
    fn deref(&self) -> &Self::Target { &self.elements }
}
impl DerefMut for MapObj {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.elements }
}

impl Drop for MapObj {
    fn drop(&mut self) {
        for (k, v) in self.elements.iter() {
            unsafe {
                k.dec_ref();
                v.dec_ref();
            }
        }
    }
}
