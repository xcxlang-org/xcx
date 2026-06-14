pub mod hash;
pub mod token;

use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;

/// Hashes the provided string using the specified algorithm.
pub fn hash(dst: u8, pass_src: u8, alg_src: u8, locals: &mut [Value]) -> OpResult {
    let data = locals[pass_src as usize].as_string().data.clone();
    let algo = locals[alg_src as usize].to_string();
    let res = hash::hash_impl(data, algo);

    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}

/// Verifies a password against a hash using the specified algorithm.
pub fn verify(dst: u8, pass_src: u8, hash_src: u8, alg_src: u8, locals: &mut [Value]) -> OpResult {
    let password = locals[pass_src as usize].to_string();
    let hashed = locals[hash_src as usize].to_string();
    let algo = locals[alg_src as usize].to_string();
    
    let ok = hash::verify_impl(password, hashed, algo);
    
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = Value::from_bool(ok);
    OpResult::Continue
}

/// Generates a random hexadecimal token of the specified length.
pub fn token(dst: u8, len_src: u8, locals: &mut [Value]) -> OpResult {
    let len = locals[len_src as usize].as_i64() as usize;
    let res = token::token_impl(len);
    
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}
