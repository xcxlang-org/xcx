use cranelift::prelude::*;
use std::collections::{HashSet, HashMap};
use crate::jit::nan_ops::{VALUE_SIZE};


#[derive(Clone, Copy)]
pub struct SlotVars {
    pub bits_var: Variable,
    pub tag_var:  Variable,
}

pub fn executor_field_offsets() -> (u32, u32) {
    let dummy = std::mem::MaybeUninit::<crate::vm::core::executor::Executor>::uninit();
    let base_ptr = dummy.as_ptr() as usize;
    let depth_ptr = unsafe { &(*dummy.as_ptr()).call_depth as *const _ as usize };
    let sptr = unsafe { &(*dummy.as_ptr()).stack_ptr as *const _ as usize };
    ((depth_ptr - base_ptr) as u32, (sptr - base_ptr) as u32)
}

pub struct CodegenCtx<'a, 'b> {
    pub b: &'a mut cranelift_frontend::FunctionBuilder<'b>,

    pub out_ptr: Value,
    /// Pointer to the locals array in the executor stack (each slot = VALUE_SIZE bytes).
    pub locals_ptr: Value,
    /// Pointer to the globals array (each slot = VALUE_SIZE bytes).
    pub globals_ptr: Value,
    /// Pointer to the constants pool (each slot = VALUE_SIZE bytes).
    pub consts_ptr: Value,
    pub vm_ptr: Value,
    pub executor_ptr: Value,
    pub shutdown_ptr: Value,

    pub start_ip: usize,
    pub max_locals: usize,

    /// Cranelift variable for each register slot 0..256.
    slots: [Option<SlotVars>; 256],

    /// Type annotations from the pre-analysis pass, per register per instruction.
    pub reg_types_per_ip: Vec<[crate::vm::opcode::TypeTag; 256]>,
    /// Current instruction's type annotations.
    pub reg_types: [crate::vm::opcode::TypeTag; 256],

    /// Which registers have been written and need tracking.
    pub used_registers: [u64; 4],
    /// Which registers have in-flight values not yet stored to memory.
    pub dirty_registers: [u64; 4],

    pub created_blocks: Vec<Block>,
    pub global_int_regs: HashSet<u32>,
    pub bc_blocks: HashMap<usize, Block>,

    pub global_vars: HashMap<u32, (Variable, Variable)>,

    pub self_func_idx: u32,
    pub self_func_ref: Option<cranelift::codegen::ir::FuncRef>,
    pub uses_heap: bool,
    pub current_ip: usize,
    pub register_const: [Option<i64>; 256],
    pub known_types: [crate::vm::opcode::TypeTag; 256],
    pub is_inner_func: bool,
    pub call_depth_offset: u32,
    pub stack_ptr_offset: u32,
    pub functions: Option<&'a [std::sync::Arc<crate::vm::opcode::Chunk>]>,
    pub non_ptr_regs: HashSet<u8>,
    pub may_contain_ptr: Vec<[u64; 4]>,
    pub defined_locals: [bool; 256],
    /// Registers currently holding an un-owned (inc-elided) copy of a global
    /// loaded for an upcoming specialized MethodCall receiver. Consumers must
    /// skip the matching dst-dec_ref; `def_local` clears the bit.
    pub unowned_recv_regs: [bool; 256],
}


impl<'a, 'b> CodegenCtx<'a, 'b> {
    pub fn new(
        b: &'a mut cranelift_frontend::FunctionBuilder<'b>,
        out_ptr: Value,
        locals_ptr: Value,
        globals_ptr: Value,
        consts_ptr: Value,
        vm_ptr: Value,
        executor_ptr: Value,
        shutdown_ptr: Value,
        start_ip: usize,
        max_locals: usize,
        bc_blocks: HashMap<usize, Block>,
        self_func_idx: u32,
        self_func_ref: Option<cranelift::codegen::ir::FuncRef>,
        call_depth_offset: u32,
        stack_ptr_offset: u32,
    ) -> Self {
        let mut slots: [Option<SlotVars>; 256] = [None; 256];
        for i in 0..max_locals.min(256) {
            let bits_var = b.declare_var(types::I64);
            let tag_var  = b.declare_var(types::I64);
            slots[i] = Some(SlotVars { bits_var, tag_var });
        }

        Self {
            b,
            out_ptr,
            locals_ptr,
            globals_ptr,
            consts_ptr,
            vm_ptr,
            executor_ptr,
            shutdown_ptr,
            start_ip,
            max_locals,
            slots,
            reg_types_per_ip: Vec::new(),
            reg_types: [crate::vm::opcode::TypeTag::Unknown; 256],
            known_types: [crate::vm::opcode::TypeTag::Unknown; 256],
            used_registers: [0; 4],
            dirty_registers: [0; 4],
            created_blocks: Vec::new(),
            global_int_regs: HashSet::new(),
            bc_blocks,
            global_vars: HashMap::new(),
            self_func_idx,
            self_func_ref,
            uses_heap: false,
            current_ip: start_ip,
            register_const: [None; 256],
            is_inner_func: false,
            call_depth_offset,
            stack_ptr_offset,
            functions: None,
            non_ptr_regs: HashSet::new(),
            may_contain_ptr: Vec::new(),
            defined_locals: [false; 256],
            unowned_recv_regs: [false; 256],
        }
    }


