    pub fn compile_method_impl(
        &mut self,
        func_id_idx: usize,
        self_func_idx: u32,
        chunk: &Chunk,
        constants: &[VMValue],
        _name: &str,
        is_inner_func: bool,
        inner_func_id: Option<cranelift_module::FuncId>,
    ) -> Result<*const std::ffi::c_void, String> {
        self.module.clear_context(&mut self.ctx);
        
        let mut sig = self.module.make_signature();
        let ptr_type = self.module.target_config().pointer_type();
        if is_inner_func {
            for _ in 0..chunk.arity {
                sig.params.push(AbiParam::new(types::I64));
            }
            sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
            sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
            sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
            sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
            sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
            sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
            sig.returns.push(AbiParam::new(types::I64));
            sig.returns.push(AbiParam::new(types::I32));
        } else {
            sig.params.push(AbiParam::new(ptr_type)); // out_ptr
            sig.params.push(AbiParam::new(ptr_type)); // locals_ptr
            sig.params.push(AbiParam::new(ptr_type)); // globals_ptr
            sig.params.push(AbiParam::new(ptr_type)); // consts_ptr
            sig.params.push(AbiParam::new(ptr_type)); // vm_ptr
            sig.params.push(AbiParam::new(ptr_type)); // executor_ptr
            sig.params.push(AbiParam::new(ptr_type)); // shutdown_ptr
            sig.returns.push(AbiParam::new(types::I32)); // status
        }

        let func_id = if is_inner_func {
            inner_func_id.unwrap()
        } else {
            self.module.declare_function(
                &format!("method_{}", func_id_idx),
                Linkage::Export,
                &sig,
            ).map_err(|e: cranelift_module::ModuleError| e.to_string())?
        };

        self.ctx.func.signature = sig;
        
        let local_callee = if let Some(fid) = inner_func_id {
            Some(self.module.declare_func_in_func(fid, &mut self.ctx.func))
        } else if self_func_idx != u32::MAX {
            Some(self.module.declare_func_in_func(func_id, &mut self.ctx.func))
        } else {
            None
        };
        let registry = SymbolRegistry::new(&mut self.module);
        let symbols = registry.import_in_func(&mut self.module, &mut self.ctx.func);
        
        let now = std::time::Instant::now();
        let _chunk_name = chunk.name.clone();

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let mut creates_ptrs = false;
        for op in chunk.bytecode.iter() {
            match op {
                OpCode::ArrayInit { .. } | OpCode::BoolArrayInit { .. } | OpCode::SetInit { .. } | OpCode::MapInit { .. } | 
                OpCode::TableInit { .. } | OpCode::MethodCall { .. } | OpCode::MethodCallCustom { .. } |
                OpCode::SetName { .. } | OpCode::JsonParse { .. } | OpCode::DateNow { .. } |
                OpCode::JsonBind { .. } | OpCode::JsonBindLocal { .. } | OpCode::JsonInject { .. } |
                OpCode::JsonInjectLocal { .. } | OpCode::FiberCreate { .. } | OpCode::Yield { .. } |
                OpCode::YieldWithTarget { .. } | OpCode::HttpCall { .. } | OpCode::HttpRequest { .. } |
                OpCode::HttpServe { .. } | OpCode::CryptoHash { .. } | OpCode::CryptoVerify { .. } |
                OpCode::CryptoToken { .. } | OpCode::CastString { .. } | OpCode::MakeClosure { .. } |
                OpCode::GetIndex { .. } | OpCode::SetIndex { .. } | OpCode::GetMember { .. } |
                OpCode::SetMember { .. } | OpCode::DatabaseInit { .. } | OpCode::TablePushRow { .. } |
                OpCode::TableCloneSkeleton { .. } => {
                    creates_ptrs = true;
                    break;
                }
                OpCode::LoadConst { idx, .. } => {
                    if constants[*idx as usize].is_ptr() {
                        creates_ptrs = true;
                        break;
                    }
                }
                _ => {}
            }
        }
        let _pure_func = !creates_ptrs;

        let mut blocks: HashMap<usize, Block> = HashMap::new();
        
        let entry_block = b.create_block();
        blocks.insert(0, entry_block);

        for (ip, op) in chunk.bytecode.iter().enumerate() {
            let _ = ip;
            match op {
                OpCode::Jump { target } => { blocks.entry(*target as usize).or_insert_with(|| b.create_block()); }
                OpCode::JumpIfFalse { target, .. } | OpCode::JumpIfTrue { target, .. } => {
                    blocks.entry(*target as usize).or_insert_with(|| b.create_block());
                }
                OpCode::LoopNext { target, .. } | OpCode::IncLocalLoopNext { target, .. } | 
                OpCode::IncVarLoopNext { target, .. } | OpCode::ArrayLoopNext { target, .. } |
                OpCode::TableIter { target, .. } => {
                    blocks.entry(*target as usize).or_insert_with(|| b.create_block());
                }
                _ => {}
            }
        }

        b.append_block_params_for_function_params(entry_block);
        b.switch_to_block(entry_block);

        let arg_vals = b.block_params(entry_block).to_vec();
        let out_ptr = if is_inner_func {
            b.ins().iconst(types::I64, 0)
        } else {
            arg_vals[0]
        };
        let locals_ptr = if is_inner_func { arg_vals[chunk.arity] } else { arg_vals[1] };
        let globals_ptr = if is_inner_func { arg_vals[chunk.arity + 1] } else { arg_vals[2] };
        let consts_ptr = if is_inner_func { arg_vals[chunk.arity + 2] } else { arg_vals[3] };
        let vm_ptr = if is_inner_func { arg_vals[chunk.arity + 3] } else { arg_vals[4] };
        let executor_ptr = if is_inner_func { arg_vals[chunk.arity + 4] } else { arg_vals[5] };
        let shutdown_ptr = if is_inner_func { arg_vals[chunk.arity + 5] } else { arg_vals[6] };

        let bool_array_hints_early = std::collections::HashSet::new();
        let (types_at_ip, uses_heap) = analyze_chunk_types(&chunk.bytecode, constants, None, chunk.arity, self_func_idx, &bool_array_hints_early);

        {
            let call_depth_offset = {
                let dummy = std::mem::MaybeUninit::<crate::vm::core::executor::Executor>::uninit();
                let base_ptr = dummy.as_ptr() as usize;
                let depth_ptr = unsafe { &(*dummy.as_ptr()).call_depth as *const _ as usize };
                (depth_ptr - base_ptr) as u32
            };

            let mut ctx = CodegenCtx::new(
                &mut b, out_ptr, locals_ptr, globals_ptr, consts_ptr,
                vm_ptr, executor_ptr, shutdown_ptr,
                0, chunk.max_locals, blocks.clone(),
                self_func_idx, local_callee, call_depth_offset,
            );
            ctx.is_inner_func = is_inner_func;
            ctx.set_reg_types_per_ip(types_at_ip);
            ctx.uses_heap = uses_heap;
            // eprintln!("[JIT] Compiling method {} (idx {})", _name, self_func_idx);
            
            let used_locals = analyze_chunk_locals(&chunk.bytecode);
            
            let initial_types = [TypeTag::Unknown; 256];
            
            let bool_array_hints = analyze_bool_array_regs(&chunk.bytecode, constants);
            let (inferred_types, uses_heap) = analyze_chunk_types(&chunk.bytecode, constants, Some(&initial_types), chunk.arity, self_func_idx, &bool_array_hints);
            
            // Heuristic: if it's a pure math function, we can elide heap tracking.
            // A function is pure math if it never assigns a non-primitive type.
            if !uses_heap {
                // Optimization: if no heap is used, we don't need any GC cleanup.
            }
            let global_ints = analyze_global_int_regs(&chunk.bytecode, constants);
            ctx.set_global_int_regs(global_ints);
            
            ctx.set_reg_types_per_ip(inferred_types);
            ctx.uses_heap = uses_heap;
            
            if is_inner_func {
                for i in 0..chunk.arity {
                    let val = arg_vals[i];
                    ctx.def_local_nanboxed(i as u8, val);
                }
            }
            
            let mut filtered_locals = Vec::new();
            for &reg in &used_locals {
                if !is_inner_func || (reg as usize) >= chunk.arity {
                    filtered_locals.push(reg);
                }
            }
            ctx.preload_locals(&filtered_locals);



            // Use entry_block as block_0 (it's already switched-to and parameters are loaded)
            let mut terminated = false;

            for (ip, op) in chunk.bytecode.iter().enumerate() {
                if let Some(&block) = blocks.get(&ip) {
                    if ip > 0 {
                        if !terminated { 
                            ctx.sync_for_jump();
                            ctx.b.ins().jump(block, &[]); 
                        }
                        ctx.b.switch_to_block(block);
                        ctx.clear_block_state(false);
                        terminated = false;
                    }
                }
                if terminated { continue; }
                ctx.update_current_reg_types(ip);

                match *op {
                    OpCode::LoadConst { dst, idx } => {
                        emit_load_const(&mut ctx, &symbols, dst, idx);
                        let val = constants[idx as usize];
                        if val.is_int() {
                            ctx.register_const[dst as usize] = Some(val.as_i64());
                        } else {
                            ctx.register_const[dst as usize] = None;
                        }
                    }
                    OpCode::Move { dst, src } => {
                        emit_move(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::Add { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_add_int(&mut ctx, &symbols, dst, src1, src2);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_add_float(&mut ctx, &symbols, dst, src1, src2);
                        } else {
                            emit_add_poly(&mut ctx, &symbols, dst, src1, src2);
                        }
                    }
                    OpCode::Sub { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_sub_int(&mut ctx, &symbols, dst, src1, src2);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_sub_float(&mut ctx, &symbols, dst, src1, src2);
                        } else {
                            emit_sub_poly(&mut ctx, &symbols, dst, src1, src2);
                        }
                    }
                    OpCode::Mul { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_mul_int(&mut ctx, &symbols, dst, src1, src2);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_mul_float(&mut ctx, &symbols, dst, src1, src2);
                        } else {
                            emit_mul_poly(&mut ctx, &symbols, dst, src1, src2);
                        }
                    }
                    OpCode::Div { dst, src1, src2 } => {
                        emit_poly_div_mod_fast_path(&mut ctx, &symbols, dst, src1, src2, symbols.xcx_jit_div, false);
                    }
                    OpCode::Mod { dst, src1, src2 } => {
                        emit_poly_div_mod_fast_path(&mut ctx, &symbols, dst, src1, src2, symbols.xcx_jit_mod, true);
                    }
                    OpCode::Neg { dst, src } => {
                        let t = ctx.get_reg_type(src as usize);
                        if t == TypeTag::Int {
                            emit_neg_int(&mut ctx, &symbols, dst, src);
                        } else if t == TypeTag::Float {
                            emit_neg_float(&mut ctx, &symbols, dst, src);
                        } else {
                            emit_neg_poly(&mut ctx, &symbols, dst, src);
                        }
                    }
                    OpCode::IncLocal { reg } => {
                        emit_inc_local(&mut ctx, reg);
                    }
                    OpCode::GetVar { dst, idx } => {
                        emit_get_var(&mut ctx, &symbols, dst, idx);
                    }
                    OpCode::SetVar { idx, src } => {
                        emit_set_var(&mut ctx, &symbols, idx, src);
                    }
                    OpCode::Equal { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 0);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 0);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 0);
                        }
                    }
                    OpCode::NotEqual { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 1);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 1);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 1);
                        }
                    }
                    OpCode::Greater { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 2);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 2);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 2);
                        }
                    }
                    OpCode::Less { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 3);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 3);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 3);
                        }
                    }
                    OpCode::GreaterEqual { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 4);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 4);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 4);
                        }
                    }
                    OpCode::LessEqual { dst, src1, src2 } => {
                        let t1 = ctx.get_reg_type(src1 as usize);
                        let t2 = ctx.get_reg_type(src2 as usize);
                        if t1 == TypeTag::Int && t2 == TypeTag::Int {
                            emit_cmp_int(&mut ctx, &symbols, dst, src1, src2, 5);
                        } else if t1 == TypeTag::Float || t2 == TypeTag::Float {
                            emit_cmp_float(&mut ctx, &symbols, dst, src1, src2, 5);
                        } else {
                            emit_cmp_poly(&mut ctx, &symbols, dst, src1, src2, 5);
                        }
                    }
                    OpCode::IntConcat { dst, src1, src2 } => {
                        emit_int_concat(&mut ctx, &symbols, dst, src1, src2);
                    }
                    OpCode::CastInt { dst, src } => {
                        emit_cast_to_int(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::CastFloat { dst, src } => {
                        emit_cast_to_float(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::CastString { dst, src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_cast_string, &[v_bits, v_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::Print { src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        ctx.b.ins().call(symbols.xcx_jit_print, &[v_bits, v_tag]);
                    }
                    OpCode::Jump { target } => {
                        emit_jump(&mut ctx, &blocks, target, &mut terminated);
                    }
                    OpCode::JumpIfFalse { src, target } => {
                        emit_jump_if(&mut ctx, &symbols, &blocks, src, target, false);
                    }
                    OpCode::JumpIfTrue { src, target } => {
                        emit_jump_if(&mut ctx, &symbols, &blocks, src, target, true);
                    }
                    OpCode::Call { dst, func_idx, base, arg_count } => {
                        emit_call(&mut ctx, &symbols, dst, func_idx, base, arg_count);
                    }
                    OpCode::Return { src } => {
                        emit_return(&mut ctx, &symbols, Some(src), &mut terminated);
                    }
                    OpCode::ReturnVoid => {
                        emit_return(&mut ctx, &symbols, None, &mut terminated);
                    }
                    OpCode::Wait { src } => {
                        let (v_bits, _v_tag) = ctx.use_local(src);
                        let ms = ctx.b.ins().band_imm(v_bits, 0x0000_FFFF_FFFF_FFFF_i64);
                        ctx.b.ins().call(symbols.xcx_jit_wait, &[ms]);
                    }
                    OpCode::MethodCall { dst, kind, base, arg_count } => {
                        emit_method_call(&mut ctx, &symbols, dst, kind, base, arg_count);
                    }
                    OpCode::GetIndex { dst, container, index } => {
                        let (cv_bits, cv_tag) = ctx.use_local(container);
                        let (iv_bits, _iv_tag) = ctx.use_local(index);
                        let container_ty = ctx.get_reg_type(container as usize);
                        if container_ty == TypeTag::BoolArray {
                            let call = ctx.b.ins().call(symbols.xcx_jit_array_get_bool, &[cv_bits, cv_tag, iv_bits]);
                            let raw = ctx.b.inst_results(call)[0];
                            let bool_tag = ctx.b.ins().iconst(types::I64, TAG_BOOL as i64);
                            ctx.def_local(dst, raw, bool_tag);
                        } else if container_ty == TypeTag::Table {
                            let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_table_get_row, &[cv_bits, cv_tag, iv_bits]);
                            if !ctx.should_skip_dec_ref(dst) {
                                let (old_bits, old_tag) = ctx.use_local(dst);
                                emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                            }
                            ctx.def_local(dst, res_bits, res_tag);
                        } else {
                            let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_get, &[cv_bits, cv_tag, iv_bits]);
                            if !ctx.should_skip_dec_ref(dst) {
                                let (old_bits, old_tag) = ctx.use_local(dst);
                                emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                            }
                            ctx.def_local(dst, res_bits, res_tag);
                        }
                    }
                    OpCode::SetIndex { container, index, src } => {
                        let (cv_bits, cv_tag) = ctx.use_local(container);
                        let (iv_bits, _iv_tag) = ctx.use_local(index);
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        if ctx.get_reg_type(container as usize) == TypeTag::BoolArray {
                            let unpacked_bool = if ctx.get_reg_type(src as usize) == TypeTag::Bool {
                                sv_bits
                            } else {
                                super::nan_ops::unpack_bool(ctx.b, sv_bits)
                            };
                            let sv_bits_i8 = ctx.b.ins().ireduce(types::I8, unpacked_bool);
                            ctx.b.ins().call(symbols.xcx_jit_array_set_bool, &[cv_bits, cv_tag, iv_bits, sv_bits_i8]);
                        } else {
                            ctx.b.ins().call(symbols.xcx_jit_array_update, &[cv_bits, cv_tag, iv_bits, sv_bits, sv_tag]);
                        }
                    }
                    OpCode::JsonParse { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_json_parse, &[sv_bits, sv_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::DateNow { dst } => {
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_date_now, &[]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::CastBool { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_cast_int, &[sv_bits, sv_tag]);
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::LoopNext { reg, limit_reg, target } => {
                        emit_loop_next(&mut ctx, &symbols, &blocks, reg, limit_reg, target);
                    }
                    OpCode::IncLocalLoopNext { inc_reg, reg, limit_reg, target } => {
                        emit_inc_local_loop_next_opcode(&mut ctx, &symbols, &blocks, inc_reg, reg, limit_reg, target);
                    }
                    OpCode::IncVar { idx } => {
                        emit_inc_var(&mut ctx, &symbols, idx);
                    }
                    OpCode::IncVarLoopNext { g_idx, reg, limit_reg, target } => {
                        emit_inc_var_loop_next_opcode(&mut ctx, &symbols, &blocks, g_idx, reg, limit_reg, target);
                    }

                    OpCode::ArrayLoopNext { idx_reg, size_reg, target } => {
                        emit_array_loop_next_opcode(&mut ctx, &symbols, &blocks, idx_reg, size_reg, target);
                    }
                    OpCode::TableIter { tbl_reg, idx_reg, row_reg, limit_reg, target } => {
                        emit_table_iter_opcode(&mut ctx, &symbols, &blocks, tbl_reg, idx_reg, row_reg, limit_reg, target);
                    }
                    OpCode::HaltAlert { src } => {
                        emit_halt_alert(&mut ctx, &symbols, src);
                    }
                    OpCode::HaltError { src } => {
                        emit_halt_error(&mut ctx, &symbols, src, &mut terminated);
                    }
                    OpCode::HaltFatal { src } => {
                        emit_halt_fatal(&mut ctx, &symbols, src, &mut terminated);
                    }
                    OpCode::Typeof { dst, src } => {
                        let (sv_bits, sv_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_typeof, &[sv_bits, sv_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::StoreRead { dst, base } => {
                        emit_store_read(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::StoreWrite { dst, base } => {
                        emit_store_write(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::StoreAppend { dst, base } => {
                        emit_store_append(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::StoreExists { dst, base } => {
                        emit_store_exists(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::StoreDelete { dst, base } => {
                        emit_store_delete(&mut ctx, &symbols, dst, base);
                    }
                    OpCode::DatabaseInit { dst, engine_src, path_src, tables_base_reg, table_count } => {
                        emit_database_init(&mut ctx, &symbols, dst, engine_src, path_src, tables_base_reg, table_count);
                    }
                    OpCode::GetMember { dst, container, name_idx } => {
                        emit_get_member(&mut ctx, &symbols, dst, container, name_idx, constants);
                    }
                    OpCode::SetMember { container, name_idx, src } => {
                        emit_set_member(&mut ctx, &symbols, container, name_idx, src, constants);
                    }
                    OpCode::TablePushRow { tbl_reg, row_reg } => {
                        emit_table_push_row(&mut ctx, &symbols, tbl_reg, row_reg);
                    }
                    OpCode::TableCloneSkeleton { dst, src } => {
                        emit_table_clone_skeleton(&mut ctx, &symbols, dst, src);
                    }
                    OpCode::RowGet { dst, row_reg, col_idx } => {
                        emit_row_get(&mut ctx, &symbols, dst, row_reg, col_idx);
                    }
                    OpCode::EnvGet { dst, src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_env_get, &[v_bits, v_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::EnvArgs { dst } => {
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_env_args, &[]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::CryptoHash { dst, pass_src, alg_src } => {
                        let (p_bits, p_tag) = ctx.use_local(pass_src);
                        let (a_bits, a_tag) = ctx.use_local(alg_src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_crypto_hash, &[p_bits, p_tag, a_bits, a_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::CryptoVerify { dst, pass_src, hash_src, alg_src } => {
                         let (p_bits, p_tag) = ctx.use_local(pass_src);
                         let (h_bits, h_tag) = ctx.use_local(hash_src);
                         let (a_bits, a_tag) = ctx.use_local(alg_src);
                         let call = ctx.b.ins().call(symbols.xcx_jit_crypto_verify, &[p_bits, p_tag, h_bits, h_tag, a_bits, a_tag]);
                         let res_i32 = ctx.b.inst_results(call)[0];
                         let res_bits = ctx.b.ins().uextend(types::I64, res_i32);
                         let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                         if !ctx.should_skip_dec_ref(dst) {
                             let (old_bits, old_tag) = ctx.use_local(dst);
                             emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                         }
                         ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::CryptoToken { dst, len_src } => {
                        let (l_bits, l_tag) = ctx.use_local(len_src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_crypto_token, &[l_bits, l_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::RandomChoice { dst, src } => {
                        let (v_bits, v_tag) = ctx.use_local(src);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_random_choice, &[v_bits, v_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::RandomInt { dst, min, max, step, has_step } => {
                        let (min_bits, min_tag) = ctx.use_local(min);
                        let (max_bits, max_tag) = ctx.use_local(max);
                        let (step_bits, step_tag) = ctx.use_local(step);
                        let (hs_bits, _hs_tag) = ctx.use_local(has_step);
                        let hs = ctx.b.ins().ireduce(types::I8, hs_bits);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_random_int, &[min_bits, min_tag, max_bits, max_tag, step_bits, step_tag, hs]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::RandomFloat { dst, min, max, step, has_step } => {
                        let (min_bits, min_tag) = ctx.use_local(min);
                        let (max_bits, max_tag) = ctx.use_local(max);
                        let (step_bits, step_tag) = ctx.use_local(step);
                        let (hs_bits, _hs_tag) = ctx.use_local(has_step);
                        let hs = ctx.b.ins().ireduce(types::I8, hs_bits);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_random_float, &[min_bits, min_tag, max_bits, max_tag, step_bits, step_tag, hs]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetRange { dst, start, end, step, has_step } => {
                        let (start_bits, start_tag) = ctx.use_local(start);
                        let (end_bits, end_tag) = ctx.use_local(end);
                        let (step_bits, step_tag) = ctx.use_local(step);
                        let (hs_bits, _hs_tag) = ctx.use_local(has_step);
                        let hs = ctx.b.ins().ireduce(types::I8, hs_bits);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_range, &[start_bits, start_tag, end_bits, end_tag, step_bits, step_tag, hs]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::And { dst, src1, src2 } => {
                        let (b1, _t1) = ctx.use_local(src1);
                        let (b2, _t2) = ctx.use_local(src2);
                        let res_bits = ctx.b.ins().band(b1, b2);
                        let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::Or { dst, src1, src2 } => {
                        let (b1, _t1) = ctx.use_local(src1);
                        let (b2, _t2) = ctx.use_local(src2);
                        let res_bits = ctx.b.ins().bor(b1, b2);
                        let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::Not { dst, src } => {
                        let (b, _t) = ctx.use_local(src);
                        let res_bits = ctx.b.ins().bxor_imm(b, 1);
                        let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::Has { dst, src1, src2 } => {
                        let (b1, t1) = ctx.use_local(src1);
                        let (b2, t2) = ctx.use_local(src2);
                        let call = ctx.b.ins().call(symbols.xcx_jit_has, &[b1, t1, b2, t2]);
                        let res_i8 = ctx.b.inst_results(call)[0];
                        let res_bits = ctx.b.ins().uextend(types::I64, res_i8);
                        let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::ArrayInit { dst, base, count } => {
                        ctx.spill_all();
                        let offset = (base as i64) * 16;
                        let elem_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);
                        let count_val = ctx.b.ins().iconst(types::I32, count as i64);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_array_init, &[ctx.executor_ptr, elem_ptr, count_val]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::BoolArrayInit { dst } => {
                        ctx.spill_all();
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_bool_array_init, &[ctx.executor_ptr]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetInit { dst, base, count } => {
                        ctx.spill_all();
                        let offset = (base as i64) * 16;
                        let elem_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);
                        let count_val = ctx.b.ins().iconst(types::I32, count as i64);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_init, &[ctx.executor_ptr, elem_ptr, count_val]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetUnion { dst, src1, src2 } => {
                        let (s1_bits, s1_tag) = ctx.use_local(src1);
                        let (s2_bits, s2_tag) = ctx.use_local(src2);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_union, &[s1_bits, s1_tag, s2_bits, s2_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetIntersection { dst, src1, src2 } => {
                        let (s1_bits, s1_tag) = ctx.use_local(src1);
                        let (s2_bits, s2_tag) = ctx.use_local(src2);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_intersection, &[s1_bits, s1_tag, s2_bits, s2_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetDifference { dst, src1, src2 } => {
                        let (s1_bits, s1_tag) = ctx.use_local(src1);
                        let (s2_bits, s2_tag) = ctx.use_local(src2);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_difference, &[s1_bits, s1_tag, s2_bits, s2_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::SetSymDifference { dst, src1, src2 } => {
                        let (s1_bits, s1_tag) = ctx.use_local(src1);
                        let (s2_bits, s2_tag) = ctx.use_local(src2);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_set_sym_difference, &[s1_bits, s1_tag, s2_bits, s2_tag]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::MapInit { dst, base, count } => {
                        ctx.spill_all();
                        let offset = (base as i64) * 16;
                        let elem_ptr = ctx.b.ins().iadd_imm(ctx.locals_ptr, offset);
                        let count_val = ctx.b.ins().iconst(types::I32, count as i64);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_map_init, &[ctx.executor_ptr, elem_ptr, count_val]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::TableInit { dst, skeleton_idx, base, row_count, col_count } => {
                        ctx.spill_all();
                        let skel_val = ctx.b.ins().iconst(types::I32, skeleton_idx as i64);
                        let base_val = ctx.b.ins().iconst(types::I32, base as i64);
                        let rows_val = ctx.b.ins().iconst(types::I32, row_count as i64);
                        let cols_val = ctx.b.ins().iconst(types::I32, col_count as i64);
                        let (res_bits, res_tag) = ctx.call_ffi_value(symbols.xcx_jit_table_init, &[
                            skel_val, base_val, rows_val, cols_val, ctx.locals_ptr, ctx.consts_ptr
                        ]);
                        if !ctx.should_skip_dec_ref(dst) {
                            let (old_bits, old_tag) = ctx.use_local(dst);
                            emit_conditional_dec_ref(&mut ctx, &symbols, old_bits, old_tag);
                        }
                        ctx.def_local(dst, res_bits, res_tag);
                    }
                    OpCode::MethodCallCustom { dst, method_name_idx, base, arg_count } => {
                        emit_method_call_custom(&mut ctx, &symbols, dst, method_name_idx, base, arg_count, constants);
                    }
                    _ => {
                        // For Phase 3, we abort if we hit unsupported opcode in method JIT
                        return Err(format!("Unsupported opcode for compile_method: {:?}", op));
                    }
                }
            }

            if !terminated {
                ctx.spill_all();
                if is_inner_func {
                    let false_bits = ctx.b.ins().iconst(types::I64, 0);
                    let false_tag = ctx.b.ins().iconst(types::I64, crate::vm::value::TAG_BOOL as i64);
                    let res_val = super::nan_ops::pack_value(ctx.b, false_bits, false_tag);
                    let status = ctx.b.ins().iconst(types::I32, 0);
                    ctx.b.ins().return_(&[res_val, status]);
                } else {
                    let res_bits = ctx.b.ins().iconst(types::I64, 0);
                    let res_tag = ctx.b.ins().iconst(types::I64, 2); // TAG_BOOL (false)
                    ctx.b.ins().store(super::abi::trusted(), res_bits, out_ptr, 0);
                    ctx.b.ins().store(super::abi::trusted(), res_tag, out_ptr, 8);
                    let rv = ctx.b.ins().iconst(types::I32, 0);
                    ctx.b.ins().return_(&[rv]);
                }
            }

            // Seal all blocks at the end
            for (_, &block) in &blocks {
                ctx.b.seal_block(block);
            }
            for &block in &ctx.created_blocks {
                ctx.b.seal_block(block);
            }
        }
        b.finalize();

        if let Err(errors) = self.module.define_function(func_id, &mut self.ctx) {
            let _ = errors;
            return Err(format!("{:#?}", errors));
        }
        self.module.clear_context(&mut self.ctx);

        if is_inner_func {
            Ok(std::ptr::null())
        } else {
            self.module.finalize_definitions().map_err(|e: cranelift_module::ModuleError| e.to_string())?;
            let _elapsed = now.elapsed();
            Ok(self.module.get_finalized_function(func_id) as *const std::ffi::c_void)
        }
    }

