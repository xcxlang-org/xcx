use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::core::vm::OpResult;
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;
use crate::vm::object::{TableObj, RowObj, JsonObj};
use crate::vm::utils::json::value_to_json;

impl Executor {
    pub fn handle_table_index<'a>(
        &mut self,
        dst: u8,
        t_rc: Arc<RwLock<TableObj>>,
        args: &[Value],
        _ip: usize,
        locals: &mut [Value],
        is_first: bool,
    ) -> OpResult {
        let t = t_rc.read();
        if is_first {
            let res = if !t.rows.is_empty() {
                Value::from_row(Arc::new(RowObj {
                    table: t_rc.clone(),
                    row_idx: 0,
                }))
            } else {
                Value::from_bool(false)
            };
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            return OpResult::Continue;
        }

        let idx = if args[0].is_int() { args[0].as_i64() } else { -1 };
        if idx >= 0 && (idx as usize) < t.len() {
            let res = Value::from_row(Arc::new(RowObj { table: t_rc.clone(), row_idx: idx as u32 }));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            OpResult::Continue
        } else {
            self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            OpResult::Halt
        }
    }

    pub fn handle_row_method(
        &mut self,
        dst: u8,
        r: Arc<RowObj>,
        kind: MethodKind,
        args: &[Value],
        _names: Option<&[String]>,
        _ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<crate::vm::core::vm::VM>,
    ) -> OpResult {
        match kind {
            MethodKind::Get => {
                let col_name = args[0].to_string();
                let t = r.table.read();
                if let Some(ci) = t.columns.iter().position(|c| c.name == col_name) {
                    let val = t.rows[r.row_idx as usize * t.columns.len() + ci];
                    unsafe { val.inc_ref(); }
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = val;
                } else {
                    self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    return OpResult::Halt;
                }
            }
            MethodKind::Set => {
                let col_name = args[0].to_string();
                let val = args[1];
                let mut t = r.table.write();
                if let Some(ci) = t.columns.iter().position(|c| c.name == col_name) {
                    unsafe { val.inc_ref(); }
                    let cell_idx = r.row_idx as usize * t.columns.len() + ci;
                    let old = t.rows[cell_idx];
                    t.rows[cell_idx] = val;
                    unsafe { old.dec_ref(); }
                    
                    if let Some(binding) = &t.sql_binding {
                        let pk_idx = t.columns.iter().position(|c| c.is_pk);
                        if let Some(pki) = pk_idx {
                            let pk_val = t.rows[r.row_idx as usize * t.columns.len() + pki];
                            let sql = format!("UPDATE [{}] SET [{}] = ? WHERE [{}] = ?", binding.table_name, col_name, t.columns[pki].name);
                            let conn = binding.db_conn.lock();
                            if let Ok(mut stmt) = conn.prepare(&sql) {
                                for (i, v) in [val, pk_val].iter().enumerate() {
                                    if v.is_int() { let _ = stmt.raw_bind_parameter(i + 1, v.as_i64()); }
                                    else if v.is_float() { let _ = stmt.raw_bind_parameter(i + 1, v.as_f64()); }
                                    else if v.is_bool() { let _ = stmt.raw_bind_parameter(i + 1, if v.as_bool() { 1 } else { 0 }); }
                                    else { let _ = stmt.raw_bind_parameter(i + 1, v.to_string()); }
                                }
                                let _ = stmt.raw_execute();
                            }
                        }
                    }
                    let res = Value::from_bool(true);
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = res;
                } else {
                    self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    return OpResult::Halt;
                }
            }
            MethodKind::ToJson => {
                let t = r.table.read();
                let mut obj = Vec::new();
                let cols_len = t.columns.len();
                for (i, col) in t.columns.iter().enumerate() {
                    obj.push((crate::vm::object::intern_key(col.name.clone()), value_to_json(&t.rows[r.row_idx as usize * cols_len + i])));
                }
                let res = Value::from_json(Arc::new(JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(RwLock::new(obj))))));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Show => {
                let t = r.table.read();
                let mut s = String::new();
                let cols_len = t.columns.len();
                for (i, col) in t.columns.iter().enumerate() {
                    s.push_str(&format!("{}: {}, ", col.name, t.rows[r.row_idx as usize * cols_len + i].to_string()));
                }
                println!("{}", s);
                let res = Value::from_bool(true);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => { 
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt; 
            }
        }
        OpResult::Continue
    }

    pub fn handle_row_custom(
        &mut self,
        dst: u8,
        r: Arc<RowObj>,
        col_name_bytes: &[u8],
        _ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<crate::vm::core::vm::VM>,
    ) -> OpResult {
        let col_name = String::from_utf8_lossy(col_name_bytes);
        let t = r.table.read();
        if let Some(ci) = t.columns.iter().position(|c| c.name == col_name) {
            let val = t.rows[r.row_idx as usize * t.columns.len() + ci];
            unsafe { val.inc_ref(); }
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = val;
            OpResult::Continue
        } else {
            self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            OpResult::Halt
        }
    }
}
