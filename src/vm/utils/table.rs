use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use crate::vm::value::{Value, TAG_STR, TAG_FLOAT, TAG_INT, TAG_BOOL, TAG_DATE};
use crate::vm::object::{TableObj, RowObj, VMColumn, JoinPred};
use crate::vm::core::executor::Executor;
use crate::vm::core::vm::VM;
use crate::vm::opcode::OpCode;

#[derive(Clone, Copy)]
pub struct HashableValue(pub Value);

impl PartialEq for HashableValue {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for HashableValue {}

impl std::hash::Hash for HashableValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.tag);
        match self.0.tag {
            TAG_STR => {
                let val_str = self.0.as_string();
                std::hash::Hash::hash(&*val_str, state);
            }
            TAG_INT | TAG_BOOL | TAG_DATE | TAG_FLOAT => {
                state.write_u64(self.0.bits);
            }
            _ => {
                state.write_u64(self.0.bits);
            }
        }
    }
}

pub fn join_tables(
    left: &TableObj,
    right: &TableObj,
    pred: &JoinPred,
    right_name: &str,
    executor: &mut Executor,
    vm_arc: &Arc<VM>,
) -> TableObj {
    let right_key_name: Option<&str> = match pred {
        JoinPred::Keys(_, rk) => Some(rk.as_str()),
        JoinPred::Lambda(_) => None,
        JoinPred::Closure(_, _) => None,
    };
    let left_col_names: std::collections::HashSet<&str> =
        left.columns.iter().map(|c| c.name.as_str()).collect();
    let mut out_cols: Vec<VMColumn> = left.columns.clone();
    let mut right_col_map: Vec<Option<usize>> = Vec::new();
    for (_ci, col) in right.columns.iter().enumerate() {
        if right_key_name == Some(col.name.as_str()) {
            right_col_map.push(None);
            continue;
        }
        let out_name = if left_col_names.contains(col.name.as_str()) {
            format!("{}_{}", right_name, col.name)
        } else {
            col.name.clone()
        };
        right_col_map.push(Some(out_cols.len()));
        out_cols.push(VMColumn { name: out_name, ty: col.ty.clone(), is_auto: col.is_auto, is_pk: col.is_pk, is_unique: col.is_unique });
    }
    let left_rc  = Arc::new(RwLock::new(left.clone()));
    let right_rc = Arc::new(RwLock::new(right.clone()));
    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    match pred {
        JoinPred::Keys(lk, rk) => {
            let lc = left.columns.iter().position(|c| &c.name == lk);
            let rc = right.columns.iter().position(|c| &c.name == rk);
            if let (Some(lci), Some(rci)) = (lc, rc) {
                let mut right_hash: HashMap<HashableValue, Vec<usize>> = HashMap::with_capacity(right.rows.len());
                for ri in 0..right.rows.len() {
                    let key = HashableValue(right.rows[ri][rci]);
                    right_hash.entry(key).or_default().push(ri);
                }
                for li in 0..left.rows.len() {
                    let left_key = HashableValue(left.rows[li][lci]);
                    if let Some(right_indices) = right_hash.get(&left_key) {
                        for &ri in right_indices {
                            let mut row = Vec::with_capacity(out_cols.len());
                            for v in &left.rows[li] { unsafe { v.inc_ref(); } row.push(*v); }
                            for (r_col_idx, out_idx) in right_col_map.iter().enumerate() {
                                if let Some(_oi) = out_idx {
                                    let v = right.rows[ri][r_col_idx];
                                    unsafe { v.inc_ref(); }
                                    row.push(v);
                                }
                            }
                            out_rows.push(row);
                        }
                    }
                }
            }
        }
        JoinPred::Lambda(fid) => {
            for li in 0..left.rows.len() {
                for ri in 0..right.rows.len() {
                    let row_a = Value::from_row(Arc::new(RowObj { table: left_rc.clone(), row_idx: li as u32 }));
                    let row_b = Value::from_row(Arc::new(RowObj { table: right_rc.clone(), row_idx: ri as u32 }));
                    let m = matches!(executor.run_frame(executor.ctx.functions[*fid].clone(), &[row_a, row_b], vm_arc), Some(res) if res.is_bool() && res.as_bool());
                    unsafe { row_a.dec_ref(); row_b.dec_ref(); }
                    if m {
                        let mut row = Vec::with_capacity(out_cols.len());
                        for v in &left.rows[li] { unsafe { v.inc_ref(); } row.push(*v); }
                        for (rci, out_idx) in right_col_map.iter().enumerate() {
                            if let Some(_oi) = out_idx {
                                let v = right.rows[ri][rci];
                                unsafe { v.inc_ref(); }
                                row.push(v);
                            }
                        }
                        out_rows.push(row);
                    }
                }
            }
        }
        JoinPred::Closure(fid, captures) => {
            for li in 0..left.rows.len() {
                for ri in 0..right.rows.len() {
                    let row_a = Value::from_row(Arc::new(RowObj { table: left_rc.clone(), row_idx: li as u32 }));
                    let row_b = Value::from_row(Arc::new(RowObj { table: right_rc.clone(), row_idx: ri as u32 }));
                    let mut run_args = vec![row_a, row_b];
                    for v in captures { unsafe { v.inc_ref(); } run_args.push(*v); }
                    let m = matches!(executor.run_frame(executor.ctx.functions[*fid].clone(), &run_args, vm_arc), Some(res) if res.is_bool() && res.as_bool());
                    for v in run_args { unsafe { v.dec_ref(); } }
                    if m {
                        let mut row = Vec::with_capacity(out_cols.len());
                        for v in &left.rows[li] { unsafe { v.inc_ref(); } row.push(*v); }
                        for (rci, out_idx) in right_col_map.iter().enumerate() {
                            if let Some(_oi) = out_idx {
                                let v = right.rows[ri][rci];
                                unsafe { v.inc_ref(); }
                                row.push(v);
                            }
                        }
                        out_rows.push(row);
                    }
                }
            }
        }
    }
    TableObj { table_name: String::new(), columns: out_cols, rows: out_rows, sql_binding: None, sql_where: None, pending_op: None }
}

