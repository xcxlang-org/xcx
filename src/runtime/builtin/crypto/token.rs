use std::sync::Arc;
use crate::vm::value::Value;

/// Generates a random hexadecimal token of the specified length.
pub fn token_impl(len: usize) -> Value {
    let mut rng = rand::rng();
    let token: String = (0..len * 2).map(|_| {
        const CHARSET: &[u8] = b"0123456789abcdef";
        use rand::Rng;
        let idx = rng.random_range(0..CHARSET.len());
        CHARSET[idx] as char
    }).collect();
    
    Value::from_string(Arc::new(crate::vm::object::StringObj::new(token.into_bytes())))
}