    pub fn set_functions(&mut self, functions: &'a [std::sync::Arc<crate::vm::opcode::Chunk>]) {
        self.functions = Some(functions);
    }

    pub fn create_block(&mut self) -> Block {
        let blk = self.b.create_block();
        self.created_blocks.push(blk);
        blk
    }

    pub fn set_reg_types_per_ip(&mut self, types: Vec<[crate::vm::opcode::TypeTag; 256]>) {
        self.reg_types_per_ip = types;
        if self.start_ip < self.reg_types_per_ip.len() {
            self.reg_types = self.reg_types_per_ip[self.start_ip];
        }
    }

    pub fn update_current_reg_types(&mut self, ip: usize) {
        self.current_ip = ip;
        if ip < self.reg_types_per_ip.len() {
            self.reg_types = self.reg_types_per_ip[ip];
            self.known_types = self.reg_types_per_ip[ip];
        }
    }

    pub fn set_global_int_regs(&mut self, ints: HashSet<u32>) {
        self.global_int_regs = ints;
    }

    pub fn set_non_ptr_regs(&mut self, regs: HashSet<u8>) {
        self.non_ptr_regs = regs;
    }

    pub fn set_may_contain_ptr(&mut self, mcp: Vec<[u64; 4]>) {
        self.may_contain_ptr = mcp;
    }

    pub fn emit_halt_if_errors(&mut self, symbols: &super::symbols::ImportedSymbols) {
        let err_chk = self.b.ins().call(symbols.xcx_jit_has_errors, &[self.executor_ptr]);
        let has_errs = self.b.inst_results(err_chk)[0];
        
        let halt_blk = self.create_block();
        let next_blk = self.create_block();
        let zero = self.b.ins().iconst(cranelift::prelude::types::I32, 0);
        let cond = self.b.ins().icmp(cranelift::prelude::IntCC::NotEqual, has_errs, zero);
        self.b.ins().brif(cond, halt_blk, &[], next_blk, &[]);

        self.b.switch_to_block(halt_blk);
        let sys_status = self.b.ins().iconst(cranelift::prelude::types::I32, 1);
        self.b.ins().return_(&[sys_status]);

        self.b.switch_to_block(next_blk);
    }

    pub fn global_is_int(&self, idx: u32) -> bool {
        self.global_int_regs.contains(&idx)
    }

    pub fn get_reg_type(&self, r: usize) -> crate::vm::opcode::TypeTag {
        let ty = self.known_types[r];
        if ty != crate::vm::opcode::TypeTag::Unknown {
            ty
        } else {
            self.reg_types[r]
        }
    }

    pub fn preload_globals(&mut self, globals: &[u32]) {
        for &idx in globals {
            self.ensure_global_var(idx);
        }
    }

    pub fn get_def_reg_type(&self, r: usize) -> crate::vm::opcode::TypeTag {
        let next_ip = self.current_ip + 1;
        if self.reg_types_per_ip.is_empty() {
            crate::vm::opcode::TypeTag::Unknown
        } else if next_ip < self.reg_types_per_ip.len() {
            self.reg_types_per_ip[next_ip][r.min(255)]
        } else {
            crate::vm::opcode::TypeTag::Unknown
        }
    }

