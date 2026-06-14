use std::sync::Arc;
use parking_lot::RwLock;

use crate::vm::object::{TableObj, SetObj, FiberObj, RowObj, DatabaseObj, StringObj, ArrayObj, MapObj, JsonObj};

use super::nan_boxing::{
    TAG_FLOAT, TAG_INT, TAG_BOOL, TAG_DATE, TAG_STR, TAG_ARR, TAG_SET,
    TAG_MAP, TAG_TBL, TAG_FUNC, TAG_ROW, TAG_JSON, TAG_FIB, TAG_DB,
    TAG_CLOSURE, TAG_ARENA, TAG_FIRST_PTR, TAG_FUNC_PTR,
    pack_float_bits, unpack_float_bits,
};
use super::tag::Tag;
use super::ref_count;
use super::heap_object;

/// A tagged runtime value.
///
/// Layout: 16 bytes, repr(C), align(8).
/// `bits` holds the raw payload: i64 bits for integers, f64 bits for floats,
/// or the full 64-bit pointer for heap types.
/// `tag`  is one of the TAG_* constants defined in nan_boxing.
/// `_pad` is zero-fill to preserve alignment and allow stack slot loads of full 16 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Value {
    pub bits: u64,
    pub tag:  u64,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.tag != other.tag { return false; }
        match self.tag {
            TAG_FLOAT  => unpack_float_bits(self.bits) == unpack_float_bits(other.bits),
            TAG_INT    => self.bits == other.bits,
            TAG_BOOL   => self.bits == other.bits,
            TAG_DATE   => self.bits == other.bits,
            TAG_STR    => {
                let s1 = heap_object::as_string(self);
                let s2 = heap_object::as_string(other);
                *s1 == *s2
            }
            TAG_ARR    => {
                let a1 = heap_object::as_array(self);
                let a2 = heap_object::as_array(other);
                *a1.read() == *a2.read()
            }
            TAG_SET    => {
                let s1 = heap_object::as_set(self);
                let s2 = heap_object::as_set(other);
                s1.read().elements == s2.read().elements
            }
            TAG_MAP    => {
                let m1 = heap_object::as_map(self);
                let m2 = heap_object::as_map(other);
                m1.read().elements == m2.read().elements
            }
            _ => self.bits == other.bits,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let r1 = self.variant_rank();
        let r2 = other.variant_rank();
        if r1 != r2 { return r1.cmp(&r2); }

        match self.tag {
            TAG_INT   => self.as_i64().cmp(&other.as_i64()),
            TAG_FLOAT => self.as_f64().partial_cmp(&other.as_f64()).unwrap_or(std::cmp::Ordering::Equal),
            TAG_BOOL  => self.as_bool().cmp(&other.as_bool()),
            TAG_STR   => self.as_string().cmp(&other.as_string()),
            TAG_DATE  => self.as_date().cmp(&other.as_date()),
            _         => self.bits.cmp(&other.bits),
        }
    }
}

impl std::ops::Neg for Value {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        if self.is_int()   { Self::from_i64(-(self.as_i64())) }
        else if self.is_float() { Self::from_f64(-(self.as_f64())) }
        else { self }
    }
}

impl Value {
    // --- Arithmetic ---

    #[inline(always)]
    pub fn add(self, rhs: Self) -> Self {
        if self.tag == TAG_INT && rhs.tag == TAG_INT {
            return Self::from_i64((self.bits as i64).wrapping_add(rhs.bits as i64));
        }
        if self.is_string() || rhs.is_string() {
            let s1 = self.to_string();
            let s2 = rhs.to_string();
            let combined = s1 + &s2;
            return Self::from_string(Arc::new(StringObj::new(combined.into_bytes())));
        }
        if self.is_float() || rhs.is_float() {
            return Self::from_f64(self.as_f64() + rhs.as_f64());
        }
        if self.is_date() && rhs.is_int() {
            return Self::from_date(self.as_date() + rhs.as_i64() * 86_400_000);
        }
        if self.is_int() && rhs.is_date() {
            return Self::from_date(rhs.as_date() + self.as_i64() * 86_400_000);
        }
        Self::from_i64((self.bits as i64).wrapping_add(rhs.bits as i64))
    }

