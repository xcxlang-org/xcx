use std::sync::Arc;
use parking_lot::RwLock;
use crate::vm::object::{
    TableObj, SetObj, FiberObj, RowObj, DatabaseObj, StringObj, ArrayObj, MapObj, JsonObj, ClosureObj, FunctionObj, BoolArrayObj
};
use super::value::Value;
use super::nan_boxing::*;
use super::tag::*;

#[inline]
pub unsafe fn inc_ref(val: &Value) {
    if !val.is_ptr() { return; }
    let p = val.bits as *const ();
    match val.tag {
        TAG_STR     => unsafe { Arc::increment_strong_count(p as *const StringObj); }
        TAG_ARR     => unsafe { Arc::increment_strong_count(p as *const RwLock<ArrayObj>); }
        TAG_BOOL_ARR => unsafe { Arc::increment_strong_count(p as *const RwLock<BoolArrayObj>); }
        TAG_SET     => unsafe { Arc::increment_strong_count(p as *const RwLock<SetObj>); }
        TAG_MAP     => unsafe { Arc::increment_strong_count(p as *const RwLock<MapObj>); }
        TAG_TBL     => unsafe { Arc::increment_strong_count(p as *const RwLock<TableObj>); }
        TAG_JSON    => unsafe { Arc::increment_strong_count(p as *const JsonObj); }
        TAG_FIB     => unsafe { Arc::increment_strong_count(p as *const RwLock<FiberObj>); }
        TAG_ROW     => unsafe { Arc::increment_strong_count(p as *const RowObj); }
        TAG_DB      => unsafe { Arc::increment_strong_count(p as *const DatabaseObj); }
        TAG_CLOSURE => unsafe { Arc::increment_strong_count(p as *const ClosureObj); }
        TAG_FUNC_PTR => unsafe { Arc::increment_strong_count(p as *const FunctionObj); }
        _ => {}
    }
}

#[inline]
pub unsafe fn dec_ref(val: &Value) {
    if !val.is_ptr() { return; }
    let p = val.bits as *const ();
    match val.tag {
        TAG_STR     => unsafe { Arc::decrement_strong_count(p as *const StringObj); }
        TAG_ARR     => unsafe { Arc::decrement_strong_count(p as *const RwLock<ArrayObj>); }
        TAG_BOOL_ARR => unsafe { Arc::decrement_strong_count(p as *const RwLock<BoolArrayObj>); }
        TAG_SET     => unsafe { Arc::decrement_strong_count(p as *const RwLock<SetObj>); }
        TAG_MAP     => unsafe { Arc::decrement_strong_count(p as *const RwLock<MapObj>); }
        TAG_TBL     => unsafe { Arc::decrement_strong_count(p as *const RwLock<TableObj>); }
        TAG_JSON    => unsafe { Arc::decrement_strong_count(p as *const JsonObj); }
        TAG_FIB     => unsafe { Arc::decrement_strong_count(p as *const RwLock<FiberObj>); }
        TAG_ROW     => unsafe { Arc::decrement_strong_count(p as *const RowObj); }
        TAG_DB      => unsafe { Arc::decrement_strong_count(p as *const DatabaseObj); }
        TAG_CLOSURE => unsafe { Arc::decrement_strong_count(p as *const ClosureObj); }
        TAG_FUNC_PTR => unsafe { Arc::decrement_strong_count(p as *const FunctionObj); }
        _ => {}
    }
}

