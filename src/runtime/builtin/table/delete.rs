use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use crate::vm::value::Value;

impl Executor {
    pub fn handle_table_delete<'a>(
        &mut self,
        dst: u8,
        t_rc: Arc<RwLock<crate::vm::object::TableObj>>,
        args: &[Value],
        locals: &mut [Value],
    ) -> OpResult {
        let idx = if args[0].is_int() { args[0].as_i64() } else { -1 };
        fn make_update_result(affected: i64) -> Value {
            let mut obj = Vec::new();
            obj.push((std::sync::Arc::new("affected".to_string()), crate::vm::object::JsonVal::Int(affected)));
            obj.push((std::sync::Arc::new("insertId".to_string()), crate::vm::object::JsonVal::Int(0)));
            let jv = crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(obj)));
            Value::from_json(Arc::new(crate::vm::object::JsonObj::new(jv)))
        }

        if idx >= 0 {
            let mut t_mut = t_rc.write();
            if (idx as usize) < t_mut.len() {
                let key = Arc::as_ptr(&t_rc) as usize;
                if let Some(cache_vec) = self.row_cache.remove(&key) {
                    for v in cache_vec {
                        if v.is_row() {
                            unsafe { v.dec_ref(); }
                        }
                    }
                }
                let cols_len = t_mut.columns.len();
                let start_idx = idx as usize * cols_len;
                let drained: Vec<Value> = t_mut.rows.drain(start_idx..start_idx + cols_len).collect();
                for v in drained { unsafe { v.dec_ref(); } }
                let res = make_update_result(1);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
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

    pub fn handle_table_clear<'a>(
        &mut self,
        dst: u8,
        t_rc: Arc<RwLock<crate::vm::object::TableObj>>,
        locals: &mut [Value],
    ) -> OpResult {
        let mut t_mut = t_rc.write();
        let key = Arc::as_ptr(&t_rc) as usize;
        if let Some(cache_vec) = self.row_cache.remove(&key) {
            for v in cache_vec {
                if v.is_row() {
                    unsafe { v.dec_ref(); }
                }
            }
        }
        for v in t_mut.rows.drain(..) { unsafe { v.dec_ref(); } }
        let res = Value::from_bool(true);
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = res;
        OpResult::Continue
    }
}
