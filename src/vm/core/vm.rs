use std::sync::Arc;
use parking_lot::{RwLock, Mutex};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

use crate::vm::value::Value;
pub use crate::vm::opcode::{OpCode, Chunk};
use crate::vm::trace::Trace;

pub enum OpResult {
    Continue,
    Return(Option<Value>),
    Yield(Option<Value>),
    YieldWithTarget(u8, Option<Value>),
    Halt,
}

#[derive(Clone)]
pub struct SharedContext {
    pub constants: Arc<Vec<Value>>,
    pub functions: Arc<Vec<Arc<Chunk>>>,
    pub http_req: Option<Arc<std::sync::Mutex<Option<tiny_http::Request>>>>,
}

pub struct VM {
    pub globals: parking_lot::RwLock<Vec<Value>>,
    pub global_names: Arc<RwLock<HashMap<String, usize>>>,
    pub error_count: std::sync::atomic::AtomicUsize,
    pub traces: Arc<RwLock<HashMap<(usize, usize), Arc<RwLock<Trace>>>>>,
    pub jit: Mutex<crate::jit::JIT>,
    pub disable_jit: bool,
    pub jit_threshold: u32,
    pub start_instant: std::time::Instant,
}

impl VM {
    pub fn new() -> Self {
        let globals = vec![Value::from_bool(false); 65536];
        Self {
            globals: parking_lot::RwLock::new(globals),
            global_names: Arc::new(RwLock::new(HashMap::new())),
            error_count: std::sync::atomic::AtomicUsize::new(0),
            traces: Arc::new(RwLock::new(HashMap::new())),
            jit: Mutex::new(crate::jit::JIT::new()),
            disable_jit: false,
            jit_threshold: 50,
            start_instant: std::time::Instant::now(),
        }
    }

    pub fn run(self: &Arc<Self>, chunk: Arc<Chunk>, ctx: SharedContext, params: &[Value]) -> Option<Value> {
        if !self.disable_jit && chunk.jit_ptr.load(Ordering::Acquire).is_null() {
            let mut jit = self.jit.lock();
            let func_id_idx = chunk.bytecode.as_ptr() as usize;
            match jit.compile_method(func_id_idx, u32::MAX, &chunk, &ctx.constants, &ctx.functions, "main") {
                Ok(ptr) => { chunk.jit_ptr.store(ptr as *mut _, Ordering::Release); }
                Err(_e) => {}
            }
        }
        let ctx = Arc::new(ctx);
        let mut executor = crate::vm::core::executor::Executor::new(self.clone(), ctx);
        executor.run_frame(chunk, params, self)
    }

    pub fn get_global(&self, idx: usize) -> Value {
        if idx >= 65536 { return Value::from_bool(false); }
        self.globals.read()[idx]
    }

    pub fn set_global(&self, idx: usize, val: Value) {
        if idx >= 65536 { return; }
        if val.is_ptr() { unsafe { val.inc_ref(); } }
        let mut g = self.globals.write();
        let old = g[idx];
        g[idx] = val;
        drop(g);
        if old.is_ptr() { unsafe { old.dec_ref(); } }
    }
}

impl Drop for VM {
    fn drop(&mut self) {
        let g = self.globals.get_mut();
        for val in g.iter() {
            if val.is_ptr() { unsafe { val.dec_ref(); } }
        }
    }
}

thread_local! {
    pub static ACTIVE_VM: std::cell::Cell<*const VM> = std::cell::Cell::new(std::ptr::null());
}

pub struct ActiveVmGuard {
    old: *const VM,
}

impl ActiveVmGuard {
    pub fn new(vm: *const VM) -> Self {
        let old = ACTIVE_VM.with(|cell| {
            let old = cell.get();
            cell.set(vm);
            old
        });
        Self { old }
    }
}

impl Drop for ActiveVmGuard {
    fn drop(&mut self) {
        ACTIVE_VM.with(|cell| cell.set(self.old));
    }
}

pub fn increment_error_count() {
    ACTIVE_VM.with(|cell| {
        let ptr = cell.get();
        if !ptr.is_null() {
            unsafe {
                (*ptr).error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    });
}

