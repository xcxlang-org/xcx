use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::vm::{VM, SharedContext, OpResult, Chunk, Value, OpCode, MethodKind};

pub static SHUTDOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub const RECURSION_LIMIT: usize = 800;

pub struct Executor {
    pub vm: Arc<VM>,
    pub ctx: Arc<SharedContext>,
    pub current_spans: Option<Arc<Vec<crate::error::Span>>>,
    pub fiber_yielded: bool,
    pub terminal_raw_enabled: bool,
    pub fiber_next_ip: usize,
    pub current_bytecode_ptr: usize,
    pub stack: Vec<Value>,
    pub stack_ptr: usize,
    pub call_depth: usize,
    pub in_fiber: bool,
    pub globals_raw: *mut Value,
    pub row_cache: std::collections::HashMap<usize, Vec<Value>>,
    /// Lazily materialized constant-pool names, mirroring the row_cache
    /// pattern. GetMember/SetMember/StrAppendMember, named/custom method
    /// dispatch and SetName resolve the same constant index into a String
    /// allocation on every execution; the resolved value is cached here once
    /// per index. The constant pool is immutable after compilation, so a
    /// cached name can never go stale.
    pub member_name_strings: std::collections::HashMap<u32, Arc<String>>,
    pub method_name_lists: std::collections::HashMap<u32, Option<Arc<Vec<String>>>>,
}

impl Executor {
    pub fn new(vm: Arc<VM>, ctx: Arc<SharedContext>) -> Self {
        let globals_raw = vm.globals.read().as_ptr() as *mut Value;
        debug_assert_eq!(
            vm.globals.read().len(),
            65536,
            "globals Vec must be fixed-size; reallocating would invalidate globals_raw"
        );


        let stack_size = if vm.disable_jit.load(Ordering::Acquire) {
            1024 * 64 // 64K values (1MB) for fast interpreter cache locality
        } else {
            1024 * 512 // 512K values (8MB) for deeper JIT execution frames
        };

        Self {
            vm,
            ctx,
            current_spans: None,
            fiber_yielded: false,
            terminal_raw_enabled: false,
            fiber_next_ip: 0,
            current_bytecode_ptr: 0,
            stack: vec![Value::from_bool(false); stack_size],
            stack_ptr: 0,
            call_depth: 0,
            in_fiber: false,
            globals_raw,
            row_cache: std::collections::HashMap::new(),
            member_name_strings: std::collections::HashMap::new(),
            method_name_lists: std::collections::HashMap::new(),
        }
    }

    #[inline(always)]
    unsafe fn dispatch_jit_call(
        &mut self,
        jit_ptr: *mut std::ffi::c_void,
        locals_start: usize,
        vm_arc: &Arc<VM>,
        old_spans: Option<Arc<Vec<crate::error::Span>>>,
        old_stack_ptr: usize,
    ) -> OpResult {
        let _guard = crate::vm::core::vm::ActiveVmGuard::new(Arc::as_ptr(vm_arc) as *const crate::vm::VM);
        let jit_fn: crate::jit::abi::MethodJitFunction = unsafe { std::mem::transmute(jit_ptr) };

        let globals_ptr = self.globals_raw;
        let consts_ptr = self.ctx.constants.as_ptr() as *const Value;
        let vm_ptr = Arc::as_ptr(vm_arc) as *mut crate::vm::VM;
        let shutdown_ptr = &SHUTDOWN as *const std::sync::atomic::AtomicBool as *const bool;
        let locals_ptr = unsafe { self.stack.as_mut_ptr().add(locals_start) };
        
        let mut out_val = Value::from_bool(false);
        let status = unsafe { jit_fn(&mut out_val as *mut Value, locals_ptr, globals_ptr, consts_ptr, vm_ptr, self, shutdown_ptr) };

        self.current_spans = old_spans;
        self.stack_ptr = old_stack_ptr;
        self.call_depth -= 1;

        if status == 1 {
            OpResult::Halt
        } else {
            if out_val.is_ptr() { unsafe { out_val.inc_ref(); } }
            OpResult::Return(Some(out_val))
        }
    }

    pub fn current_span_info(&self, ip: usize) -> String {
        if let Some(spans) = &self.current_spans {
            if ip > 0 && ip <= spans.len() {
                let s = &spans[ip - 1];
                return format!(" [line: {}, col: {}]", s.line, s.col);
            }
        }
        "".to_string()
    }

    /// Unsynchronized access to the fixed-size globals array. This mirrors the
    /// JIT access model exactly: the Vec is allocated once in VM::new with a
    /// fixed 65536 slots and is only ever indexed, never resized, so the raw
    /// pointer cached in Executor::new stays valid for the executor's lifetime.
    #[inline(always)]
    pub(crate) unsafe fn get_global_raw(&self, idx: usize) -> Value {
        if idx >= 65536 {
            return Value::from_bool(false);
        }
        unsafe { *self.globals_raw.add(idx) }
    }