    /// Returns the (bits, tag) pair for a local register directly.
    pub fn use_local(&mut self, reg: u8) -> (Value, Value) {
        let r = reg as usize;
        self.mark_used(r);
        self.use_slot(r, self.locals_ptr, (r as i64) * (VALUE_SIZE as i64))
    }

    /// Defines a local register with a (bits, tag) pair.
    pub fn def_local(&mut self, reg: u8, bits: Value, tag: Value) {
        let r = reg as usize;
        self.mark_used(r);
        self.mark_dirty(r);
        self.defined_locals[r] = true;
        self.unowned_recv_regs[r] = false;
        let slots = self.ensure_slot(r);

        let ty = self.get_def_reg_type(r);

        self.b.def_var(slots.bits_var, bits);
        self.b.def_var(slots.tag_var, tag);
        self.register_const[r] = None;
        self.known_types[r] = ty;
    }



    /// Defines a local register directly with a packed quiet-NaN Cranelift Value.
    pub fn def_local_nanboxed(&mut self, reg: u8, val: Value) {
        let r = reg as usize;
        self.mark_used(r);
        self.mark_dirty(r);
        self.defined_locals[r] = true;
        let slots = self.ensure_slot(r);
        
        let ty = self.get_def_reg_type(r);
        let (bits, tag) = match ty {
            crate::vm::opcode::TypeTag::Int => {
                let b = super::nan_ops::unpack_int(self.b, val);
                let t = self.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
                (b, t)
            }
            crate::vm::opcode::TypeTag::Bool => {
                let b = super::nan_ops::unpack_bool(self.b, val);
                let t = self.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                (b, t)
            }
            _ => super::nan_ops::unpack_value(self.b, val)
        };
        
        self.b.def_var(slots.bits_var, bits);
        self.b.def_var(slots.tag_var, tag);
        self.register_const[r] = None;
        self.known_types[r] = ty;
    }

    // --- Globals ---

    pub fn ensure_global_var(&mut self, idx: u32) -> (Variable, Variable) {
        if let Some(&v) = self.global_vars.get(&idx) {
            v
        } else {
            let bits_var = self.b.declare_var(types::I64);
            let tag_var  = self.b.declare_var(types::I64);
            self.global_vars.insert(idx, (bits_var, tag_var));
            let offset = (idx as i64) * (VALUE_SIZE as i64);
            let (bits, tag) = self.load_value_from(self.globals_ptr, offset);
            self.b.def_var(bits_var, bits);
            self.b.def_var(tag_var, tag);
            (bits_var, tag_var)
        }
    }

    pub fn use_global(&mut self, idx: u32) -> (Value, Value) {
        let (bits_var, tag_var) = self.ensure_global_var(idx);
        let bits = self.b.use_var(bits_var);
        let tag = self.b.use_var(tag_var);
        if self.global_is_int(idx) {
            use crate::vm::value::TAG_INT;
            let int_tag = self.b.ins().iconst(types::I64, TAG_INT as i64);
            (bits, int_tag)
        } else {
            (bits, tag)
        }
    }

    pub fn def_global(&mut self, idx: u32, bits: Value, tag: Value) {
        let (bv, tv) = self.ensure_global_var(idx);
        self.b.def_var(bv, bits);
        self.b.def_var(tv, tag);
    }

    pub fn spill_globals(&mut self) {
        let vars: Vec<(u32, (Variable, Variable))> = self.global_vars.iter()
            .map(|(&idx, &vars)| (idx, vars))
            .collect();
        for (idx, (bv_var, tv_var)) in vars {
            let bits = self.b.use_var(bv_var);
            let tag  = self.b.use_var(tv_var);
            let (bv, tv) = if self.global_is_int(idx) {
                use crate::vm::value::TAG_INT;
                let int_tag = self.b.ins().iconst(types::I64, TAG_INT as i64);
                (bits, int_tag)
            } else {
                (bits, tag)
            };
            let offset = (idx as i64) * (VALUE_SIZE as i64);
            self.store_value_to(self.globals_ptr, offset, bv, tv);
        }
    }

    // --- Constants ---

    /// Loads a constant from the constants pool by index.
    pub fn load_const(&mut self, idx: u32) -> (Value, Value) {
        let offset = (idx as i64) * (VALUE_SIZE as i64);
        self.load_value_from(self.consts_ptr, offset)
    }

    // --- Preload / Spill ---

