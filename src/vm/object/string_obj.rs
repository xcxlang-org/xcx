use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Eq, Hash)]
pub struct StringObj {
    pub data: Vec<u8>,
    pub hash: Option<u64>,
}

impl PartialEq for StringObj {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl StringObj {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, hash: None }
    }
}

impl Deref for StringObj {
    type Target = [u8];
    fn deref(&self) -> &Self::Target { &self.data }
}

impl DerefMut for StringObj {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.data }
}

impl PartialEq<[u8]> for StringObj {
    fn eq(&self, other: &[u8]) -> bool { self.data == other }
}
impl AsRef<[u8]> for StringObj {
    fn as_ref(&self) -> &[u8] { &self.data }
}

impl PartialEq<&[u8]> for StringObj {
    fn eq(&self, other: &&[u8]) -> bool { self.data == *other }
}

impl<const N: usize> PartialEq<[u8; N]> for StringObj {
    fn eq(&self, other: &[u8; N]) -> bool { self.data == other }
}

impl<const N: usize> PartialEq<&[u8; N]> for StringObj {
    fn eq(&self, other: &&[u8; N]) -> bool { self.data == *other }
}
