pub mod access;
pub mod inject;
pub mod parse;

use std::sync::Arc;

use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::{Value};
use parking_lot::RwLock;
use crate::vm::object::{JsonObj, StringObj, ArrayObj};
use crate::vm::utils::json::value_to_json;

impl Executor {
    pub fn handle_json_method(
        &mut self,
        dst: u8,
        json_rc: Arc<JsonObj>,
        kind: MethodKind,
        args: &[Value],
        _names: Option<&[String]>,
        ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Get => {
                if args[0].is_int() {
                    if let crate::vm::object::JsonVal::Array(a) = &json_rc.root {
                        let idx = args[0].as_i64();
                        if idx >= 0 {
                            let idx_u = idx as usize;
                            let a_read = unsafe { &*(*a).data_ptr() };
                            if idx_u < a_read.len() {
                                let val = crate::vm::utils::json_val_to_value(&a_read[idx_u]);
                                unsafe { val.inc_ref(); }
                                unsafe { locals[dst as usize].dec_ref(); }
                                locals[dst as usize] = val;
                            } else {
                                unsafe { locals[dst as usize].dec_ref(); }
                                locals[dst as usize] = Value::from_bool(false);
                            }
                        } else {
                            unsafe { locals[dst as usize].dec_ref(); }
                            locals[dst as usize] = Value::from_bool(false);
                        }
                        return OpResult::Continue;
                    }
                }
                let path_borrow = unsafe { args[0].as_str_borrow() };
                let path_temp;
                let path = match path_borrow {
                    Some(s) => s,
                    None => {
                        path_temp = args[0].to_string();
                        &path_temp
                    }
                };
                let is_simple = !path.is_empty() && path.bytes().all(|b| b != b'.' && b != b'[' && b != b']' && b != b'/');
                if is_simple {
                    match &json_rc.root {
                        crate::vm::object::JsonVal::Object(o) => {
                            let o_read = unsafe { &*(*o).data_ptr() };
                            if let Some((_, v)) = o_read.iter().find(|(k, _)| k.as_str() == path) {
                                let val = crate::vm::utils::json_val_to_value(v);
                                unsafe { val.inc_ref(); }
                                unsafe { locals[dst as usize].dec_ref(); }
                                locals[dst as usize] = val;
                            } else {
                                unsafe { locals[dst as usize].dec_ref(); }
                                locals[dst as usize] = Value::from_bool(false);
                            }
                        }
                        crate::vm::object::JsonVal::Array(a) => {
                            if let Ok(idx) = path.parse::<usize>() {
                                let a_read = unsafe { &*(*a).data_ptr() };
                                if idx < a_read.len() {
                                    let val = crate::vm::utils::json_val_to_value(&a_read[idx]);
                                    unsafe { val.inc_ref(); }
                                    unsafe { locals[dst as usize].dec_ref(); }
                                    locals[dst as usize] = val;
                                } else {
                                    unsafe { locals[dst as usize].dec_ref(); }
                                    locals[dst as usize] = Value::from_bool(false);
                                }
                            } else {
                                unsafe { locals[dst as usize].dec_ref(); }
                                locals[dst as usize] = Value::from_bool(false);
                            }
                        }
                        _ => {
                            unsafe { locals[dst as usize].dec_ref(); }
                            locals[dst as usize] = Value::from_bool(false);
                        }
                    }
                    return OpResult::Continue;
                }
                let pointer = crate::runtime::builtin::json::access::normalize_json_path(path);
                if let Some(v) = json_rc.root.pointer(&pointer) {
                    let val = crate::vm::utils::json_val_to_value(&v);
                    unsafe { val.inc_ref(); }
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = val;
                } else {
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = Value::from_bool(false);
                }
            }
            MethodKind::Set => {
                let path_borrow = unsafe { args[0].as_str_borrow() };
                let path_temp;
                let path = match path_borrow {
                    Some(s) => s,
                    None => {
                        path_temp = args[0].to_string();
                        &path_temp
                    }
                };
                let val = args[1];
                let is_simple = !path.starts_with('/') && !path.contains('.') && !path.contains('[') && !path.contains(']');
                let json_ptr = Arc::as_ptr(&json_rc) as *mut JsonObj;
                unsafe {
                    (*json_ptr).version.fetch_add(1, std::sync::atomic::Ordering::Release);
                    (*json_ptr).root.make_mutable();
                    if is_simple {
                        if let crate::vm::object::JsonVal::Object(o) = &mut (*json_ptr).root {
                            let mut obj = o.write();
                            if let Some(pos) = obj.iter().position(|(k, _)| k.as_str() == path) {
                                obj[pos].1 = value_to_json(&val);
                            } else {
                                obj.push((std::sync::Arc::new(path.to_string()), value_to_json(&val)));
                            }
                        }
                    } else {
                        let mut root_copy = (*json_ptr).root.clone();
                        crate::vm::utils::set_json_value_at_path(&mut root_copy, path, value_to_json(&val));
                        (*json_ptr).root = root_copy;
                    }
                }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::ToStr => {
                let ver = json_rc.version.load(std::sync::atomic::Ordering::Acquire);
                let cached_ver = json_rc.cached_version.load(std::sync::atomic::Ordering::Acquire);
                if ver == cached_ver {
                    if let Some(s) = json_rc.cached_str.lock().as_ref() {
                        let res = Value::from_string(s.clone());
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                        return OpResult::Continue;
                    }
                }
                
                let capacity = match &json_rc.root {
                    crate::vm::object::JsonVal::Array(a) => a.read().len() * 64,
                    crate::vm::object::JsonVal::Object(o) => o.read().len() * 64,
                    _ => 1024,
                };
                let mut buf = String::with_capacity(capacity.max(4096));
                json_rc.root.to_string_buf(&mut buf);
                let string_obj = Arc::new(StringObj::new(buf.into_bytes()));
                
                let mut lock = json_rc.cached_str.lock();
                if json_rc.version.load(std::sync::atomic::Ordering::Acquire) == ver {
                    *lock = Some(string_obj.clone());
                    json_rc.cached_version.store(ver, std::sync::atomic::Ordering::Release);
                }
                let res = Value::from_string(string_obj);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Show => {
                let ver = json_rc.version.load(std::sync::atomic::Ordering::Acquire);
                let cached_ver = json_rc.cached_version.load(std::sync::atomic::Ordering::Acquire);
                if ver == cached_ver {
                    let cached_opt = json_rc.cached_str.lock().clone();
                    if let Some(s_obj) = cached_opt {
                        println!("{}", String::from_utf8_lossy(&s_obj.data));
                        let res = Value::from_bool(true);
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                        return OpResult::Continue;
                    }
                }
                
                let capacity = match &json_rc.root {
                    crate::vm::object::JsonVal::Array(a) => a.read().len() * 64,
                    crate::vm::object::JsonVal::Object(o) => o.read().len() * 64,
                    _ => 1024,
                };
                let mut buf = String::with_capacity(capacity.max(4096));
                json_rc.root.to_string_buf(&mut buf);
                println!("{}", buf);
                let string_obj = Arc::new(StringObj::new(buf.into_bytes()));
                
                let mut lock = json_rc.cached_str.lock();
                if json_rc.version.load(std::sync::atomic::Ordering::Acquire) == ver {
                    *lock = Some(string_obj);
                    json_rc.cached_version.store(ver, std::sync::atomic::Ordering::Release);
                }
                
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Inject => {
                let ok = if args.len() == 2 {
                    if args[0].is_map() && args[1].is_table() {
                        self.native_inject_table(&args[1], Value::from_json(json_rc.clone()), &args[0]);
                        true
                    } else { false }
                } else if args.len() == 3 {
                    if args[0].is_string() && args[1].is_map() && args[2].is_table() {
                        let path = args[0].to_string();
                        let ok = if let Some(sub_val) = crate::runtime::builtin::json::access::get_path_value_xcx(Value::from_json(json_rc.clone()), &path) {
                            self.native_inject_table(&args[2], sub_val, &args[1]);
                            true
                        } else {
                            false
                        };
                        ok
                    } else { false }
                } else { false };
                let res = Value::from_bool(ok);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Push => {
                let val = args[0];
                let json_ptr = Arc::as_ptr(&json_rc) as *mut JsonObj;
                unsafe {
                    (*json_ptr).root.make_mutable();
                    if let crate::vm::object::JsonVal::Array(a) = &mut (*json_ptr).root {
                        let mut arr = a.write();
                        arr.push(value_to_json(&val));
                        let res = Value::from_bool(true);
                        locals[dst as usize].dec_ref();
                        locals[dst as usize] = res;
                    } else {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        return OpResult::Halt;
                    }
                }
            }

            MethodKind::First => {
                if let crate::vm::object::JsonVal::Array(a) = &json_rc.root {
                    let arr = a.read();
                    if arr.is_empty() {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        crate::runtime::builtin::io::eprint_buffered(&format!("HALT.ERROR: JSON array is empty on .first(){}\n", self.current_span_info(ip)));
                        return OpResult::Halt;
                    } else {
                        let res = crate::vm::utils::json::json_val_to_value(&arr[0]);
                        unsafe { res.inc_ref(); }
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                    }
                } else {
                    self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    crate::runtime::builtin::io::eprint_buffered(&format!("HALT.ERROR: .first() called on non-array JSON{}\n", self.current_span_info(ip)));
                    return OpResult::Halt;
                }
            }
            MethodKind::Exists | MethodKind::Has | MethodKind::Contains => {
                let path_borrow = unsafe { args[0].as_str_borrow() };
                let path_temp;
                let path = match path_borrow {
                    Some(s) => s,
                    None => {
                        path_temp = args[0].to_string();
                        &path_temp
                    }
                };
                let is_simple = !path.is_empty() && path.bytes().all(|b| b != b'.' && b != b'[' && b != b']' && b != b'/');
                let exists = if is_simple {
                    match &json_rc.root {
                        crate::vm::object::JsonVal::Object(o) => {
                            let o_read = unsafe { &*(*o).data_ptr() };
                            o_read.iter().any(|(k, _)| k.as_str() == path)
                        }
                        crate::vm::object::JsonVal::Array(a) => {
                            if let Ok(idx) = path.parse::<usize>() {
                                idx < unsafe { &*(*a).data_ptr() }.len()
                            } else {
                                false
                            }
                        }
                        _ => false,
                    }
                } else {
                    let pointer = crate::runtime::builtin::json::access::normalize_json_path(path);
                    json_rc.root.pointer(&pointer).is_some()
                };
                let res = Value::from_bool(exists);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Keys => {
                let keys: Vec<Value> = if let crate::vm::object::JsonVal::Object(obj) = &json_rc.root {
                    let obj_read = unsafe { &*(*obj).data_ptr() };
                    obj_read.iter().map(|(k, _)| Value::from_string(Arc::new(StringObj::new(k.as_bytes().to_vec())))).collect()
                } else {
                    vec![]
                };
                let res = Value::from_array(Arc::new(RwLock::new(ArrayObj::new(keys))));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Len | MethodKind::Count | MethodKind::Size => {
                let len = match &json_rc.root {
                    crate::vm::object::JsonVal::Array(a) => unsafe { &*(*a).data_ptr() }.len(),
                    crate::vm::object::JsonVal::Object(o) => unsafe { &*(*o).data_ptr() }.len(),
                    _ => 0,
                };
                let res = Value::from_i64(len as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => {
                eprintln!("Method {:?} not supported for JSON{}", kind, self.current_span_info(ip));
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt;
            }
        }
        OpResult::Continue
    }

    pub fn handle_json_custom(
        &mut self,
        dst: u8,
        json_rc: Arc<JsonObj>,
        method_name_bytes: &[u8],
        args: &[Value],
        _ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        let path_borrow = std::str::from_utf8(method_name_bytes).ok();
        let path_temp;
        let path = match path_borrow {
            Some(s) => s,
            None => {
                path_temp = String::from_utf8_lossy(method_name_bytes).into_owned();
                &path_temp
            }
        };
        if args.is_empty() {
            let is_simple = !path.contains('.') && !path.contains('[') && !path.contains(']') && !path.contains('/') && !path.is_empty();
            let mut found_val = None;
            if is_simple {
                match &json_rc.root {
                    crate::vm::object::JsonVal::Object(o) => {
                        let o_read = unsafe { &*(*o).data_ptr() };
                        if let Some((_, v)) = o_read.iter().find(|(k, _)| k.as_str() == path) {
                            found_val = Some(v.clone());
                        }
                    }
                    crate::vm::object::JsonVal::Array(a) => {
                        if let Ok(idx) = path.parse::<usize>() {
                            let a_read = unsafe { &*(*a).data_ptr() };
                            if idx < a_read.len() {
                                found_val = Some(a_read[idx].clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            if let Some(ref v) = found_val {
                let val = crate::vm::utils::json_val_to_value(v);
                unsafe { val.inc_ref(); }
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = val;
            } else if is_simple {
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = Value::from_bool(false);
            } else {
                let pointer = format!("/{}", path);
                if let Some(v) = json_rc.root.pointer(&pointer) {
                    let val = crate::vm::utils::json_val_to_value(&v);
                    unsafe { val.inc_ref(); }
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = val;
                } else {
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = Value::from_bool(false);
                }
            }
        } else {
            let val = args[0];
            let json_ptr = Arc::as_ptr(&json_rc) as *mut JsonObj;
            unsafe {
                let mut root_copy = (*json_ptr).root.clone();
                crate::vm::utils::set_json_value_at_path(&mut root_copy, path, value_to_json(&val));
                (*json_ptr).root = root_copy;
            }
            let res = Value::from_bool(true);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpResult::Continue
    }
}
