use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::{Value, TAG_STR};
use crate::vm::object::{ArrayObj, JsonObj, StringObj};
use crate::vm::utils::{get_path_value_xcx, set_path_value_xcx};
use crate::vm::utils::json::value_to_json;

impl Executor {
    pub fn handle_array_method(
        &mut self,
        dst: u8,
        arr_rc: Arc<RwLock<ArrayObj>>,
        kind: MethodKind,
        args: &[Value],
        _names: Option<&[String]>,
        ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Push => { 
                let val = args[0];
                unsafe { val.inc_ref(); }
                arr_rc.write().elements.push(val); 
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Pop  => { 
                let res = arr_rc.write().elements.pop().unwrap_or(Value::from_bool(false)); 
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Len | MethodKind::Count | MethodKind::Size => {
                let res = Value::from_i64(arr_rc.read().elements.len() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Clear => { 
                let mut arr = arr_rc.write();
                for v in arr.elements.drain(..) { unsafe { v.dec_ref(); } }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Contains => {
                let res = Value::from_bool(arr_rc.read().elements.contains(&args[0]));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::IsEmpty  => {
                let res = Value::from_bool(arr_rc.read().elements.is_empty());
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Get => {
                if args[0].is_int() {
                    let arr = arr_rc.read();
                    let i = args[0].as_i64();
                    if i >= 0 && (i as usize) < arr.elements.len() {
                        let v = arr.elements[i as usize];
                        unsafe { v.inc_ref(); }
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = v;
                    } else {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!("R303: Array index out of bounds: {} (Array length: {}){}", i, arr.elements.len(), self.current_span_info(ip));
                        return OpResult::Halt;
                    }
                } else if args[0].is_string() {
                    let path = args[0].to_string();
                    if path.starts_with('/') {
                        let v = get_path_value_xcx(Value::from_array(arr_rc.clone()), &path).unwrap_or(Value::from_bool(false));
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = v;
                    } else {
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = Value::from_bool(false);
                    }
                } else { 
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = Value::from_bool(false);
                }
            }
            MethodKind::Insert => {
                if args[0].is_int() {
                    let i = args[0].as_i64();
                    let val = args[1];
                    let mut arr = arr_rc.write();
                    if i >= 0 && (i as usize) <= arr.elements.len() {
                        unsafe { val.inc_ref(); }
                        arr.elements.insert(i as usize, val);
                        let res = Value::from_bool(true);
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                    } else {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!("R303: Array insert index out of bounds: {}{}", i, self.current_span_info(ip));
                        return OpResult::Halt;
                    }
                } else { 
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = Value::from_bool(false);
                }
            }
            MethodKind::Update | MethodKind::Set => {
                if args[0].is_int() {
                    let i = args[0].as_i64();
                    let val = args[1];
                    let mut arr = arr_rc.write();
                    if i >= 0 && (i as usize) < arr.elements.len() {
                        unsafe { val.inc_ref(); }
                        let old = arr.elements[i as usize];
                        arr.elements[i as usize] = val;
                        unsafe { old.dec_ref(); }
                        let res = Value::from_bool(true);
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                    } else {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!("R303: Array update index out of bounds: {}{}", i, self.current_span_info(ip));
                        return OpResult::Halt;
                    }
                } else if args[0].is_string() {
                    let path = args[0].to_string();
                    if path.starts_with('/') {
                        let val = args[1];
                        set_path_value_xcx(Value::from_array(arr_rc.clone()), &path, val);
                        let res = Value::from_bool(true);
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                    } else {
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = Value::from_bool(false);
                    }
                } else { 
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = Value::from_bool(false);
                }
            }
            MethodKind::Delete => {
                if args[0].is_int() {
                    let i = args[0].as_i64();
                    let mut arr = arr_rc.write();
                    if i >= 0 && (i as usize) < arr.elements.len() {
                        let old = arr.elements.remove(i as usize);
                        unsafe { old.dec_ref(); }
                        let res = Value::from_bool(true);
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                    } else {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        eprintln!("R303: Array delete index out of bounds: {}{}", i, self.current_span_info(ip));
                        return OpResult::Halt;
                    }
                } else { 
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = Value::from_bool(false);
                }
            }
            MethodKind::Find => {
                let needle = &args[0];
                let idx = arr_rc.read().elements.iter().position(|v| v == needle).map(|i| i as i64).unwrap_or(-1);
                let res = Value::from_i64(idx);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Join => {
                let sep = if args[0].is_ptr() && args[0].tag == TAG_STR {
                    let s = args[0].as_string();
                    s.data.clone()
                } else { b"".to_vec() };
                let res_bytes = arr_rc.read().elements.iter()
                    .map(|v| {
                        if v.is_string() { v.as_string().data.clone() }
                        else { v.to_string().into_bytes() }
                    })
                    .collect::<Vec<_>>()
                    .join(sep.as_slice());
                let res = Value::from_string(Arc::new(StringObj::new(res_bytes)));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Show => { 
                let arr_val = Value::from_array(arr_rc.clone());
                println!("{}", arr_val.to_string()); 
                unsafe { arr_val.dec_ref(); }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Sort | MethodKind::Reverse => {
                return self.handle_array_sort(dst, arr_rc.clone(), kind, locals);
            }
            MethodKind::ToStr => {
                let arr_val = Value::from_array(arr_rc.clone());
                let s = arr_val.to_string();
                unsafe { arr_val.dec_ref(); }
                let res = Value::from_string(Arc::new(StringObj::new(s.into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::ToJson => {
                let arr_val = Value::from_array(arr_rc.clone());
                let json = value_to_json(&arr_val);
                unsafe { arr_val.dec_ref(); }
                let res = Value::from_json(Arc::new(JsonObj::new(json)));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => { 
                eprintln!("Method {:?} not supported for Array{}", kind, self.current_span_info(ip)); 
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt; 
            }
        }
        OpResult::Continue
    }
}
