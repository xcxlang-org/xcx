use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;

/// Reads the entire contents of a file into a string value.
pub fn read(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    super::fs_ops::validate_path_safety(&path_str);
    
    match std::fs::read(&path_str) {
        Ok(b) => {
            let res = Value::from_string(Arc::new(crate::vm::object::StringObj::new(b)));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            OpResult::Continue
        }
        Err(e) => {
            eprintln!("HALT.FATAL: store.read failed for '{}': {}", path_str, e);
            OpResult::Halt
        }
    }
}

/// Writes a string value to a file.
pub fn write(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    super::fs_ops::validate_path_safety(&path_str);
    let path = std::path::Path::new(&path_str);
    let content_val = locals[(base + 1) as usize];
    let content = content_val.as_string();
    
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut res = std::fs::write(path, &*content).is_ok();
    
    #[cfg(windows)]
    if !res {
        if let Some(ext) = path.extension() {
            if ext.to_string_lossy().to_lowercase() == "exe" {
                let old_path = path.with_extension("exe.old");
                let _ = std::fs::remove_file(&old_path);
                if std::fs::rename(path, &old_path).is_ok() {
                    res = std::fs::write(path, &*content).is_ok();
                }
            }
        }
    }
    
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = Value::from_bool(res);
    OpResult::Continue
}

/// Appends a string value to a file.
pub fn append(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    super::fs_ops::validate_path_safety(&path_str);
    let path = std::path::Path::new(&path_str);
    let content_val = locals[(base + 1) as usize];
    let content = content_val.as_string();
    
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let ok = if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(path) {
        f.write_all(&content.data).is_ok()
    } else {
        false
    };

    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = Value::from_bool(ok);
    OpResult::Continue
}

/// Deletes a file or directory.
pub fn delete(dst: u8, base: u8, locals: &mut [Value]) -> OpResult {
    let path_str = locals[base as usize].to_string();
    super::fs_ops::validate_path_safety(&path_str);
    let path = std::path::Path::new(&path_str);
    let ok = if path.is_dir() {
        std::fs::remove_dir_all(path).is_ok()
    } else {
        std::fs::remove_file(path).is_ok()
    };
    
    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = Value::from_bool(ok);
    OpResult::Continue
}
