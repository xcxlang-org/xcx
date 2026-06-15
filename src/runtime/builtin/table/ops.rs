use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::{Value, TAG_STR, TAG_MAP, TAG_TBL};
use crate::vm::object::{MapObj, ArrayObj, JsonObj, StringObj};
use crate::vm::utils::{get_path_value_xcx, set_path_value_xcx};
use crate::vm::utils::json::value_to_json;

impl Executor {
    pub fn handle_map_method(
        &mut self,
        dst: u8,
        map_rc: Arc<RwLock<MapObj>>,
        kind: MethodKind,
        args: &[Value],
        _names: Option<&[String]>,
        _ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Get => {
                let key = &args[0];
                let map = map_rc.read();
                if let Some((_, v)) = map.iter().find(|(k, _)| k == key) {
                    unsafe { v.inc_ref(); }
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = *v;
                } else {
                    if key.is_string() {
                        let path = key.to_string();
                        if path.starts_with('/') || path.contains('[') || path.contains('.') {
                            drop(map);
                            let v = get_path_value_xcx(Value::from_map(map_rc.clone()), &path).unwrap_or(Value::from_bool(false));
                            unsafe { locals[dst as usize].dec_ref(); }
                            locals[dst as usize] = v;
                        } else {
                            self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            return OpResult::Halt;
                        }
                    } else {
                        self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        return OpResult::Halt;
                    }
                }
            }
            MethodKind::Set | MethodKind::Insert => {
                let key = args[0]; 
                let val = args[1];
                if key.is_string() {
                    let path = key.to_string();
                    if path.starts_with('/') || path.contains('[') || path.contains('.') {
                        set_path_value_xcx(Value::from_map(map_rc.clone()), &path, val);
                        let res = Value::from_bool(true);
                        unsafe { locals[dst as usize].dec_ref(); }
                        locals[dst as usize] = res;
                        return OpResult::Continue;
                    }
                }
                let mut map = map_rc.write();
                unsafe { key.inc_ref(); val.inc_ref(); }
                if let Some(e) = (*map).iter_mut().find(|(k, _)| *k == key) { 
                    let old_k = e.0;
                    let old_v = e.1;
                    e.0 = key;
                    e.1 = val;
                    unsafe { old_k.dec_ref(); old_v.dec_ref(); }
                } else { map.push((key, val)); }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Exists => {
                let key = &args[0];
                let found = if key.is_string() {
                    let path = key.to_string();
                    if path.starts_with('/') {
                        let ok = if let Some(v) = get_path_value_xcx(Value::from_map(map_rc.clone()), &path) {
                            let exists = v.is_ptr() || v.is_bool() || v.is_int() || v.is_float();
                            unsafe { v.dec_ref(); }
                            exists && !v.is_bool_false()
                        } else {
                            false
                        };
                        ok
                    } else {
                        map_rc.read().iter().any(|(k, _)| k == key)
                    }
                } else {
                    map_rc.read().iter().any(|(k, _)| k == key)
                };
                let res = Value::from_bool(found);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Len | MethodKind::Count | MethodKind::Size => {
                let res = Value::from_i64(map_rc.read().len() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Keys => {
                let mut keys = Vec::new();
                for (k, _) in map_rc.read().iter() { 
                    unsafe { k.inc_ref(); }
                    keys.push(*k);
                }
                let res = Value::from_array(Arc::new(RwLock::new(ArrayObj::new(keys))));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Inject => {
                let ok = if args.len() == 2 {
                    if args[0].is_ptr() && args[0].tag == TAG_MAP &&
                       args[1].is_ptr() && args[1].tag == TAG_TBL {
                        self.native_inject_table(&args[1], Value::from_map(map_rc.clone()), &args[0]);
                        true
                    } else { false }
                } else if args.len() == 3 {
                    if args[0].is_ptr() && args[0].tag == TAG_STR &&
                       args[1].is_ptr() && args[1].tag == TAG_MAP &&
                       args[2].is_ptr() && args[2].tag == TAG_TBL {
                        let path = args[0].to_string();
                        let ok = if let Some(sub_val) = get_path_value_xcx(Value::from_map(map_rc.clone()), &path) {
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
            MethodKind::Values => {
                let mut vals = Vec::new();
                for (_, v) in (*map_rc.read()).iter() {
                    unsafe { v.inc_ref(); }
                    vals.push(*v);
                }
                let res = Value::from_array(Arc::new(RwLock::new(ArrayObj::new(vals))));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Contains => {
                let key = &args[0];
                let has = (*map_rc.read()).iter().any(|(k, _)| k == key);
                let res = Value::from_bool(has);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Remove | MethodKind::Delete => {
                let key = &args[0];
                let mut map = map_rc.write();
                let before = map.len();
                if let Some(pos) = map.iter().position(|(k, _)| k == key) {
                    let (k, v) = map.remove(pos);
                    unsafe { k.dec_ref(); v.dec_ref(); }
                }
                let res = Value::from_bool(map.len() < before);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Clear => { 
                let mut map = map_rc.write();
                for (k, v) in map.drain(..) { unsafe { k.dec_ref(); v.dec_ref(); } }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Show  => { 
                let map_val = Value::from_map(map_rc.clone());
                println!("{}", map_val.to_string()); 
                unsafe { map_val.dec_ref(); }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::ToJson => {
                let map_val = Value::from_map(map_rc.clone());
                let json = value_to_json(&map_val);
                unsafe { map_val.dec_ref(); }
                let res = Value::from_json(Arc::new(JsonObj::new(json)));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::ToStr => {
                let map_val = Value::from_map(map_rc.clone());
                let s = map_val.to_string();
                unsafe { map_val.dec_ref(); }
                let res = Value::from_string(Arc::new(StringObj::new(s.into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => { 
                // eprintln!("Method {:?} not supported for Map{}", kind, self.current_span_info(ip)); 
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt; 
            }
        }
        OpResult::Continue
    }
}