    pub fn preload_locals(&mut self, locals_to_load: &[u8], needs_init: &HashSet<u8>) {
        for &reg in locals_to_load {
            let r = reg as usize;
            if r >= self.max_locals { continue; }
            if !needs_init.contains(&reg) {
                continue;
            }
            self.mark_used(r);
            self.defined_locals[r] = true;
            
            let offset = (r as i64) * (VALUE_SIZE as i64);
            let (bits, tag) = self.load_value_from(self.locals_ptr, offset);
            
            let slots = self.ensure_slot(r);
            self.b.def_var(slots.bits_var, bits);
            self.b.def_var(slots.tag_var, tag);
        }
    }



    /// Spills all dirty registers back to the locals array in memory.
    pub fn spill_all(&mut self) {
        for i_idx in 0..4usize {

            let mut bits = self.dirty_registers[i_idx];
            while bits != 0 {
                let bit = bits.trailing_zeros() as usize;
                let r = i_idx * 64 + bit;
                bits &= !(1 << bit);
                if r >= self.max_locals { continue; }
                if let Some(slot) = self.slots[r] {
                    let bits = self.b.use_var(slot.bits_var);
                    let tag  = self.b.use_var(slot.tag_var);
                    let ty = self.get_reg_type(r);
                    let (bv, tv) = match ty {
                        crate::vm::opcode::TypeTag::Int => {
                            let t = self.b.ins().iconst(types::I64, crate::vm::value::TAG_INT as i64);
                            (bits, t)
                        }
                        crate::vm::opcode::TypeTag::Bool => {
                            let t = self.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                            (bits, t)
                        }
                        crate::vm::opcode::TypeTag::Float => {
                            let t = self.b.ins().iconst(types::I64, crate::vm::value::TAG_FLOAT as i64);
                            (bits, t)
                        }
                        _ => (bits, tag)
                    };
                    let offset = (r as i64) * (VALUE_SIZE as i64);
                    self.store_value_to(self.locals_ptr, offset, bv, tv);

                }
            }
            self.dirty_registers[i_idx] = 0;
        }
        self.spill_globals();
    }


    pub fn reload_globals(&mut self) {
        let vars: Vec<(u32, (Variable, Variable))> = self.global_vars.iter()
            .map(|(&idx, &vars)| (idx, vars))
            .collect();
        for (idx, (bv, tv)) in vars {
            let offset = (idx as i64) * (VALUE_SIZE as i64);
            let (bits, tag) = self.load_value_from(self.globals_ptr, offset);
            self.b.def_var(bv, bits);
            self.b.def_var(tv, tag);
        }
    }

    pub fn reload_local(&mut self, reg: u8) {
        let r = reg as usize;
        self.mark_used(r);
        self.mark_dirty(r);
        self.defined_locals[r] = true;
        self.known_types[r] = crate::vm::opcode::TypeTag::String;
        if let Some(slot) = self.slots[r] {
            let offset = (r as i64) * (VALUE_SIZE as i64);
            let (bits, tag) = self.load_value_from(self.locals_ptr, offset);
            self.b.def_var(slot.bits_var, bits);
            self.b.def_var(slot.tag_var, tag);
        }
    }


    pub fn sync_for_jump(&mut self) {
    }



    pub fn clear_block_state(&mut self, keep_consts: bool) {
        if !keep_consts {
            self.register_const = [None; 256];
        }
    }

    /// Decrements the refcount of every live heap value in used registers.
    pub fn cleanup_all(
        &mut self,
        symbols: &super::symbols::ImportedSymbols,
        skip_reg: Option<u8>,
    ) {
        if !self.uses_heap { return; }
        for i_idx in 0..4usize {
            let mut bits = self.used_registers[i_idx];
            while bits != 0 {
                let bit = bits.trailing_zeros() as usize;
                let r = i_idx * 64 + bit;
                bits &= !(1 << bit);
                if r >= self.max_locals { continue; }
                if let Some(s) = skip_reg {
                    if r == s as usize { continue; }
                }
                if !self.defined_locals[r] { continue; }
                if self.is_known_non_ptr(r) || self.reg_is_never_ptr(r) { continue; }

                let val = self.use_slot(r, self.locals_ptr, (r as i64) * (VALUE_SIZE as i64));
                super::nan_ops::emit_conditional_dec_ref(self, symbols, val.0, val.1);
            }
        }
    }