    #[inline]
    pub fn sub(self, rhs: Self) -> Self {
        if self.tag == TAG_INT && rhs.tag == TAG_INT {
            return Self::from_i64((self.bits as i64).wrapping_sub(rhs.bits as i64));
        }
        if self.is_float() || rhs.is_float() {
            return Self::from_f64(self.as_f64() - rhs.as_f64());
        }
        if self.is_date() && rhs.is_int() {
            return Self::from_date(self.as_date() - rhs.as_i64() * 86_400_000);
        }
        if self.is_date() && rhs.is_date() {
            let diff_ms = self.as_date() - rhs.as_date();
            return Self::from_i64(diff_ms / 86_400_000);
        }
        Self::from_i64((self.bits as i64).wrapping_sub(rhs.bits as i64))
    }

    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        if self.tag == TAG_INT && rhs.tag == TAG_INT {
            return Self::from_i64((self.bits as i64).wrapping_mul(rhs.bits as i64));
        }
        Self::from_f64(self.as_f64() * rhs.as_f64())
    }

    #[inline]
    pub fn div(self, rhs: Self) -> Result<Self, ()> {
        if self.is_float() || rhs.is_float() {
            let r_f = rhs.as_f64();
            if r_f == 0.0 {
                return Err(());
            }
            return Ok(Self::from_f64(self.as_f64() / r_f));
        }
        let r = rhs.as_i64();
        if r == 0 {
            return Err(());
        }
        Ok(Self::from_i64(self.as_i64() / r))
    }

    #[inline]
    pub fn rem(self, rhs: Self) -> Result<Self, ()> {
        if self.is_float() || rhs.is_float() {
            let r_f = rhs.as_f64();
            if r_f == 0.0 {
                return Err(());
            }
            return Ok(Self::from_f64(self.as_f64() % r_f));
        }
        let r = rhs.as_i64();
        if r == 0 {
            return Err(());
        }
        // Truncating remainder — consistent with `div` (truncating toward zero).
        Ok(Self::from_i64(self.as_i64() % r))
    }

    #[inline]
    pub fn pow(self, rhs: Self) -> Self {
        if self.is_float() || rhs.is_float() {
            return Self::from_f64(self.as_f64().powf(rhs.as_f64()));
        }
        let b = rhs.as_i64();
        if b >= 0 && b <= u32::MAX as i64 {
            Self::from_i64(self.as_i64().wrapping_pow(b as u32))
        } else {
            Self::from_f64(self.as_f64().powf(rhs.as_f64()))
        }
    }

    #[inline]
    pub fn neg(self) -> Self {
        if self.is_float() { Self::from_f64(-self.as_f64()) }
        else { Self::from_i64(-(self.as_i64())) }
    }

    // --- Constructors ---

    #[inline]
    pub fn from_f64(f: f64) -> Self {
        Self { bits: pack_float_bits(f), tag: TAG_FLOAT }
    }

    #[inline(always)]
    pub fn from_i64(v: i64) -> Self {
        Self { bits: v as u64, tag: TAG_INT }
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self { bits: b as u64, tag: TAG_BOOL }
    }

    #[inline]
    pub fn pack_ptr<T>(ptr: *const T, tag: u64) -> Self {
        Self { bits: ptr as u64, tag }
    }

    #[inline]
    pub fn unpack_ptr<T>(&self) -> *const T {
        self.bits as *const T
    }

    // --- Type queries ---

    #[inline] pub fn is_float(&self)   -> bool { self.tag == TAG_FLOAT }
    #[inline] pub fn is_int(&self)     -> bool { self.tag == TAG_INT }
    #[inline] pub fn is_bool(&self)    -> bool { self.tag == TAG_BOOL }
    #[inline] pub fn is_date(&self)    -> bool { self.tag == TAG_DATE }
    #[inline] pub fn is_ptr(&self)     -> bool { self.tag >= TAG_FIRST_PTR }
    #[inline] pub fn is_arena(&self)   -> bool { self.tag == TAG_ARENA }
    #[inline] pub fn is_string(&self)  -> bool { self.tag == TAG_STR || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_STR) }
    #[inline] pub fn is_array(&self)   -> bool { self.tag == TAG_ARR || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_ARR) }
    #[inline] pub fn is_set(&self)     -> bool { self.tag == TAG_SET || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_SET) }
    #[inline] pub fn is_map(&self)     -> bool { self.tag == TAG_MAP || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_MAP) }
    #[inline] pub fn is_table(&self)   -> bool { self.tag == TAG_TBL || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_TBL) }
    #[inline] pub fn is_func(&self)    -> bool { self.tag == TAG_FUNC || self.tag == TAG_FUNC_PTR || (self.tag == TAG_ARENA && (heap_object::arena_inner_tag(self) == TAG_FUNC || heap_object::arena_inner_tag(self) == TAG_FUNC_PTR)) }
    #[inline] pub fn is_json(&self)    -> bool { self.tag == TAG_JSON || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_JSON) }
    #[inline] pub fn is_fiber(&self)   -> bool { self.tag == TAG_FIB || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_FIB) }
    #[inline] pub fn is_row(&self)     -> bool { self.tag == TAG_ROW || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_ROW) }
    #[inline] pub fn is_db(&self)      -> bool { self.tag == TAG_DB || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_DB) }
    #[inline] pub fn is_closure(&self) -> bool { self.tag == TAG_CLOSURE || (self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_CLOSURE) }
    #[inline] pub fn is_numeric(&self) -> bool { self.tag == TAG_INT || self.tag == TAG_FLOAT }
    #[inline] pub fn is_bool_false(&self) -> bool { self.tag == TAG_BOOL && self.bits == 0 }

    pub fn tag(&self) -> Tag {
        match self.tag {
            TAG_FLOAT   => Tag::Float,
            TAG_INT     => Tag::Int,
            TAG_BOOL    => Tag::Bool,
            TAG_DATE    => Tag::Date,
            TAG_STR     => Tag::String,
            TAG_ARR     => Tag::Array,
            TAG_SET     => Tag::Set,
            TAG_MAP     => Tag::Map,
            TAG_TBL     => Tag::Table,
            TAG_FUNC    => Tag::Function,
            TAG_FUNC_PTR => Tag::Function,
            TAG_ROW     => Tag::Row,
            TAG_JSON    => Tag::Json,
            TAG_FIB     => Tag::Fiber,
            TAG_DB      => Tag::Database,
            TAG_ARENA   => {
                match heap_object::arena_inner_tag(self) {
                    TAG_STR     => Tag::String,
                    TAG_ARR     => Tag::Array,
                    TAG_SET     => Tag::Set,
                    TAG_MAP     => Tag::Map,
                    TAG_TBL     => Tag::Table,
                    TAG_FUNC | TAG_FUNC_PTR => Tag::Function,
                    TAG_ROW     => Tag::Row,
                    TAG_JSON    => Tag::Json,
                    TAG_FIB     => Tag::Fiber,
                    TAG_DB      => Tag::Database,
                    TAG_CLOSURE => Tag::Unknown,
                    _           => Tag::Unknown,
                }
            }
            _ => Tag::Unknown,
        }
    }

    pub fn type_name(&self) -> &'static str {
        self.tag().name()
    }

    // --- Accessors ---

    #[inline] pub fn as_f64(&self)  -> f64  { unpack_float_bits(self.bits) }
    #[inline] pub fn as_i64(&self)  -> i64  { self.bits as i64 }
    #[inline] pub fn as_bool(&self) -> bool { self.bits != 0 }

    /// Returns the raw bits of this value. Used by the JIT FFI layer.
    #[inline] pub fn as_bits(&self) -> u64 { self.bits }

    // --- Ref counting ---

    #[inline]
    pub unsafe fn inc_ref(&self) {
        if self.tag == TAG_ARENA { return; }
        unsafe { ref_count::inc_ref(self) }
    }

    #[inline]
    pub unsafe fn dec_ref(&self) {
        if self.tag == TAG_ARENA { return; }
        unsafe { ref_count::dec_ref(self) }
    }

    /// Assigns this value to a destination, handling refcounts.
    #[inline]
    pub unsafe fn assign_to(&self, dest: &mut Value) {
        if self.is_ptr() { unsafe { self.inc_ref(); } }
        if dest.is_ptr() { unsafe { dest.dec_ref(); } }
        *dest = *self;
    }

    /// Replaces the destination assuming this value already has its refcount incremented.
    #[inline]
    pub unsafe fn replace_at(&self, dest: &mut Value) {
        if dest.is_ptr() { unsafe { dest.dec_ref(); } }
        *dest = *self;
    }

    // --- Comparison helpers ---

    pub fn xcx_eq(&self, rhs: &Self) -> bool { self == rhs }
    pub fn xcx_ne(&self, rhs: &Self) -> bool { self != rhs }
    pub fn xcx_lt(&self, rhs: &Self) -> bool { self < rhs }
    pub fn xcx_le(&self, rhs: &Self) -> bool { self <= rhs }
    pub fn xcx_gt(&self, rhs: &Self) -> bool { self > rhs }
    pub fn xcx_ge(&self, rhs: &Self) -> bool { self >= rhs }

    pub fn variant_rank(&self) -> u8 {
        match self.tag {
            TAG_INT     => 0,
            TAG_FLOAT   => 1,
            TAG_BOOL    => 2,
            TAG_STR     => 3,
            TAG_ARR     => 4,
            TAG_SET     => 5,
            TAG_MAP     => 6,
            TAG_DATE    => 7,
            TAG_TBL     => 8,
            TAG_FUNC    => 9,
            TAG_FUNC_PTR => 9,
            TAG_ROW     => 10,
            TAG_JSON    => 11,
            TAG_FIB     => 12,
            TAG_DB      => 13,
            _           => 255,
        }
    }

    // --- Heap type constructors (forwarded to heap_object) ---

    #[inline] pub fn from_string(s: Arc<StringObj>)               -> Self { heap_object::from_string(s) }
    #[inline] pub fn from_array(a: Arc<RwLock<ArrayObj>>)         -> Self { heap_object::from_array(a) }
    #[inline] pub fn from_set(s: Arc<RwLock<SetObj>>)             -> Self { heap_object::from_set(s) }
    #[inline] pub fn from_map(m: Arc<RwLock<MapObj>>)             -> Self { heap_object::from_map(m) }
    #[inline] pub fn from_table(t: Arc<RwLock<TableObj>>)         -> Self { heap_object::from_table(t) }
    #[inline] pub fn from_json(j: Arc<JsonObj>)                   -> Self { heap_object::from_json(j) }
    #[inline] pub fn from_fiber(f: Arc<RwLock<FiberObj>>)         -> Self { heap_object::from_fiber(f) }
    #[inline] pub fn from_database(d: Arc<DatabaseObj>)           -> Self { heap_object::from_db(d) }
    #[inline] pub fn from_function(id: u32)                       -> Self { Self { bits: id as u64, tag: TAG_FUNC } }
    #[inline] pub fn from_function_ptr(f: Arc<crate::vm::object::FunctionObj>) -> Self { heap_object::from_function_ptr(f) }
    #[inline] pub fn from_row(r: Arc<RowObj>)                     -> Self { heap_object::from_row(r) }
    #[inline] pub fn from_date(ts: i64)                           -> Self { heap_object::from_date(ts) }

    pub fn from_string_array(strs: Arc<Vec<String>>) -> Self {
        let mut vals = Vec::with_capacity(strs.len());
        for s in strs.iter() {
            vals.push(Self::from_string(Arc::new(StringObj::new(s.clone().into_bytes()))));
        }
        Self::from_array(Arc::new(RwLock::new(ArrayObj::new(vals))))
    }

    // --- Heap type accessors (forwarded to heap_object) ---

    pub fn as_string(&self) -> Arc<StringObj>               { heap_object::as_string(self) }
    pub fn as_array(&self)  -> Arc<RwLock<ArrayObj>>        { heap_object::as_array(self) }
    pub fn as_set(&self)    -> Arc<RwLock<SetObj>>          { heap_object::as_set(self) }
    pub fn as_map(&self)    -> Arc<RwLock<MapObj>>          { heap_object::as_map(self) }
    pub fn as_table(&self)  -> Arc<RwLock<TableObj>>        { heap_object::as_table(self) }
    pub fn as_json(&self)   -> Arc<JsonObj>                 { heap_object::as_json(self) }
    pub fn as_fiber(&self)  -> Arc<RwLock<FiberObj>>        { heap_object::as_fiber(self) }
    pub fn as_row(&self)    -> Arc<RowObj>                  { heap_object::as_row(self) }
    #[inline] pub fn as_database(&self) -> Arc<DatabaseObj> { heap_object::as_db(self) }
    #[inline] pub fn as_date(&self)     -> i64              { heap_object::as_date(self) }
    #[inline] pub fn as_function_idx(&self) -> u32          { heap_object::as_function_idx(self) }
    pub fn as_function(&self) -> Arc<crate::vm::object::FunctionObj> { heap_object::as_function(self) }

    pub fn as_array_opt(&self) -> Option<Arc<RwLock<ArrayObj>>> {
        if self.is_array() { Some(self.as_array()) } else { None }
    }

    pub fn to_sql_value(&self) -> rusqlite::types::Value { heap_object::to_sql_value(self) }

    #[inline]
    pub fn matches_str(&self, other: &str) -> bool {
        if !self.is_string() { return false; }
        let s = self.as_string();
        s.data.as_slice() == other.as_bytes()
    }

    /// Safely borrows the underlying string slice if this is a String or an Arena String.
    /// Returns None for other types. In order to avoid allocating new String objects, this
    /// method returns a direct, zero-copy borrowed string reference.
    pub unsafe fn as_str_borrow<'a>(&'a self) -> Option<&'a str> {
        if self.tag == TAG_STR {
            unsafe {
                let p = self.unpack_ptr::<StringObj>();
                std::str::from_utf8(&(*p).data).ok()
            }
        } else if self.tag == TAG_ARENA && heap_object::arena_inner_tag(self) == TAG_STR {
            unsafe {
                let p = heap_object::arena_ptr::<StringObj>(self);
                std::str::from_utf8(&(*p).data).ok()
            }
        } else {
            None
        }
    }

    pub fn as_string_lossy(&self) -> String {
        if !self.is_string() { return self.to_string(); }
        let s = self.as_string();
        String::from_utf8_lossy(&s.data).into_owned()
    }

    pub fn to_string(&self) -> String { heap_object::to_string(self) }
    pub fn typeof_str(&self) -> &'static str { self.type_name() }

    pub fn cast_int(&self) -> i64 {
        if self.is_int()    { self.as_i64() }
        else if self.is_float() { self.as_f64() as i64 }
        else if self.is_bool()  { if self.as_bool() { 1 } else { 0 } }
        else if self.is_string() { self.as_string_lossy().parse::<i64>().unwrap_or(0) }
        else if self.is_date()  { self.as_date() }
        else { 0 }
    }

    pub fn cast_float(&self) -> f64 {
        if self.is_float()  { self.as_f64() }
        else if self.is_int()   { self.as_i64() as f64 }
        else if self.is_bool()  { if self.as_bool() { 1.0 } else { 0.0 } }
        else if self.is_string() { self.as_string_lossy().parse::<f64>().unwrap_or(0.0) }
        else if self.is_date()  { self.as_date() as f64 }
        else { 0.0 }
    }

    #[inline] pub fn as_date_millis(&self) -> i64 { self.as_date() }

    pub fn has(&self, item: Self) -> bool {
        match self.tag() {
            Tag::Array  => { let arc = self.as_array(); arc.read().elements.contains(&item) }
            Tag::Set    => { let arc = self.as_set(); arc.read().elements.contains(&item) }
            Tag::Map    => { let arc = self.as_map(); arc.read().elements.iter().any(|(k, _)| *k == item) }
            Tag::String => {
                let s = self.as_string();
                let item_s = item.to_string();
                String::from_utf8_lossy(&s.data).contains(&item_s)
            }
            _ => false,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