    /// Write side of `get_global_raw`, preserving the refcount discipline of
    /// `VM::set_global` (increment the incoming pointer value, decrement the
    /// displaced one) without taking the globals lock.
    #[inline(always)]
    pub(crate) unsafe fn set_global_raw(&mut self, idx: usize, val: Value) {
        if idx >= 65536 {
            return;
        }
        unsafe {
            if val.is_ptr() { val.inc_ref(); }
            let slot = self.globals_raw.add(idx);
            let old = *slot;
            *slot = val;
            if old.is_ptr() { old.dec_ref(); }
        }
    }

    /// Resolves a constant-pool index into its String form once, then serves
    /// further lookups from the per-executor cache (same lifetime pattern as
    /// row_cache). Returns an owned Arc so no borrow of the executor outlives
    /// the call.
    #[inline]
    pub(crate) fn const_string(&mut self, idx: u32) -> Arc<String> {
        if let Some(s) = self.member_name_strings.get(&idx) {
            return s.clone();
        }
        let s = Arc::new(self.ctx.constants[idx as usize].to_string());
        self.member_name_strings.insert(idx, s.clone());
        s
    }

    /// Cached `Option` form of a string-array constant used by named method
    /// dispatch; `None` is cached too, so a non-array constant pays once.
    #[inline]
    pub(crate) fn const_name_list(&mut self, idx: u32) -> Option<Arc<Vec<String>>> {
        if let Some(v) = self.method_name_lists.get(&idx) {
            return v.clone();
        }
        let val = self.ctx.constants[idx as usize];
        let mut names_vec = Vec::new();
        if val.is_array() {
            let arr = val.as_array();
            let arr_rd = arr.read();
            for v in arr_rd.elements.iter() {
                names_vec.push(v.to_string());
            }
        }
        let resolved = if names_vec.is_empty() { None } else { Some(Arc::new(names_vec)) };
        self.method_name_lists.insert(idx, resolved.clone());
        resolved
    }

    #[inline(always)]
    fn prepare_frame(
        &mut self,
        chunk: &Chunk,
        args: &[Value],
        vm_arc: &Arc<VM>,
        error_context: &str,
    ) -> Result<(usize, Option<Arc<Vec<crate::error::Span>>>, usize), ()> {
        let old_spans = self.current_spans.replace(chunk.spans.clone());
        let old_stack_ptr = self.stack_ptr;
        let locals_start = self.stack_ptr;
        self.stack_ptr += chunk.max_locals;

        if self.stack_ptr > self.stack.len() {
            let err_msg = format!("ERROR halt: XCX VM Stack Overflow{}\n", error_context);
            crate::runtime::builtin::io::eprint_buffered(&err_msg);
            vm_arc.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.stack_ptr = old_stack_ptr;
            self.current_spans = old_spans;
            return Err(());
        }

        let stack_ptr_raw = self.stack.as_mut_ptr();
        let limit = std::cmp::min(args.len(), chunk.max_locals);
        for i in 0..limit {
            let v = args[i];
            if v.is_ptr() { unsafe { v.inc_ref(); } }
            unsafe { *stack_ptr_raw.add(locals_start + i) = v; }
        }
        for i in limit..chunk.max_locals {
            unsafe { *stack_ptr_raw.add(locals_start + i) = Value::from_bool(false); }
        }

        Ok((locals_start, old_spans, old_stack_ptr))
    }

    #[inline(always)]
    fn check_jit_warmup(
        &self,
        chunk: &Arc<Chunk>,
        func_idx: u32,
        vm_arc: &Arc<VM>,
    ) -> *mut std::ffi::c_void {
        let mut jit_ptr = chunk.jit_ptr.load(Ordering::Acquire);
        if !vm_arc.disable_jit.load(Ordering::Acquire) && jit_ptr.is_null() {
            let count = chunk.call_count.fetch_add(1, Ordering::Relaxed) + 1;
            if count == vm_arc.jit_threshold as usize {
                let mut jit = vm_arc.jit.lock();
                let func_id_idx = chunk.bytecode.as_ptr() as usize;
                let func_name = chunk.name.clone();
                match jit.compile_method(func_id_idx, func_idx, chunk, &self.ctx.constants, &self.ctx.functions, &func_name) {
                    Ok(ptr) => {
                        jit_ptr = ptr as *mut std::ffi::c_void;
                        chunk.jit_ptr.store(jit_ptr, Ordering::Release);
                    }
                    Err(e) => {
                        eprintln!("[XCX] JIT disabled: code finalization failed for `{func_name}`: {e}. Falling back to the interpreter.");
                        vm_arc.disable_jit.store(true, Ordering::Release);
                    }
                }
            }
        }
        jit_ptr
    }