    pub fn should_skip_dec_ref(&self, reg: u8) -> bool {
        if !self.defined_locals[reg as usize] {
            return true;
        }
        self.is_known_non_ptr(reg as usize)
            || self.reg_is_never_ptr(reg as usize)
            || self.non_ptr_regs.contains(&reg)
            || (self.current_ip < self.may_contain_ptr.len()
                && (self.may_contain_ptr[self.current_ip][(reg / 64) as usize] & (1u64 << (reg % 64))) == 0)
    }


    // --- Helpers ---

    fn mark_used(&mut self, r: usize) {
        self.used_registers[r / 64] |= 1 << (r % 64);
    }

    fn mark_dirty(&mut self, r: usize) {
        self.dirty_registers[r / 64] |= 1 << (r % 64);
    }

    fn ensure_slot(&mut self, r: usize) -> SlotVars {
        if let Some(s) = self.slots[r] {
            return s;
        }
        let bits_var = self.b.declare_var(types::I64);
        let tag_var  = self.b.declare_var(types::I64);
        let s = SlotVars { bits_var, tag_var };
        self.slots[r] = Some(s);
        s
    }

    fn use_slot(&mut self, r: usize, base_ptr: Value, offset: i64) -> (Value, Value) {
        if let Some(slot) = self.slots[r] {
            return (self.b.use_var(slot.bits_var), self.b.use_var(slot.tag_var));
        }
        let (bits, tag) = self.load_value_from(base_ptr, offset);
        
        let slot = self.ensure_slot(r);
        self.b.def_var(slot.bits_var, bits);
        self.b.def_var(slot.tag_var, tag);
        (bits, tag)
    }

    fn load_value_from(&mut self, base: Value, offset: i64) -> (Value, Value) {
        let bits = self.b.ins().load(types::I64, MemFlags::trusted(), base, offset as i32);
        let tag  = self.b.ins().load(types::I64, MemFlags::trusted(), base, (offset + 8) as i32);
        (bits, tag)
    }

    fn store_value_to(&mut self, base: Value, offset: i64, bits: Value, tag: Value) {
        self.b.ins().store(MemFlags::trusted(), bits, base, offset as i32);
        self.b.ins().store(MemFlags::trusted(), tag,  base, (offset + 8) as i32);
    }

    /// True if a register is known to hold a non-pointer value (int, float, or bool)
    /// based on the type analysis results.
    pub fn is_known_non_ptr(&self, r: usize) -> bool {
        use crate::vm::opcode::TypeTag;
        matches!(
            self.reg_types[r.min(255)],
            TypeTag::Int | TypeTag::Float | TypeTag::Bool
        )
    }

    /// True if a register is never inferred as a heap pointer type (e.g. it only ever holds
    /// Int, Float, Bool or Unknown, meaning it cannot hold an Array or Set, etc.)
    /// throughout the entire block compiled by this JIT context.
    pub fn reg_is_never_ptr(&self, r: usize) -> bool {
        use crate::vm::opcode::TypeTag;
        let reg_idx = r.min(255);
        if self.reg_types_per_ip.is_empty() {
            return false;
        }
        for types_at_ip in &self.reg_types_per_ip {
            match types_at_ip[reg_idx] {
                TypeTag::Int | TypeTag::Float | TypeTag::Bool => {}
                _ => return false,
            }
        }
        true
    }



    /// Invokes an FFI function that returns a Value.
    /// Handles the Windows ABI by automatically allocating a 16-byte StackSlot,
    /// passing it as the first argument (out_ptr), and then loading the (bits, tag) result
    /// and packing it to a single quiet-NaN.
    pub fn call_ffi_value(&mut self, func: cranelift::codegen::ir::FuncRef, args: &[Value]) -> (Value, Value) {
        let slot_data = cranelift::codegen::ir::StackSlotData::new(cranelift::codegen::ir::StackSlotKind::ExplicitSlot, 16, 8);
        let slot = self.b.create_sized_stack_slot(slot_data);
        let out_ptr = self.b.ins().stack_addr(types::I64, slot, 0);
        
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(out_ptr);
        call_args.extend_from_slice(args);
        
        self.b.ins().call(func, &call_args);
        
        let res_bits = self.b.ins().load(types::I64, MemFlags::trusted(), out_ptr, 0);
        let res_tag  = self.b.ins().load(types::I64, MemFlags::trusted(), out_ptr, 8);
        (res_bits, res_tag)
    }
}
