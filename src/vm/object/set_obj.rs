use std::collections::BTreeSet;
use std::ops::{Deref, DerefMut};
use crate::vm::value::Value;

// Set object representation.
#[derive(Debug, Clone)]
pub struct SetObj {
    pub elements: BTreeSet<Value>,
    pub cache: Option<Vec<Value>>,
    pub cached_arr: Option<Value>,
}

impl PartialEq for SetObj {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}
impl SetObj {
    pub fn new(elements: BTreeSet<Value>) -> Self {
        Self { elements, cache: None, cached_arr: None }
    }
}

impl Deref for SetObj {
    type Target = BTreeSet<Value>;
    fn deref(&self) -> &Self::Target { &self.elements }
}

impl DerefMut for SetObj {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.elements }
}

impl Drop for SetObj {
    fn drop(&mut self) {
        for val in self.elements.iter() {
            unsafe { val.dec_ref(); }
        }
        if let Some(arr) = self.cached_arr {
            unsafe { arr.dec_ref(); }
        }
    }
}
