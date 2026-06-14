use crate::vm::core::vm::OpResult;
use crate::vm::value::Value;
#[cfg(feature = "jit")]

pub fn set_json_value_at_path(target: &mut serde_json::Value, path: &str, value: serde_json::Value) {
    let is_simple = !path.starts_with('/') && !path.contains('.') && !path.contains('[') && !path.contains(']');
    if is_simple {
        if !target.is_object() {
            *target = serde_json::Value::Object(serde_json::Map::new());
        }
        let obj = target.as_object_mut().unwrap();
        obj.insert(path.to_string(), value);
        return;
    }

    let pointer = crate::runtime::builtin::json::access::normalize_json_path(path);
    let parts: Vec<&str> = pointer.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        *target = value;
        return;
    }
    let mut current = target;
    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        if let Ok(idx) = part.parse::<usize>() {
            if !current.is_array() {
                *current = serde_json::Value::Array(Vec::new());
            }
            let arr = current.as_array_mut().unwrap();
            while arr.len() <= idx {
                arr.push(serde_json::Value::Null);
            }
            if is_last {
                arr[idx] = value;
                return;
            }
            current = &mut arr[idx];
        } else {
            if !current.is_object() {
                *current = serde_json::Value::Object(serde_json::Map::new());
            }
            let obj = current.as_object_mut().unwrap();
            if is_last {
                obj.insert(part.to_string(), value);
                return;
            }
            let next_is_array = if i + 1 < parts.len() {
                parts[i+1].parse::<usize>().is_ok()
            } else {
                false
            };
            current = obj.entry(part.to_string()).or_insert_with(|| {
                if next_is_array {
                    serde_json::Value::Array(Vec::new())
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                }
            });
        }
    }
}

pub fn json_inject_table_impl(executor: &mut crate::vm::core::executor::Executor, table_val: Value, json_val: Value, mapping_val: Value) -> OpResult {
    let res = executor.native_inject_table(&table_val, json_val, &mapping_val);
    unsafe {
        table_val.dec_ref();
        mapping_val.dec_ref();
    }
    res
}

