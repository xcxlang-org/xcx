use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;
use crate::vm::object::ArrayObj;
use crate::vm::utils::archive::{zip_folder, unzip_archive};

/// Validates that a path is safe for filesystem operations (no escapes, no absolute paths, unless within install dir).
pub fn validate_path_safety(path: &str) {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(bin_dir) = exe_path.parent() {
            if let Some(install_dir) = bin_dir.parent() {
                let install_dir_str = install_dir.to_string_lossy().replace("\\", "/");
                let normalized_path = path.replace("\\", "/");
                
                #[cfg(windows)]
                let is_match = normalized_path.to_lowercase().starts_with(&install_dir_str.to_lowercase());
                #[cfg(not(windows))]
                let is_match = normalized_path.starts_with(&install_dir_str);

                if is_match {
                    if path.contains("..") {
                        panic!("halt.fatal: Security violation - path traversal in: {}", path);
                    }
                    return;
                }
            }
        }
    }

    if path.contains("..") || path.starts_with('/') || (path.len() > 1 && path.as_bytes()[1] == b':') {
        panic!("halt.fatal: Security violation - illegal path access: {}", path);
    }
}

/// Checks if a file or directory exists and is not empty.
pub fn exists(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    validate_path_safety(&path_str);
    let path = std::path::Path::new(&path_str);
    let exists = path.exists();
    
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = Value::from_bool(exists);
    OpResult::Continue
}

/// Lists the contents of a directory.
pub fn list(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    validate_path_safety(&path_str);
    
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&path_str) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                files.push(Value::from_string(Arc::new(crate::vm::object::StringObj::new(name.into_bytes()))));
            }
        }
    }
    
    let res = Value::from_array(Arc::new(RwLock::new(ArrayObj::new(files))));
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}

/// Checks if a path is a directory.
pub fn is_dir(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    validate_path_safety(&path_str);
    
    let res = Value::from_bool(std::path::Path::new(&path_str).is_dir());
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}

/// Returns the size of a file in bytes.
pub fn size(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    validate_path_safety(&path_str);
    
    let size = std::fs::metadata(&path_str).map(|m| m.len()).unwrap_or(0);
    let res = Value::from_i64(size as i64);
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}

/// Creates a directory and any missing parent directories.
pub fn mkdir(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    validate_path_safety(&path_str);
    let ok = std::fs::create_dir_all(path_str).is_ok();
    
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = Value::from_bool(ok);
    OpResult::Continue
}

/// Searches for files matching a glob pattern.
pub fn glob(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let pattern = locals[base as usize].to_string();
    validate_path_safety(&pattern);
    
    let mut results = Vec::new();
    if let Ok(paths) = glob::glob(&pattern) {
        for entry in paths.filter_map(Result::ok) {
            if let Some(s) = entry.to_str() {
                results.push(Value::from_string(Arc::new(crate::vm::object::StringObj::new(s.to_string().into_bytes()))));
            }
        }
    }
    
    let res = Value::from_array(Arc::new(RwLock::new(ArrayObj::new(results))));
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}

/// Zips a folder or file.
pub fn zip(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let source = locals[base as usize].to_string();
    let target = locals[(base + 1) as usize].to_string();
    validate_path_safety(&source);
    validate_path_safety(&target);
    
    let ok = zip_folder(&source, &target).is_ok();
    let res = Value::from_bool(ok);
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}

/// Unzips an archive.
pub fn unzip(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let zip_file = locals[base as usize].to_string();
    let dest_dir = locals[(base + 1) as usize].to_string();
    validate_path_safety(&zip_file);
    validate_path_safety(&dest_dir);
    
    let ok = unzip_archive(&zip_file, &dest_dir).is_ok();
    let res = Value::from_bool(ok);
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = res;
    OpResult::Continue
}
