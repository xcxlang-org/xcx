use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::OpResult;
use crate::vm::object::{StringObj, SetObj, TableObj, RowObj};
use crate::vm::core::executor::Executor;

pub fn handle(op: OpCode, locals: &mut [Value]) -> Option<OpResult> {
    match op {
        OpCode::ArrayInit { dst, base, count } => {
            let res = crate::vm::core::runtime_ops::RuntimeOps::array_init(&locals[base as usize..(base as usize + count as usize)]);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::SetInit { dst, base, count } => {
            let res = crate::vm::core::runtime_ops::RuntimeOps::set_init(&locals[base as usize..(base as usize + count as usize)]);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::MapInit { dst, base, count } => {
            let res = crate::vm::core::runtime_ops::RuntimeOps::map_init(&locals[base as usize..(base as usize + (count * 2) as usize)]);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::Has { dst, src1, src2 } => {
            let col = locals[src1 as usize];
            let item = locals[src2 as usize];
            let res = col.has(item);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(res);
        }
        OpCode::GetIndex { dst, container, index } => {
            let c = locals[container as usize];
            let idx = locals[index as usize];
            let mut res = Value::from_bool(false);

            if c.is_array() {
                let arr = c.as_array();
                let arr_rd = arr.read();
                let i = idx.as_i64() as usize;
                if i < arr_rd.len() {
                    res = arr_rd[i];
                    unsafe { res.inc_ref(); }
                }
            } else if c.is_map() {
                let map = c.as_map();
                let map_rd = map.read();
                for (k, v) in map_rd.iter() {
                    if *k == idx {
                        res = *v;
                        unsafe { res.inc_ref(); }
                        break;
                    }
                }
            } else if c.is_string() {
                let s = c.as_string();
                let i = idx.as_i64() as usize;
                if i < s.data.len() {
                    let b = s.data[i];
                    res = Value::from_string(Arc::new(StringObj::new(vec![b])));
                }
            } else if c.is_json() {
                let j = c.as_json();
                let key = idx.to_string();
                match &j.root {
                    crate::vm::object::JsonVal::Object(o) => {
                        let o_read = o.read();
                        if let Some((_, val)) = o_read.iter().find(|(k, _)| k.as_str() == key.as_str()) {
                            res = crate::vm::utils::json_val_to_value(val);
                        }
                    }
                    crate::vm::object::JsonVal::Array(a) => {
                        if idx.is_int() {
                            let i = idx.as_i64() as usize;
                            let a_read = a.read();
                            if i < a_read.len() {
                                res = crate::vm::utils::json_val_to_value(&a_read[i]);
                            }
                        }
                    }
                    _ => {}
                }
            } else if c.is_table() {
                let tbl = c.as_table();
                let tbl_read = tbl.read();
                let i = idx.as_i64() as usize;
                if i < tbl_read.rows.len() {
                    res = Value::from_row(Arc::new(RowObj {
                        table: tbl.clone(),
                        row_idx: i as u32,
                    }));
                }
            }

            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::SetIndex { container, index, src } => {
            let c = locals[container as usize];
            let idx = locals[index as usize];
            let val = locals[src as usize];

            if c.is_array() {
                let arr = c.as_array();
                let mut arr_wr = arr.write();
                let i = idx.as_i64() as usize;
                if i < arr_wr.len() {
                    unsafe { val.inc_ref(); arr_wr[i].dec_ref(); }
                    arr_wr[i] = val;
                }
            } else if c.is_map() {
                let map = c.as_map();
                let mut map_wr = map.write();
                let mut found = false;
                for (k, v) in map_wr.iter_mut() {
                    if *k == idx {
                        unsafe { val.inc_ref(); v.dec_ref(); }
                        *v = val;
                        found = true;
                        break;
                    }
                }
                if !found {
                    unsafe { idx.inc_ref(); val.inc_ref(); }
                    map_wr.push((idx, val));
                }
            } else if c.is_json() {
                let j = c.as_json();
                let key = idx.to_string();
                if let crate::vm::object::JsonVal::Object(o) = &j.root {
                    let mut obj = o.write();
                    if let Some(pos) = obj.iter().position(|(k, _)| k.as_str() == key.as_str()) {
                        obj[pos].1 = crate::vm::utils::value_to_json(&val);
                    } else {
                        obj.push((std::sync::Arc::new(key), crate::vm::utils::value_to_json(&val)));
                    }
                }
            }
        }
        OpCode::SetUnion { dst, src1, src2 } => {
            let s1_rc = locals[src1 as usize].as_set();
            let s2_rc = locals[src2 as usize].as_set();
            let s1 = s1_rc.read();
            let s2 = s2_rc.read();
            let mut elements = std::collections::BTreeSet::new();
            for v in s1.elements.iter().chain(s2.elements.iter()) {
                if elements.insert(*v) {
                    unsafe { v.inc_ref(); }
                }
            }
            let res = Value::from_set(Arc::new(RwLock::new(SetObj::new(elements))));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::SetIntersection { dst, src1, src2 } => {
            let s1_rc = locals[src1 as usize].as_set();
            let s2_rc = locals[src2 as usize].as_set();
            let s1 = s1_rc.read();
            let s2 = s2_rc.read();
            let mut elements = std::collections::BTreeSet::new();
            for v in s1.elements.intersection(&s2.elements) {
                unsafe { v.inc_ref(); }
                elements.insert(*v);
            }
            let res = Value::from_set(Arc::new(RwLock::new(SetObj::new(elements))));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::SetDifference { dst, src1, src2 } => {
            let s1_rc = locals[src1 as usize].as_set();
            let s2_rc = locals[src2 as usize].as_set();
            let s1 = s1_rc.read();
            let s2 = s2_rc.read();
            let mut elements = std::collections::BTreeSet::new();
            for v in s1.elements.difference(&s2.elements) {
                unsafe { v.inc_ref(); }
                elements.insert(*v);
            }
            let res = Value::from_set(Arc::new(RwLock::new(SetObj::new(elements))));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::SetSymDifference { dst, src1, src2 } => {
            let s1_rc = locals[src1 as usize].as_set();
            let s2_rc = locals[src2 as usize].as_set();
            let s1 = s1_rc.read();
            let s2 = s2_rc.read();
            let mut elements = std::collections::BTreeSet::new();
            for v in s1.elements.symmetric_difference(&s2.elements) {
                unsafe { v.inc_ref(); }
                elements.insert(*v);
            }
            let res = Value::from_set(Arc::new(RwLock::new(SetObj::new(elements))));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::RandomChoice { dst, src } => {
            let col = locals[src as usize];
            if col.is_ptr() {
                let mut rng = rand::rng();
                use rand::Rng;
                let res = match col.tag {
                    tag if tag == crate::vm::value::TAG_ARR as u64 => {
                        let arr_rd = col.as_array();
                        let arr = arr_rd.read();
                        if arr.is_empty() { Value::from_bool(false) }
                        else { let v = arr[rng.random_range(0..arr.len())]; unsafe { v.inc_ref(); } v }
                    }
                    tag if tag == crate::vm::value::TAG_SET as u64 => {
                        let s_rd = col.as_set();
                        let mut s_write = s_rd.write();
                        if s_write.cache.is_none() {
                            s_write.cache = Some(s_write.elements.iter().cloned().collect());
                        }
                        let cache = s_write.cache.as_ref().unwrap();
                        if cache.is_empty() { Value::from_bool(false) }
                        else { let v = cache[rng.random_range(0..cache.len())]; unsafe { v.inc_ref(); } v }
                    }
                    _ => Value::from_bool(false)
                };
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
        }
        OpCode::IntConcat { dst, src1, src2 } => {
            let a = locals[src1 as usize].as_i64();
            let b = locals[src2 as usize].as_i64();
            let combined = format!("{}{}", a, b).parse::<i64>().unwrap_or(0);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_i64(combined);
        }
        OpCode::SetRange { dst, start, end, step, has_step } => {
            let v_s = locals[start as usize];
            let v_e = locals[end as usize];
            let v_h_step = !locals[has_step as usize].is_bool_false();
            let mut elements = std::collections::BTreeSet::new();

            if v_s.is_int() && v_e.is_int() {
                let v_start = v_s.as_i64();
                let v_end = v_e.as_i64();
                let v_step = if v_h_step { locals[step as usize].as_i64() } else { 1 };
                if v_step > 0 {
                    let mut curr = v_start;
                    while curr <= v_end {
                        let val = Value::from_i64(curr);
                        if !elements.insert(val) { unsafe { val.dec_ref(); } }
                        curr += v_step;
                    }
                } else if v_step < 0 {
                    let mut curr = v_start;
                    while curr >= v_end {
                        let val = Value::from_i64(curr);
                        if !elements.insert(val) { unsafe { val.dec_ref(); } }
                        curr += v_step;
                    }
                }
            } else if v_s.is_float() || v_e.is_float() {
                let v_start = v_s.cast_float();
                let v_end = v_e.cast_float();
                let v_step = if v_h_step { locals[step as usize].cast_float() } else { 1.0 };
                if v_step > 0.0 {
                    let mut curr = v_start;
                    while curr <= v_end + 1e-12 {
                        let val = Value::from_f64(curr);
                        if !elements.insert(val) { unsafe { val.dec_ref(); } }
                        curr += v_step;
                    }
                } else if v_step < 0.0 {
                    let mut curr = v_start;
                    while curr >= v_end - 1e-12 {
                        let val = Value::from_f64(curr);
                        if !elements.insert(val) { unsafe { val.dec_ref(); } }
                        curr += v_step;
                    }
                }
            } else if v_s.is_string() && v_e.is_string() {
                let s_start = v_s.as_string();
                let s_end = v_e.as_string();
                if s_start.data.len() == 1 && s_end.data.len() == 1 {
                    let v_start = s_start.data[0] as i64;
                    let v_end = s_end.data[0] as i64;
                    let v_step = if v_h_step { locals[step as usize].as_i64() } else { 1 };
                    if v_step > 0 {
                        let mut curr = v_start;
                        while curr <= v_end {
                            let v = Value::from_string(Arc::new(crate::vm::object::StringObj::new(vec![curr as u8])));
                            if !elements.insert(v) { unsafe { v.dec_ref(); } }
                            curr += v_step;
                        }
                    } else if v_step < 0 {
                        let mut curr = v_start;
                        while curr >= v_end {
                            let v = Value::from_string(Arc::new(crate::vm::object::StringObj::new(vec![curr as u8])));
                            if !elements.insert(v) { unsafe { v.dec_ref(); } }
                            curr += v_step;
                        }
                    }
                }
            }

            let res = Value::from_set(Arc::new(RwLock::new(SetObj::new(elements))));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
        }
        OpCode::RandomInt { dst, min, max, step, has_step } => {
            let v_min_val = locals[min as usize];
            let v_max_val = locals[max as usize];
            let v_has_step = !locals[has_step as usize].is_bool_false();
            let v_step_val = if v_has_step { locals[step as usize] } else { Value::from_i64(1) };
            
            let mut res_val = Value::from_bool(false);
            crate::runtime::builtin::math::random::xcx_jit_random_int(
                &mut res_val, 
                v_min_val.bits, v_min_val.tag,
                v_max_val.bits, v_max_val.tag,
                v_step_val.bits, v_step_val.tag,
                v_has_step
            );
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res_val;
        }
        OpCode::RandomFloat { dst, min, max, step, has_step } => {
            let v_min_val = locals[min as usize];
            let v_max_val = locals[max as usize];
            let v_has_step = !locals[has_step as usize].is_bool_false();
            let v_step_val = if v_has_step { locals[step as usize] } else { Value::from_f64(1.0) };
            
            let mut res_val = Value::from_bool(false);
            crate::runtime::builtin::math::random::xcx_jit_random_float(
                &mut res_val, 
                v_min_val.bits, v_min_val.tag,
                v_max_val.bits, v_max_val.tag,
                v_step_val.bits, v_step_val.tag,
                v_has_step
            );
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res_val;
        }
        OpCode::RowGet { dst, row_reg, col_idx } => {
            let row_val = locals[row_reg as usize];
            if row_val.is_row() {
                let row = row_val.as_row();
                let table = row.table.read();
                if (col_idx as usize) < table.columns.len() {
                    let val = table.rows[row.row_idx as usize][col_idx as usize];
                    unsafe { val.inc_ref(); }
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = val;
                }
            }
        }
        OpCode::TablePushRow { tbl_reg, row_reg } => {
            let tbl_val = locals[tbl_reg as usize];
            let row_val = locals[row_reg as usize];
            if tbl_val.is_table() && row_val.is_row() {
                let t_rc = tbl_val.as_table();
                let r_obj = row_val.as_row();
                
                let mut table = t_rc.write();
                let r_table = r_obj.table.read();
                let row_data = &r_table.rows[r_obj.row_idx as usize];
                
                let mut row_copy = Vec::with_capacity(row_data.len());
                for v in row_data {
                    unsafe { v.inc_ref(); }
                    row_copy.push(*v);
                }
                table.rows.push(row_copy);
            }
        }
        OpCode::TableCloneSkeleton { dst, src } => {
            let src_val = locals[src as usize];
            if src_val.is_table() {
                let t_rc = src_val.as_table();
                let t_read = t_rc.read();
                let res = Value::from_table(Arc::new(RwLock::new(TableObj {
                    table_name: t_read.table_name.clone(),
                    columns: t_read.columns.clone(),
                    rows: Vec::new(),
                    sql_binding: t_read.sql_binding.clone(),
                    sql_where: None,
                    pending_op: None,
                })));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
        }
        OpCode::TableInitRow { tbl_dst, base, col_count: _ } => {
            let tbl_val = locals[tbl_dst as usize];
            if tbl_val.is_table() {
                let tbl = tbl_val.as_table();
                let cols = {
                    let r = tbl.read();
                    r.columns.clone()
                };
                let row_idx = {
                    let r = tbl.read();
                    r.rows.len()
                };
                let start = base as usize;
                let mut current_offset = 0;
                let mut row = Vec::with_capacity(cols.len());
                for c in 0..cols.len() {
                    if cols[c].is_auto {
                        row.push(Value::from_i64(row_idx as i64 + 1));
                    } else {
                        let val = locals[start + current_offset];
                        if val.is_ptr() { unsafe { val.inc_ref(); } }
                        row.push(val);
                        current_offset += 1;
                    }
                }
                tbl.write().rows.push(row);
            }
        }
        _ => return None,
    }
    Some(OpResult::Continue)
}

pub fn handle_table_init(exec: &Executor, op: OpCode, locals: &mut [Value]) -> Option<OpResult> {
    match op {
        OpCode::TableInit { dst, skeleton_idx, base, row_count, col_count: _ } => {
            let skeleton_val = exec.ctx.constants[skeleton_idx as usize];
            let values = &locals[base as usize..];
            match crate::vm::core::runtime_ops::RuntimeOps::table_init(skeleton_val, row_count, values) {
                Ok(res) => {
                    unsafe { locals[dst as usize].dec_ref(); }
                    locals[dst as usize] = res;
                    return Some(OpResult::Continue);
                }
                Err(_e) => {
                    return Some(OpResult::Halt);
                }
            }
        }
        OpCode::TableBegin { dst, skeleton_idx } => {
            let skeleton_val = exec.ctx.constants[skeleton_idx as usize];
            if skeleton_val.is_table() {
                let s_rc = skeleton_val.as_table();
                let s_read = s_rc.read();
                let res = Value::from_table(Arc::new(RwLock::new(TableObj {
                    table_name: s_read.table_name.clone(),
                    columns: s_read.columns.clone(),
                    rows: Vec::new(),
                    sql_binding: s_read.sql_binding.clone(),
                    sql_where: None,
                    pending_op: None,
                })));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
                return Some(OpResult::Continue);
            } else {
                return Some(OpResult::Halt);
            }
        }
        _ => None,
    }
}
