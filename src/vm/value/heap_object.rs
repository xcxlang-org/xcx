use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::object::{
    TableObj, SetObj, FiberObj, RowObj, DatabaseObj, StringObj, ArrayObj, MapObj, JsonObj, FunctionObj, ClosureObj, BoolArrayObj
};
use super::value::Value;
use super::nan_boxing::*;
use super::tag::*;

// --- Constructors ---

pub fn from_string(s: Arc<StringObj>)           -> Value { Value::pack_ptr(Arc::into_raw(s), TAG_STR) }
pub fn from_array(a: Arc<RwLock<ArrayObj>>)     -> Value { Value::pack_ptr(Arc::into_raw(a), TAG_ARR) }
pub fn from_bool_array(a: Arc<RwLock<BoolArrayObj>>) -> Value { Value::pack_ptr(Arc::into_raw(a), TAG_BOOL_ARR) }
pub fn from_set(s: Arc<RwLock<SetObj>>)         -> Value { Value::pack_ptr(Arc::into_raw(s), TAG_SET) }
pub fn from_map(m: Arc<RwLock<MapObj>>)         -> Value { Value::pack_ptr(Arc::into_raw(m), TAG_MAP) }
pub fn from_table(t: Arc<RwLock<TableObj>>)     -> Value { Value::pack_ptr(Arc::into_raw(t), TAG_TBL) }
pub fn from_json(j: Arc<JsonObj>)               -> Value { Value::pack_ptr(Arc::into_raw(j), TAG_JSON) }
pub fn from_fiber(f: Arc<RwLock<FiberObj>>)     -> Value { Value::pack_ptr(Arc::into_raw(f), TAG_FIB) }
pub fn from_db(d: Arc<DatabaseObj>)             -> Value { Value::pack_ptr(Arc::into_raw(d), TAG_DB) }
pub fn from_function_ptr(f: Arc<FunctionObj>)   -> Value { Value::pack_ptr(Arc::into_raw(f), TAG_FUNC_PTR) }
pub fn from_closure(c: Arc<ClosureObj>)         -> Value { Value::pack_ptr(Arc::into_raw(c), TAG_CLOSURE) }
pub fn from_row(r: Arc<RowObj>)                 -> Value { Value::pack_ptr(Arc::into_raw(r), TAG_ROW) }

/// Date is stored inline: bits = timestamp in ms, tag = TAG_DATE.
pub fn from_date(ts: i64) -> Value {
    Value { bits: ts as u64, tag: TAG_DATE }
}

/// Arena strings and JSON are stored as raw pointers with TAG_ARENA.
/// The inner type is encoded in the upper 4 bits of the bits field.
pub fn from_arena_string(s: *const StringObj) -> Value {
    Value { bits: (s as u64) | ((TAG_STR as u64) << 48), tag: TAG_ARENA }
}

pub fn from_arena_json(j: *const JsonObj) -> Value {
    Value { bits: (j as u64) | ((TAG_JSON as u64) << 48), tag: TAG_ARENA }
}

/// Returns the inner type tag for arena values.
/// The inner tag is stored in bits 48-55 of `bits`.
pub fn arena_inner_tag(val: &Value) -> u64 {
    debug_assert_eq!(val.tag, TAG_ARENA);
    (val.bits >> 48) & 0xFF
}

/// Returns the raw pointer for arena values (bits 0-47).
pub fn arena_ptr<T>(val: &Value) -> *const T {
    (val.bits & 0x0000_FFFF_FFFF_FFFF) as *const T
}

// --- Accessors ---

