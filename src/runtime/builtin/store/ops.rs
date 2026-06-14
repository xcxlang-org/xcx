use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;

/// Proxy for read_write::read
pub fn read(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::read_write::read(dst, base, locals)
}

/// Proxy for read_write::write
pub fn write(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::read_write::write(dst, base, locals)
}

/// Proxy for read_write::append
pub fn append(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::read_write::append(dst, base, locals)
}

/// Proxy for fs_ops::exists
pub fn exists(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::fs_ops::exists(dst, base, locals)
}

/// Proxy for read_write::delete
pub fn delete(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::read_write::delete(dst, base, locals)
}

/// Proxy for fs_ops::list
pub fn list(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::fs_ops::list(dst, base, locals)
}

/// Proxy for fs_ops::is_dir
pub fn is_dir(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::fs_ops::is_dir(dst, base, locals)
}

/// Proxy for fs_ops::size
pub fn size(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::fs_ops::size(dst, base, locals)
}

/// Proxy for fs_ops::mkdir
pub fn mkdir(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::fs_ops::mkdir(dst, base, locals)
}

/// Proxy for fs_ops::glob
pub fn glob(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::fs_ops::glob(dst, base, locals)
}

/// Proxy for fs_ops::zip
pub fn zip(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::fs_ops::zip(dst, base, locals)
}

/// Proxy for fs_ops::unzip
pub fn unzip(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    super::fs_ops::unzip(dst, base, locals)
}

pub fn validate_path_safety(path: &str) {
    super::fs_ops::validate_path_safety(path)
}
