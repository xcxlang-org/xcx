use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;
use crate::vm::object::{SetObj, ArrayObj};
impl Executor {
    pub fn handle_set_method(
        &mut self,
        dst: u8,
        set_rc: Arc<RwLock<SetObj>>,
        kind: MethodKind,
        args: &[Value],
        _names: Option<&[String]>,
        _ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Has | MethodKind::Contains => {
                let set_data = set_rc.read();
                let res = Value::from_bool((*set_data).contains(&args[0]));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                return OpResult::Continue;
            }
            MethodKind::Len | MethodKind::Count | MethodKind::Size => {
                let set_data = set_rc.read();
                let res = Value::from_i64((*set_data).len() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                return OpResult::Continue;
            }
            MethodKind::IsEmpty => {
                let set_data = set_rc.read();
                let res = Value::from_bool((*set_data).is_empty());
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                return OpResult::Continue;
            }
            MethodKind::Values => {
                let cached = set_rc.read().cached_arr;
                if let Some(arr_val) = cached {
                    let dst_val = locals[dst as usize];
                    if dst_val.bits != arr_val.bits || dst_val.tag != arr_val.tag {
                        unsafe { arr_val.inc_ref(); }
                        unsafe { dst_val.dec_ref(); }
                        locals[dst as usize] = arr_val;
                    }
                    return OpResult::Continue;
                }
            }
            _ => {}
        }

        let mut set_data = set_rc.write();
        match kind {
            MethodKind::Add => { 
                let val = args[0];
                unsafe { val.inc_ref(); }
                if (*set_data).insert(val) {
                    set_data.cache = None;
                    if let Some(arr) = set_data.cached_arr.take() {
                        unsafe { arr.dec_ref(); }
                    }
                }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Remove   => { 
                let res_bool = (*set_data).remove(&args[0]);
                if res_bool {
                    set_data.cache = None;
                    if let Some(arr) = set_data.cached_arr.take() {
                        unsafe { arr.dec_ref(); }
                    }
                }
                let res = Value::from_bool(res_bool);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Clear => { 
                for v in (*set_data).iter() { unsafe { v.dec_ref(); } }
                (*set_data).clear();
                set_data.cache = None;
                if let Some(arr) = set_data.cached_arr.take() {
                    unsafe { arr.dec_ref(); }
                }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Values => {
                let mut arr = Vec::with_capacity((*set_data).len());
                for v in (*set_data).iter() {
                    unsafe { v.inc_ref(); }
                    arr.push(*v);
                }
                let res = Value::from_array(Arc::new(RwLock::new(ArrayObj::new(arr))));
                unsafe { res.inc_ref(); }
                set_data.cached_arr = Some(res);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Show     => { 
                drop(set_data);
                let set_val = Value::from_set(set_rc.clone());
                // println!("{}", set_val.to_string()); 
                unsafe { set_val.dec_ref(); }
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => { 
                // eprintln!("Method {:?} not supported for Set{}", kind, self.current_span_info(ip)); 
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt; 
            }
        }
        OpResult::Continue
    }
}
