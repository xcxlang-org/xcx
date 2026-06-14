use std::sync::Arc;
use crate::vm::value::Value;

/// Hashes the provided string using the specified algorithm.
/// Supported algorithms: argon2 (default), bcrypt, base64_encode, base64_decode.
pub fn hash_impl(data: Vec<u8>, algo: String) -> Value {
    let bytes = &data;
    
    match algo.as_str() {
        "bcrypt" => {
            let s = String::from_utf8_lossy(bytes).into_owned();
            bcrypt::hash(&*s, bcrypt::DEFAULT_COST)
                .map(|h| Value::from_string(Arc::new(crate::vm::object::StringObj::new(h.into_bytes()))))
                .unwrap_or(Value::from_bool(false))
        }
        "argon2" => {
            use argon2::PasswordHasher;
            let mut salt_bytes = [0u8; 16];
            rand::fill(&mut salt_bytes);
            let salt = argon2::password_hash::SaltString::encode_b64(&salt_bytes).unwrap();
            let argon2_inst = argon2::Argon2::default();
            argon2_inst.hash_password(bytes, &salt)
                .map(|h| Value::from_string(Arc::new(crate::vm::object::StringObj::new(h.to_string().into_bytes()))))
                .unwrap_or(Value::from_bool(false))
        }
        "base64_encode" => {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&*bytes);
            Value::from_string(Arc::new(crate::vm::object::StringObj::new(encoded.into_bytes())))
        }
        "base64_decode" => {
            use base64::Engine;
            let s = String::from_utf8_lossy(&bytes);
            match base64::engine::general_purpose::STANDARD.decode(s.as_ref()) {
                Ok(decoded) => Value::from_string(Arc::new(crate::vm::object::StringObj::new(decoded))),
                Err(_) => Value::from_bool(false),
            }
        }
        _ => Value::from_bool(false),
    }
}

/// Verifies a password against a hash using the specified algorithm.
pub fn verify_impl(password: String, hashed: String, algo: String) -> bool {
    match algo.as_str() {
        "bcrypt" => bcrypt::verify(&*password, &*hashed).unwrap_or(false),
        "argon2" => {
            use argon2::PasswordVerifier;
            if let Ok(parsed_hash) = argon2::PasswordHash::new(&hashed) {
                let verify_res = argon2::Argon2::default().verify_password(password.as_bytes(), &parsed_hash);
                verify_res.is_ok()
            } else { 
                false 
            }
        }
        _ => false,
    }
}
