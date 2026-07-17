use std::ops::{Deref, DerefMut};

pub struct BoolArrayObj {
    pub data: Vec<u8>,
}

impl PartialEq for BoolArrayObj {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl BoolArrayObj {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Deref for BoolArrayObj {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for BoolArrayObj {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
