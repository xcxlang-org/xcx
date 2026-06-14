use std::ops::{Deref, DerefMut};
use super::super::value::Value;

// Array object representation.
pub struct ArrayObj {
    pub elements: Vec<Value>,
}

impl PartialEq for ArrayObj {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl ArrayObj {
    pub fn new(elements: Vec<Value>) -> Self {
        Self { elements }
    }
}

impl Deref for ArrayObj {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target { &self.elements }
}
impl DerefMut for ArrayObj {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.elements }
}

impl Drop for ArrayObj {
    fn drop(&mut self) {
        for val in self.elements.iter() {
            unsafe { val.dec_ref(); }
        }
    }
}