pub fn translate_filter_to_sql(executor: &Executor, func_idx: usize, cols: &[VMColumn], captures: &[Value]) -> Option<String> {
    let chunk = &executor.ctx.functions[func_idx];
    if chunk.bytecode.len() > 40 { return None; } 
    
    let mut reg_values: HashMap<u8, (Option<String>, Option<Value>)> = HashMap::new();
    let row_reg = 0;

    for (i, v) in captures.iter().enumerate() {
        reg_values.insert((i + 1) as u8, (None, Some(*v)));
    }

    let mut final_col = None;
    let mut final_op = None;
    let mut final_val = None;

    for instr in &*chunk.bytecode {
        match *instr {
            OpCode::Move { dst, src } => {
                if let Some(val) = reg_values.get(&src).cloned() {
                    reg_values.insert(dst, val);
                }
            }
            OpCode::LoadConst { dst, idx } => {
                let v = executor.ctx.constants[idx as usize];
                reg_values.insert(dst, (None, Some(v)));
            }
            OpCode::MethodCallCustom { dst, method_name_idx, base, arg_count, .. } if arg_count == 0 => {
                if base == row_reg {
                    let name_val = executor.ctx.constants[method_name_idx as usize];
                    let name = String::from_utf8_lossy(&name_val.as_string()).into_owned();
                    if cols.iter().any(|c| c.name == name) {
                        reg_values.insert(dst, (Some(name), None));
                    }
                }
            }
            OpCode::Equal { src1, src2, .. } |
            OpCode::NotEqual { src1, src2, .. } |
            OpCode::Greater { src1, src2, .. } |
            OpCode::Less { src1, src2, .. } |
            OpCode::GreaterEqual { src1, src2, .. } |
            OpCode::LessEqual { src1, src2, .. } => {
                let v1 = reg_values.get(&src1);
                let v2 = reg_values.get(&src2);
                
                match (v1, v2) {
                    (Some((Some(col), None)), Some((None, Some(val)))) => {
                        final_col = Some(col.clone());
                        final_op = Some(match instr {
                            OpCode::Equal { .. } => "=",
                            OpCode::NotEqual { .. } => "!=",
                            OpCode::Greater { .. } => ">",
                            OpCode::Less { .. } => "<",
                            OpCode::GreaterEqual { .. } => ">=",
                            OpCode::LessEqual { .. } => "<=",
                            _ => "=",
                        });
                        final_val = Some(*val);
                    }
                    (Some((None, Some(val))), Some((Some(col), None))) => {
                        final_col = Some(col.clone());
                        final_op = Some(match instr {
                            OpCode::Equal { .. } => "=",
                            OpCode::NotEqual { .. } => "!=",
                            OpCode::Greater { .. } => "<",
                            OpCode::Less { .. } => ">",
                            OpCode::GreaterEqual { .. } => "<=",
                            OpCode::LessEqual { .. } => ">=",
                            _ => "=",
                        });
                        final_val = Some(*val);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    if let (Some(c), Some(o), Some(v)) = (final_col, final_op, final_val) {
        if v.is_int() { return Some(format!("[{}] {} {}", c, o, v.as_i64())); }
        if v.is_float() { return Some(format!("[{}] {} {}", c, o, v.as_f64())); }
        if v.is_bool() { return Some(format!("[{}] {} {}", c, o, if v.as_bool() { 1 } else { 0 })); }
        if v.is_string() { return Some(format!("[{}] {} '{}'", c, o, String::from_utf8_lossy(&v.as_string()).replace("'", "''"))); }
    }
    
    None
}

pub fn inject_json_into_table(table: &mut TableObj, json: &crate::vm::object::JsonVal, mapping: &Vec<(Value, Value)>) {
    let items = match json {
        crate::vm::object::JsonVal::Array(arr) => arr.read().clone(),
        _ => vec![json.clone()],
    };
    for item in items {
        let mut new_row = Vec::with_capacity(table.columns.len());
        for col in &table.columns {
            let mut found = false;
            for (k, v) in mapping {
                if k.is_ptr() && k.tag == TAG_STR &&
                   v.is_ptr() && v.tag == TAG_STR {
                    let col_match = k.as_string();
                    let json_path = v.as_string();
                    if &**col_match == col.name.as_bytes() {
                        let pointer = super::path::normalize_json_path(&String::from_utf8_lossy(json_path.as_ref()));
                        let raw = if pointer.is_empty() { item.clone() }
                        else { item.pointer(&pointer).unwrap_or(crate::vm::object::JsonVal::Null) };
                        let val = super::json::json_val_to_value(&raw);
                        new_row.push(val);
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                new_row.push(Value::from_bool(false));
            }
        }
        table.rows.push(new_row);
    }
}

pub fn sqlite_row_to_value(
    row: &rusqlite::Row<'_>,
    col_type: &crate::frontend::ast::Type,
    index: usize,
) -> Value {
    match col_type {
        crate::frontend::ast::Type::Int => Value::from_i64(row.get::<_, i64>(index).unwrap_or(0)),
        crate::frontend::ast::Type::Float => Value::from_f64(row.get::<_, f64>(index).unwrap_or(0.0)),
        crate::frontend::ast::Type::Bool => Value::from_bool(row.get::<_, i32>(index).unwrap_or(0) != 0),
        crate::frontend::ast::Type::String => {
            let s_val = row.get::<_, String>(index).unwrap_or_default();
            Value::from_string(Arc::new(crate::vm::object::StringObj::new(s_val.into_bytes())))
        }
        _ => Value::from_bool(false),
    }
}

