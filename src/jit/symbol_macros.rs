macro_rules! declare_jit_symbols {
    (
        $( $sym_name:ident => $sig_name:ident ),* $(,)?
    ) => {
        pub struct SymbolRegistry {
            $( pub $sym_name: FuncId, )*
        }

        pub struct ImportedSymbols {
            $( pub $sym_name: FuncRef, )*
        }

        impl SymbolRegistry {
            pub fn new(module: &mut JITModule) -> Self {
                let ptr_type = module.target_config().pointer_type();
                
                let add_val_param = |sig: &mut Signature| {
                    sig.params.push(AbiParam::new(types::I64)); // bits
                    sig.params.push(AbiParam::new(types::I64)); // tag
                };

                let mut sig_val_ret = module.make_signature();
                sig_val_ret.params.push(AbiParam::new(ptr_type));

                let mut sig_val_val_ret = module.make_signature();
                sig_val_val_ret.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_val_val_ret);

                let mut sig_val_val_val_ret = module.make_signature();
                sig_val_val_val_ret.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_val_val_val_ret);
                add_val_param(&mut sig_val_val_val_ret);

                let mut sig_val_ret_void = module.make_signature();
                add_val_param(&mut sig_val_ret_void);

                let mut sig_val_val_ret_void = module.make_signature();
                add_val_param(&mut sig_val_val_ret_void);
                add_val_param(&mut sig_val_val_ret_void);

                let mut sig_val_ret_i64 = module.make_signature();
                add_val_param(&mut sig_val_ret_i64);
                sig_val_ret_i64.returns.push(AbiParam::new(types::I64));

                let mut sig_val_ret_bool = module.make_signature();
                add_val_param(&mut sig_val_ret_bool);
                sig_val_ret_bool.returns.push(AbiParam::new(types::I8));

                let mut sig_val_val_ret_bool = module.make_signature();
                add_val_param(&mut sig_val_val_ret_bool);
                add_val_param(&mut sig_val_val_ret_bool);
                sig_val_val_ret_bool.returns.push(AbiParam::new(types::I8));

                let mut sig_random_int = module.make_signature();
                sig_random_int.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_random_int);
                add_val_param(&mut sig_random_int);
                add_val_param(&mut sig_random_int);
                sig_random_int.params.push(AbiParam::new(types::I8));

                let mut sig_random_float = module.make_signature();
                sig_random_float.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_random_float);
                add_val_param(&mut sig_random_float);
                add_val_param(&mut sig_random_float);
                sig_random_float.params.push(AbiParam::new(types::I8));

                let mut sig_pow_int = module.make_signature();
                sig_pow_int.params.push(AbiParam::new(ptr_type));
                sig_pow_int.params.push(AbiParam::new(types::I64));
                sig_pow_int.params.push(AbiParam::new(types::I64));

                let mut sig_pow_float = module.make_signature();
                sig_pow_float.params.push(AbiParam::new(ptr_type));
                sig_pow_float.params.push(AbiParam::new(types::F64));
                sig_pow_float.params.push(AbiParam::new(types::F64));

                let mut sig_call_rec = module.make_signature();
                sig_call_rec.params.push(AbiParam::new(ptr_type));
                sig_call_rec.params.push(AbiParam::new(types::I64));
                sig_call_rec.params.push(AbiParam::new(ptr_type));
                sig_call_rec.params.push(AbiParam::new(types::I8));
                sig_call_rec.params.push(AbiParam::new(ptr_type));
                sig_call_rec.params.push(AbiParam::new(ptr_type));
                sig_call_rec.params.push(AbiParam::new(ptr_type));

                let mut sig_method = module.make_signature();
                sig_method.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_method);
                sig_method.params.push(AbiParam::new(types::I8));
                sig_method.params.push(AbiParam::new(ptr_type));
                sig_method.params.push(AbiParam::new(ptr_type));
                sig_method.params.push(AbiParam::new(types::I8));
                sig_method.params.push(AbiParam::new(types::I8));
                sig_method.params.push(AbiParam::new(ptr_type));

                let mut sig_row_get = module.make_signature();
                sig_row_get.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_row_get);
                sig_row_get.params.push(AbiParam::new(types::I32));
                
                let mut sig_tbl_get_row = module.make_signature();
                sig_tbl_get_row.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_tbl_get_row);
                sig_tbl_get_row.params.push(AbiParam::new(types::I64));

                let mut sig_json_bind = module.make_signature();
                sig_json_bind.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_json_bind);
                add_val_param(&mut sig_json_bind);

                let mut sig_json_bind_const = module.make_signature();
                sig_json_bind_const.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_json_bind_const);
                sig_json_bind_const.params.push(AbiParam::new(ptr_type));
                sig_json_bind_const.params.push(AbiParam::new(types::I64));

                let mut sig_get_member = module.make_signature();
                sig_get_member.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_get_member);
                sig_get_member.params.push(AbiParam::new(ptr_type));
                sig_get_member.params.push(AbiParam::new(types::I64));

                let mut sig_set_member = module.make_signature();
                add_val_param(&mut sig_set_member);
                sig_set_member.params.push(AbiParam::new(ptr_type));
                sig_set_member.params.push(AbiParam::new(types::I64));
                add_val_param(&mut sig_set_member);
                
                let mut sig_set_fiber_state = module.make_signature();
                sig_set_fiber_state.params.push(AbiParam::new(ptr_type));
                sig_set_fiber_state.params.push(AbiParam::new(types::I64));
                sig_set_fiber_state.params.push(AbiParam::new(types::I32));

                let mut sig_report_guard_failure = module.make_signature();
                sig_report_guard_failure.params.push(AbiParam::new(ptr_type));
                sig_report_guard_failure.params.push(AbiParam::new(types::I64));

                let mut sig_wait = module.make_signature();
                sig_wait.params.push(AbiParam::new(types::I64));

                let mut sig_ptr_u32_u32_ret_val = module.make_signature();
                sig_ptr_u32_u32_ret_val.params.push(AbiParam::new(ptr_type));
                sig_ptr_u32_u32_ret_val.params.push(AbiParam::new(ptr_type));
                sig_ptr_u32_u32_ret_val.params.push(AbiParam::new(types::I32));
                sig_ptr_u32_u32_ret_val.params.push(AbiParam::new(types::I32));

                let mut sig_coll_init = module.make_signature();
                sig_coll_init.params.push(AbiParam::new(ptr_type));
                sig_coll_init.params.push(AbiParam::new(ptr_type));
                sig_coll_init.params.push(AbiParam::new(ptr_type));
                sig_coll_init.params.push(AbiParam::new(types::I32));

                let mut sig_table_init_v2 = module.make_signature();
                sig_table_init_v2.params.push(AbiParam::new(ptr_type));
                sig_table_init_v2.params.push(AbiParam::new(types::I32));
                sig_table_init_v2.params.push(AbiParam::new(types::I32));
                sig_table_init_v2.params.push(AbiParam::new(types::I32));
                sig_table_init_v2.params.push(AbiParam::new(types::I32));
                sig_table_init_v2.params.push(AbiParam::new(ptr_type));
                sig_table_init_v2.params.push(AbiParam::new(ptr_type));
                
                let mut sig_db_init = module.make_signature();
                sig_db_init.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_db_init);
                add_val_param(&mut sig_db_init);
                sig_db_init.params.push(AbiParam::new(ptr_type));
                sig_db_init.params.push(AbiParam::new(types::I32));
                sig_db_init.params.push(AbiParam::new(types::I32));
                sig_db_init.params.push(AbiParam::new(ptr_type));

                let mut sig_ptr_u32_ret_void = module.make_signature();
                sig_ptr_u32_ret_void.params.push(AbiParam::new(ptr_type));
                sig_ptr_u32_ret_void.params.push(AbiParam::new(types::I32));

                let mut sig_exec_ret_void = module.make_signature();
                sig_exec_ret_void.params.push(AbiParam::new(ptr_type));

                let mut sig_exec_val_ret_void = module.make_signature();
                sig_exec_val_ret_void.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_exec_val_ret_void);

                let mut sig_val_val_val_ret_i32 = module.make_signature();
                add_val_param(&mut sig_val_val_val_ret_i32);
                add_val_param(&mut sig_val_val_val_ret_i32);
                add_val_param(&mut sig_val_val_val_ret_i32);
                sig_val_val_val_ret_i32.returns.push(AbiParam::new(types::I32));

                let mut sig_val_i64_val_ret_i32 = module.make_signature();
                add_val_param(&mut sig_val_i64_val_ret_i32);
                sig_val_i64_val_ret_i32.params.push(AbiParam::new(types::I64));
                add_val_param(&mut sig_val_i64_val_ret_i32);
                sig_val_i64_val_ret_i32.returns.push(AbiParam::new(types::I32));

                let mut sig_val_i64_u8_ret_i32 = module.make_signature();
                add_val_param(&mut sig_val_i64_u8_ret_i32);
                sig_val_i64_u8_ret_i32.params.push(AbiParam::new(types::I64));
                sig_val_i64_u8_ret_i32.params.push(AbiParam::new(types::I8));
                sig_val_i64_u8_ret_i32.returns.push(AbiParam::new(types::I32));

                let mut sig_val_val_i64_ret = module.make_signature();
                sig_val_val_i64_ret.params.push(AbiParam::new(ptr_type));
                add_val_param(&mut sig_val_val_i64_ret);
                sig_val_val_i64_ret.params.push(AbiParam::new(types::I64));

                let mut sig_val_val_i64_ret_i64 = module.make_signature();
                add_val_param(&mut sig_val_val_i64_ret_i64);
                sig_val_val_i64_ret_i64.params.push(AbiParam::new(types::I64));
                sig_val_val_i64_ret_i64.returns.push(AbiParam::new(types::I64));

                let mut sig_none_ret_val = module.make_signature();
                sig_none_ret_val.params.push(AbiParam::new(ptr_type));
                
                let mut sig_custom = module.make_signature();
                sig_custom.params.push(AbiParam::new(ptr_type)); // out
                add_val_param(&mut sig_custom); // receiver
                sig_custom.params.push(AbiParam::new(ptr_type)); // name_ptr
                sig_custom.params.push(AbiParam::new(types::I32)); // name_len
                sig_custom.params.push(AbiParam::new(ptr_type)); // args_ptr
                sig_custom.params.push(AbiParam::new(types::I8)); // arg_count
                sig_custom.params.push(AbiParam::new(ptr_type)); // executor_ptr

                let mut sig_val_i64_i64_ret_i32 = module.make_signature();
                add_val_param(&mut sig_val_i64_i64_ret_i32); // arr_bits, arr_tag
                sig_val_i64_i64_ret_i32.params.push(AbiParam::new(types::I64)); // idx
                sig_val_i64_i64_ret_i32.params.push(AbiParam::new(types::I64)); // val
                sig_val_i64_i64_ret_i32.returns.push(AbiParam::new(types::I32));

                Self {
                    $( $sym_name: module.declare_function(stringify!($sym_name), Linkage::Import, &$sig_name).unwrap(), )*
                }
            }

            pub fn import_in_func(&self, module: &mut JITModule, func: &mut codegen::ir::Function) -> ImportedSymbols {
                ImportedSymbols {
                    $( $sym_name: module.declare_func_in_func(self.$sym_name, func), )*
                }
            }
        }
    };
}