pub fn as_string(val: &Value) -> Arc<StringObj> {
    if val.tag == TAG_ARENA {
        let p = arena_ptr::<StringObj>(val);
        return Arc::new(unsafe { (*p).clone() });
    }
    debug_assert!(val.is_string(), "Expected String, found {:?}", val.tag());
    unsafe {
        let p = val.unpack_ptr::<StringObj>();
        let arc: Arc<StringObj> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_array(val: &Value) -> Arc<RwLock<ArrayObj>> {
    debug_assert!(val.is_array(), "Expected Array, found {:?}", val.tag());
    unsafe {
        let p = val.unpack_ptr::<RwLock<ArrayObj>>();
        let arc: Arc<RwLock<ArrayObj>> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_bool_array(val: &Value) -> Arc<RwLock<BoolArrayObj>> {
    debug_assert_eq!(val.tag, TAG_BOOL_ARR, "Expected BoolArray, found tag={}", val.tag);
    unsafe {
        let p = val.unpack_ptr::<RwLock<BoolArrayObj>>();
        let arc: Arc<RwLock<BoolArrayObj>> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_set(val: &Value) -> Arc<RwLock<SetObj>> {
    debug_assert!(val.is_set(), "Expected Set, found {:?}", val.tag());
    unsafe {
        let p = val.unpack_ptr::<RwLock<SetObj>>();
        let arc: Arc<RwLock<SetObj>> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_map(val: &Value) -> Arc<RwLock<MapObj>> {
    debug_assert!(val.is_map(), "Expected Map, found {:?}", val.tag());
    unsafe {
        let p = val.unpack_ptr::<RwLock<MapObj>>();
        let arc: Arc<RwLock<MapObj>> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_table(val: &Value) -> Arc<RwLock<TableObj>> {
    debug_assert!(val.is_table(), "Expected Table, found {:?}", val.tag());
    unsafe {
        let p = val.unpack_ptr::<RwLock<TableObj>>();
        let arc: Arc<RwLock<TableObj>> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_json(val: &Value) -> Arc<JsonObj> {
    if val.tag == TAG_ARENA {
        let p = arena_ptr::<JsonObj>(val);
        return Arc::new(unsafe { (*p).clone() });
    }
    debug_assert!(val.is_json(), "Expected JSON, found {:?}", val.tag());
    unsafe {
        let p = val.unpack_ptr::<JsonObj>();
        let arc: Arc<JsonObj> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_fiber(val: &Value) -> Arc<RwLock<FiberObj>> {
    unsafe {
        let p = val.unpack_ptr::<RwLock<FiberObj>>();
        let arc: Arc<RwLock<FiberObj>> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_row(val: &Value) -> Arc<RowObj> {
    unsafe {
        let p = val.unpack_ptr::<RowObj>();
        let arc: Arc<RowObj> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_db(val: &Value) -> Arc<DatabaseObj> {
    unsafe {
        let p = val.unpack_ptr::<DatabaseObj>();
        let arc: Arc<DatabaseObj> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_function(val: &Value) -> Arc<FunctionObj> {
    debug_assert_eq!(val.tag, TAG_FUNC_PTR, "Expected FunctionPtr, found tag={}", val.tag);
    unsafe {
        let p = val.unpack_ptr::<FunctionObj>();
        let arc: Arc<FunctionObj> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

pub fn as_closure(val: &Value) -> Arc<ClosureObj> {
    unsafe {
        let p = val.unpack_ptr::<ClosureObj>();
        let arc: Arc<ClosureObj> = Arc::from_raw(p);
        let cl = arc.clone();
        std::mem::forget(arc);
        cl
    }
}

/// Date: bits field IS the timestamp in milliseconds.
pub fn as_date(val: &Value) -> i64 {
    debug_assert_eq!(val.tag, TAG_DATE, "Expected Date");
    val.bits as i64
}

/// Function stored with a heap pointer: return the chunk index from the raw ptr (NOT supported as inline).
/// For inline function index (TAG_FUNC with no heap ptr), use val.bits as u32.
pub fn as_function_idx(val: &Value) -> u32 {
    val.bits as u32
}

pub fn to_string(val: &Value) -> String {
    match val.tag {
        TAG_FLOAT => unpack_float_bits(val.bits).to_string(),
        TAG_INT   => (val.bits as i64).to_string(),
        TAG_BOOL  => if val.bits != 0 { "true".to_string() } else { "false".to_string() },
        TAG_DATE  => {
            let ts = val.bits as i64;
            let dt = chrono::DateTime::from_timestamp_millis(ts).unwrap().naive_utc();
            dt.format("%Y-%m-%d").to_string()
        }
        TAG_STR   => {
            let b = as_string(val);
            String::from_utf8_lossy(&b.data).into_owned()
        }
        TAG_JSON  => {
            let arc = as_json(val);
            let ver = arc.version.load(std::sync::atomic::Ordering::Acquire);
            let cached_ver = arc.cached_version.load(std::sync::atomic::Ordering::Acquire);
            if ver == cached_ver {
                if let Some(s) = arc.cached_str.lock().as_ref() {
                    return String::from_utf8_lossy(&s.data).into_owned();
                }
            }
            let mut buf = String::with_capacity(4096);
            arc.root.to_string_buf(&mut buf);
            let s = buf;
            let string_obj = Arc::new(StringObj::new(s.into_bytes()));
            
            let mut lock = arc.cached_str.lock();
            if arc.version.load(std::sync::atomic::Ordering::Acquire) == ver {
                *lock = Some(string_obj.clone());
                arc.cached_version.store(ver, std::sync::atomic::Ordering::Release);
            }
            String::from_utf8_lossy(&string_obj.data).into_owned()
        }
        TAG_ARR   => {
            let arc = as_array(val);
            let arr = arc.read();
            let mut out = String::from("[");
            for (i, v) in arr.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push_str(&v.to_string());
            }
            out.push(']');
            out
        }
        TAG_BOOL_ARR => {
            let arc = as_bool_array(val);
            let arr = arc.read();
            let mut out = String::from("[");
            for (i, b) in arr.data.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push_str(if *b != 0 { "true" } else { "false" });
            }
            out.push(']');
            out
        }
        TAG_SET   => {
            let arc = as_set(val);
            let set_data = arc.read();
            set_data.elements.iter().enumerate()
                .map(|(i, v)| if i > 0 { format!(", {}", v.to_string()) } else { v.to_string() })
                .collect()
        }
        TAG_MAP   => {
            let arc = as_map(val);
            let map_data = arc.read();
            map_data.iter().enumerate()
                .map(|(i, (k, v))| {
                    let entry = format!("{} :: {}", k.to_string(), v.to_string());
                    if i > 0 { format!(", {}", entry) } else { entry }
                })
                .collect()
        }
        TAG_TBL   => {
            let arc = as_table(val);
            format!("Table(rows: {})", arc.read().rows.len())
        }
        TAG_FUNC | TAG_FUNC_PTR => "Function".to_string(),
        TAG_ROW   => format!("Row({})", as_row(val).row_idx),
        TAG_FIB   => {
            let arc = as_fiber(val);
            let fib = arc.read();
            if fib.is_done { "Fiber(done)".to_string() }
            else { format!("Fiber(ip={})", fib.ip) }
        }
        TAG_DB    => {
            let arc = as_db(val);
            format!("Database(engine={}, path={})", arc.engine, arc.path)
        }
        TAG_ARENA => {
            let inner = arena_inner_tag(val);
            let tmp = Value { bits: arena_ptr::<()>(val) as u64, tag: inner as u64 };
            to_string(&tmp)
        }
        _ => format!("Value(tag={}, bits={:x})", val.tag, val.bits),
    }
}

pub fn to_sql_value(val: &Value) -> rusqlite::types::Value {
    if val.is_string() {
        let b = as_string(val);
        rusqlite::types::Value::Text(String::from_utf8_lossy(&b.data).into_owned())
    } else {
        match val.tag {
            TAG_INT   => rusqlite::types::Value::Integer(val.bits as i64),
            TAG_FLOAT => rusqlite::types::Value::Real(unpack_float_bits(val.bits)),
            TAG_BOOL  => rusqlite::types::Value::Integer(if val.bits != 0 { 1 } else { 0 }),
            _ => rusqlite::types::Value::Null,
        }
    }
}
