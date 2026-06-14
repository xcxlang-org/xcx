use std::sync::Arc;
use parking_lot::{RwLock};
use crate::vm::core::vm::{OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::value::{Value, TAG_ARR};

fn make_update_result(affected: i64) -> Value {
    let mut obj = Vec::new();
    obj.push((std::sync::Arc::new("affected".to_string()), crate::vm::object::JsonVal::Int(affected)));
    obj.push((std::sync::Arc::new("insertId".to_string()), crate::vm::object::JsonVal::Int(0)));
    let jv = crate::vm::object::JsonVal::Object(Arc::new(RwLock::new(obj)));
    Value::from_json(Arc::new(crate::vm::object::JsonObj::new(jv)))
}

impl Executor {
    pub fn handle_table_update<'a>(
        &mut self,
        dst: u8,
        t_rc: Arc<RwLock<crate::vm::object::TableObj>>,
        args: &[Value],
        locals: &mut [Value],
    ) -> OpResult {
        let idx = if args[0].is_int() { args[0].as_i64() } else { -1 };
        let vals = &args[1];
        if idx >= 0 {
            let mut t_mut = t_rc.write();
            if (idx as usize) < t_mut.rows.len() {
                if vals.is_ptr() && vals.tag == TAG_ARR {
                    let arr_rc = vals.as_array();
                    let arr = arr_rc.read();
                    let mut ai = 0usize;
                    for ci in 0..t_mut.columns.len() {
                        if !t_mut.columns[ci].is_auto {
                            if ai < arr.elements.len() {
                                let val = arr.elements[ai];
                                unsafe { val.inc_ref(); }
                                let old = t_mut.rows[idx as usize][ci];
                                t_mut.rows[idx as usize][ci] = val;
                                unsafe { old.dec_ref(); }
                                ai += 1;
                            }
                        }
                    }
                    let res = make_update_result(1);
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = res;
                } else { 
                    let res = make_update_result(0);
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = res;
                }
            } else { 
                let res = make_update_result(0);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
        } else { 
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(false);
        }
        OpResult::Continue
    }
}