    #[inline(always)]
    fn cleanup_frame(
        &mut self,
        locals_start: usize,
        chunk: &Chunk,
        old_spans: Option<Arc<Vec<crate::error::Span>>>,
        old_stack_ptr: usize,
        old_bytecode_ptr: usize,
    ) {
        self.current_spans = old_spans;
        self.current_bytecode_ptr = old_bytecode_ptr;
        let locals_cleanup = unsafe { std::slice::from_raw_parts_mut(self.stack.as_mut_ptr().add(locals_start), chunk.max_locals) };
        for v in locals_cleanup { unsafe { v.dec_ref(); } }
        self.stack_ptr = old_stack_ptr;
    }

    pub fn run_frame(
        &mut self,
        chunk: Arc<Chunk>,
        params: &[Value],
        vm_arc: &Arc<VM>
    ) -> Option<Value> {
        let _guard = crate::vm::core::vm::ActiveVmGuard::new(Arc::as_ptr(vm_arc) as *const crate::vm::VM);
        let (locals_start, old_spans, old_stack_ptr) = match self.prepare_frame(&chunk, params, vm_arc, "") {
            Ok(res) => res,
            Err(()) => return Some(Value::from_bool(false)),
        };

        let chunk_bytecode_ptr = chunk.bytecode.as_ptr() as usize;
        let old_bytecode_ptr = std::mem::replace(&mut self.current_bytecode_ptr, chunk_bytecode_ptr);

        let jit_ptr = self.check_jit_warmup(&chunk, u32::MAX, vm_arc);

        if !jit_ptr.is_null() {
            self.call_depth += 1;
            let res = unsafe { self.dispatch_jit_call(jit_ptr, locals_start, vm_arc, old_spans.clone(), old_stack_ptr) };
            self.current_bytecode_ptr = old_bytecode_ptr;
            return match res {
                OpResult::Halt => None,
                OpResult::Return(v) => v,
                _ => None,
            };
        }

        let mut ip = 0;
        let ores = {
            let lp = self.stack.as_mut_ptr();
            let locals = unsafe { std::slice::from_raw_parts_mut(lp.add(locals_start), chunk.max_locals) };
            self.execute_bytecode_inner(&chunk.bytecode, &mut ip, locals, vm_arc)
        };

        let res = match ores {
            OpResult::Return(v) => v,
            _ => None,
        };

        self.cleanup_frame(locals_start, &chunk, old_spans, old_stack_ptr, old_bytecode_ptr);
        res
    }

    pub fn handle_call(
        &mut self,
        func_idx: u32,
        chunk: Arc<Chunk>,
        args: &[Value],
        vm_arc: &Arc<VM>,
    ) -> OpResult {
        self.handle_call_inner(Some(func_idx), chunk, args, vm_arc, " in handle_call")
    }

    pub fn handle_call_no_jit(
        &mut self,
        chunk: Arc<Chunk>,
        args: &[Value],
        vm_arc: &Arc<VM>,
    ) -> OpResult {
        let _guard = crate::vm::core::vm::ActiveVmGuard::new(Arc::as_ptr(vm_arc) as *const crate::vm::VM);
        self.handle_call_inner(None, chunk, args, vm_arc, " in handle_call_no_jit")
    }

    fn handle_call_inner(
        &mut self,
        func_idx: Option<u32>,
        chunk: Arc<Chunk>,
        args: &[Value],
        vm_arc: &Arc<VM>,
        loc: &'static str,
    ) -> OpResult {
        if self.call_depth >= RECURSION_LIMIT {
            let err = format!("ERROR halt: Recursion limit exceeded ({} frames)\n", RECURSION_LIMIT);
            crate::runtime::builtin::io::eprint_buffered(&err);
            return OpResult::Halt;
        }
        self.call_depth += 1;

        let (locals_start, old_spans, old_stack_ptr) = match self.prepare_frame(&chunk, args, vm_arc, loc) {
            Ok(res) => res,
            Err(()) => {
                self.call_depth -= 1;
                return OpResult::Halt;
            }
        };

        let jit_ptr = if let Some(f_idx) = func_idx {
            self.check_jit_warmup(&chunk, f_idx, vm_arc)
        } else {
            chunk.jit_ptr.load(Ordering::Acquire)
        };

        if !jit_ptr.is_null() {
            return unsafe { self.dispatch_jit_call(jit_ptr, locals_start, vm_arc, old_spans, old_stack_ptr) };
        }

        let callee_bptr = chunk.bytecode.as_ptr() as usize;
        let old_bptr = std::mem::replace(&mut self.current_bytecode_ptr, callee_bptr);

        let mut ip = 0;
        let ores = {
            let lp = self.stack.as_mut_ptr();
            let locals = unsafe { std::slice::from_raw_parts_mut(lp.add(locals_start), chunk.max_locals) };
            self.execute_bytecode_inner(&chunk.bytecode, &mut ip, locals, vm_arc)
        };

        self.cleanup_frame(locals_start, &chunk, old_spans, old_stack_ptr, old_bptr);
        self.call_depth -= 1;

        if func_idx.is_some() {
            match ores {
                OpResult::Return(v) => OpResult::Return(v),
                OpResult::Halt      => OpResult::Halt,
                _                   => OpResult::Continue,
            }
        } else {
            ores
        }
    }

    pub(crate) fn execute_bytecode_inner(
        &mut self,
        bytecode: &[OpCode],
        ip: &mut usize,
        locals: &mut [Value],
        vm_arc: &Arc<VM>,
    ) -> OpResult {
        // The shutdown flag lives on a shared cache line; polling it every
        // instruction costs a load the CPU cannot cache across iterations.
        // A 1 us granularity (1024 ops) is still far below human perception
        // and below any meaningful Ctrl-C latency for scripts.
        let mut shutdown_poll: u32 = 0;
        while *ip < bytecode.len() {
            shutdown_poll += 1;
            if shutdown_poll >= 1024 {
                shutdown_poll = 0;
                if SHUTDOWN.load(Ordering::Relaxed) { return OpResult::Halt; }
            }

            let current_ip = *ip;
            let op = bytecode[current_ip];
            *ip += 1;

            match self.execute_step(op, locals, vm_arc, ip) {
                Some(OpResult::Continue) => {}
                Some(res) => return res,
                None => {
                    eprintln!("Unhandled opcode at IP {}: {:?}", current_ip, op);
                    return OpResult::Halt;
                }
            }
        }
        OpResult::Continue
    }


    pub unsafe fn dispatch_method(&mut self, receiver: Value, kind: u8, args: &[Value], names: Option<&[String]>) -> Result<Value, ()> {
        let kind_enum = match MethodKind::from_u8(kind) {
            Some(k) => k,
            None => {
                eprintln!("R501: Invalid method kind index: {}", kind);
                return Err(());
            }
        };
        let mut locals = [Value::from_bool(false); 256];
        let vm_arc = self.vm.clone();

        let result = {
            self.handle_method_call(0, receiver, kind_enum, args, names, 0, &mut locals, &vm_arc)
        };

        match result {
            OpResult::Continue => Ok(locals[0]),
            _ => Err(()),
        }
    }

    pub fn native_inject_table(&mut self, table_val: &Value, source_val: Value, mapping_val: &Value) -> OpResult {
        if !table_val.is_table() || !mapping_val.is_map() {
            return OpResult::Halt;
        }

        let table_rc = table_val.as_table();
        let mapping_rc = mapping_val.as_map();
        let mapping = mapping_rc.read();
        let mut table = table_rc.write();

        let items = if source_val.is_json() {
            let j_rc = source_val.as_json();
            match &j_rc.root {
                crate::vm::object::JsonVal::Array(arr) => arr.read().clone(),
                _ => vec![j_rc.root.clone()],
            }
        } else if source_val.is_map() {
            vec![crate::vm::utils::json::value_to_json(&source_val)]
        } else {
            return OpResult::Continue;
        };

        for item in items {
            if let crate::vm::object::JsonVal::Object(obj) = item {
                let mut row_vals = vec![Value::from_bool(false); table.columns.len()];
                for (i, col) in table.columns.iter().enumerate() {
                    for (tgt, src) in &mapping.elements {
                        if tgt.matches_str(&col.name) {
                            let remote_key = src.to_string();
                            let obj_read = obj.read();
                            if let Some((_, json_field)) = obj_read.iter().find(|(k, _)| k.as_str() == remote_key.as_str()) {
                                let v = crate::vm::utils::json::json_val_to_value(json_field);
                                unsafe { v.inc_ref(); }
                                row_vals[i] = v;
                            }
                            break;
                        }
                    }
                }
                table.rows.extend(row_vals);
            }
        }
        unsafe { source_val.dec_ref(); }
        OpResult::Continue
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        if crate::runtime::builtin::io::terminal::OS_RAW_ACTIVE.load(std::sync::atomic::Ordering::Acquire) {
            let _ = crossterm::terminal::disable_raw_mode();
            crate::runtime::builtin::io::terminal::OS_RAW_ACTIVE.store(false, std::sync::atomic::Ordering::Release);
        }
        for (_, cache_vec) in self.row_cache.drain() {
            for v in cache_vec {
                if v.is_row() {
                    unsafe { v.dec_ref(); }
                }
            }
        }
    }
}
