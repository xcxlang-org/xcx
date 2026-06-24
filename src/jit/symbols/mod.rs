use cranelift::prelude::*;
use cranelift_module::{Linkage, Module, FuncId};
use cranelift_jit::JITModule;
use cranelift_codegen::ir::FuncRef;

pub struct SymbolRegistry {
    pub xcx_jit_random_int: FuncId,
    pub xcx_jit_random_float: FuncId,
    pub xcx_jit_pow_int: FuncId,
    pub xcx_jit_pow_float: FuncId,
    pub xcx_jit_int_concat: FuncId,
    pub xcx_jit_has: FuncId,
    pub xcx_jit_random_choice: FuncId,
    pub xcx_jit_array_size: FuncId,
    pub xcx_jit_array_get: FuncId,
    pub xcx_jit_array_push: FuncId,
    pub xcx_jit_array_update: FuncId,
    pub xcx_jit_array_set_bool: FuncId,
    pub xcx_jit_array_get_bool: FuncId,
    pub xcx_jit_call_recursive: FuncId,
    pub xcx_jit_set_size: FuncId,
    pub xcx_jit_set_contains: FuncId,
    pub xcx_jit_set_values: FuncId,
    pub xcx_jit_inc_ref: FuncId,
    pub xcx_jit_dec_ref: FuncId,
    pub xcx_jit_dec_ref_range: FuncId,
    pub xcx_jit_method_dispatch: FuncId,
    pub xcx_jit_method_dispatch_named: FuncId,
    pub xcx_jit_fiber_is_done: FuncId,
    pub xcx_jit_fiber_next: FuncId,
    pub xcx_jit_fiber_run: FuncId,
    pub xcx_jit_add: FuncId,
    pub xcx_jit_sub: FuncId,
    pub xcx_jit_mul: FuncId,
    pub xcx_jit_div: FuncId,
    pub xcx_jit_mod: FuncId,
    pub xcx_jit_abort_div: FuncId,
    pub xcx_jit_has_errors: FuncId,
    pub xcx_jit_neg: FuncId,
    pub xcx_jit_eq: FuncId,
    pub xcx_jit_ne: FuncId,
    pub xcx_jit_gt: FuncId,
    pub xcx_jit_lt: FuncId,
    pub xcx_jit_ge: FuncId,
    pub xcx_jit_le: FuncId,
    pub xcx_jit_row_get: FuncId,
    pub xcx_jit_table_size: FuncId,
    pub xcx_jit_table_get_row: FuncId,
    pub xcx_jit_table_push_row: FuncId,
    pub xcx_jit_table_clone_skeleton: FuncId,
    pub xcx_jit_json_bind: FuncId,
    pub xcx_jit_json_bind_const: FuncId,
    pub xcx_jit_get_member: FuncId,
    pub xcx_jit_set_fiber_state: FuncId,
    pub xcx_jit_report_guard_failure: FuncId,
    pub xcx_jit_wait: FuncId,
    pub xcx_jit_string_length: FuncId,
    pub xcx_jit_json_parse: FuncId,
    pub xcx_jit_json_to_str: FuncId,
    pub xcx_jit_date_now: FuncId,
    pub xcx_jit_perf_ms: FuncId,
    pub xcx_jit_perf_us: FuncId,
    pub xcx_jit_perf_ns: FuncId,
    pub xcx_jit_array_init: FuncId,
    pub xcx_jit_set_init: FuncId,
    pub xcx_jit_map_init: FuncId,
    pub xcx_jit_table_init: FuncId,
    pub xcx_jit_method_call_custom: FuncId,
    pub xcx_jit_net_call: FuncId,
    pub xcx_jit_net_request: FuncId,
    pub xcx_jit_yield: FuncId,
    pub xcx_jit_http_serve: FuncId,
    pub xcx_jit_http_respond: FuncId,
    
    pub xcx_jit_print: FuncId,
    pub xcx_jit_halt_alert: FuncId,
    pub xcx_jit_halt_error: FuncId,
    pub xcx_jit_halt_fatal: FuncId,
    pub xcx_jit_typeof: FuncId,
    pub xcx_jit_store_read: FuncId,
    pub xcx_jit_store_write: FuncId,
    pub xcx_jit_store_append: FuncId,
    pub xcx_jit_store_exists: FuncId,
    pub xcx_jit_store_delete: FuncId,
    pub xcx_jit_database_init: FuncId,
    pub xcx_jit_set_member: FuncId,
    pub xcx_jit_env_get: FuncId,
    pub xcx_jit_env_args: FuncId,
    pub xcx_jit_crypto_hash: FuncId,
    pub xcx_jit_crypto_verify: FuncId,
    pub xcx_jit_crypto_token: FuncId,
    pub xcx_jit_fiber_create: FuncId,
    pub xcx_jit_set_range: FuncId,
    pub xcx_jit_set_remove: FuncId,
    pub xcx_jit_set_union: FuncId,
    pub xcx_jit_set_intersection: FuncId,
    pub xcx_jit_set_difference: FuncId,
    pub xcx_jit_set_sym_difference: FuncId,
    pub xcx_jit_terminal_clear: FuncId,
    pub xcx_jit_terminal_raw: FuncId,
    pub xcx_jit_terminal_normal: FuncId,
    pub xcx_jit_terminal_cursor: FuncId,
    pub xcx_jit_terminal_move: FuncId,
    pub xcx_jit_terminal_exit: FuncId,
    pub xcx_jit_terminal_run: FuncId,
    pub xcx_jit_terminal_write: FuncId,
    pub xcx_jit_json_get: FuncId,
    pub xcx_jit_json_set: FuncId,
    pub xcx_jit_json_push: FuncId,
    pub xcx_jit_json_get_push: FuncId,
    pub xcx_jit_cast_string: FuncId,
    pub xcx_jit_cast_bool: FuncId,
    pub xcx_jit_cast_int: FuncId,
    pub xcx_jit_cast_float: FuncId,
    pub xcx_jit_array_get_int: FuncId,
    pub xcx_jit_array_set_int: FuncId,
    pub xcx_jit_string_starts_with: FuncId,
    pub xcx_jit_string_ends_with: FuncId,
    pub xcx_jit_string_upper: FuncId,
    pub xcx_jit_string_lower: FuncId,
    pub xcx_jit_string_trim: FuncId,
    pub xcx_jit_string_slice: FuncId,
    pub xcx_jit_string_replace: FuncId,
    pub xcx_jit_string_index_of: FuncId,
    pub xcx_jit_string_last_index_of: FuncId,
    pub xcx_jit_string_to_int: FuncId,
    pub xcx_jit_string_to_float: FuncId,
    pub xcx_jit_map_size: FuncId,
    pub xcx_jit_map_contains: FuncId,
    pub xcx_jit_map_get: FuncId,
    pub xcx_jit_map_insert: FuncId,
    pub xcx_jit_map_remove: FuncId,
    pub xcx_jit_map_clear: FuncId,
    pub xcx_jit_map_keys: FuncId,
    pub xcx_jit_map_values: FuncId,
    pub xcx_jit_array_pop: FuncId,
    pub xcx_jit_array_clear: FuncId,
    pub xcx_jit_array_is_empty: FuncId,
    pub xcx_jit_array_contains: FuncId,
    pub xcx_jit_array_find: FuncId,
    pub xcx_jit_array_insert: FuncId,
    pub xcx_jit_array_delete: FuncId,
    pub xcx_jit_array_sort: FuncId,
    pub xcx_jit_array_reverse: FuncId,
    pub xcx_jit_date_field: FuncId,
    pub xcx_jit_check_recursion: FuncId,
    pub xcx_jit_dec_recursion: FuncId,
}

impl SymbolRegistry {
    pub fn new(module: &mut JITModule) -> Self {
        let ptr_type = module.target_config().pointer_type();
        
        let add_val_param = |sig: &mut Signature| {
            sig.params.push(AbiParam::new(types::I64)); // bits
            sig.params.push(AbiParam::new(types::I64)); // tag
        };

        // (out: *mut Value) -> void
        let mut sig_val_ret = module.make_signature();
        sig_val_ret.params.push(AbiParam::new(ptr_type));

        // (out: *mut Value, b, t) -> void
        let mut sig_val_val_ret = module.make_signature();
        sig_val_val_ret.params.push(AbiParam::new(ptr_type));
        add_val_param(&mut sig_val_val_ret);

        // (out: *mut Value, b1, t1, b2, t2) -> void
        let mut sig_val_val_val_ret = module.make_signature();
        sig_val_val_val_ret.params.push(AbiParam::new(ptr_type));
        add_val_param(&mut sig_val_val_val_ret);
        add_val_param(&mut sig_val_val_val_ret);

        // (b, t) -> void
        let mut sig_val_ret_void = module.make_signature();
        add_val_param(&mut sig_val_ret_void);

        // (b1, t1, b2, t2) -> void
        let mut sig_val_val_ret_void = module.make_signature();
        add_val_param(&mut sig_val_val_ret_void);
        add_val_param(&mut sig_val_val_ret_void);

        // (b1, t1, b2, t2, b3, t3) -> void
        let mut sig_val_val_val_ret_void = module.make_signature();
        add_val_param(&mut sig_val_val_val_ret_void);
        add_val_param(&mut sig_val_val_val_ret_void);
        add_val_param(&mut sig_val_val_val_ret_void);

        // (b, t) -> i64
        let mut sig_val_ret_i64 = module.make_signature();
        add_val_param(&mut sig_val_ret_i64);
        sig_val_ret_i64.returns.push(AbiParam::new(types::I64));

        // (b, t) -> f64
        let mut sig_val_ret_f64 = module.make_signature();
        add_val_param(&mut sig_val_ret_f64);
        sig_val_ret_f64.returns.push(AbiParam::new(types::F64));

        // (b, t) -> u8/bool
        let mut sig_val_ret_bool = module.make_signature();
        add_val_param(&mut sig_val_ret_bool);
        sig_val_ret_bool.returns.push(AbiParam::new(types::I8));

        let mut sig_val_exec_ret_bool = module.make_signature();
        add_val_param(&mut sig_val_exec_ret_bool);
        sig_val_exec_ret_bool.params.push(AbiParam::new(ptr_type));
        sig_val_exec_ret_bool.returns.push(AbiParam::new(types::I8));

        // (b1, t1, b2, t2) -> i64
        let mut sig_val_val_ret_i64 = module.make_signature();
        add_val_param(&mut sig_val_val_ret_i64);
        add_val_param(&mut sig_val_val_ret_i64);
        sig_val_val_ret_i64.returns.push(AbiParam::new(types::I64));

        // (b1, t1, b2, t2) -> u8/bool
        let mut sig_val_val_ret_bool = module.make_signature();
        add_val_param(&mut sig_val_val_ret_bool);
        add_val_param(&mut sig_val_val_ret_bool);
        sig_val_val_ret_bool.returns.push(AbiParam::new(types::I8));

        let mut sig_binop_exec = module.make_signature();
        sig_binop_exec.params.push(AbiParam::new(ptr_type));
        add_val_param(&mut sig_binop_exec);
        add_val_param(&mut sig_binop_exec);
        sig_binop_exec.params.push(AbiParam::new(ptr_type));

        let mut sig_val_val_val_val_ret = module.make_signature();
        add_val_param(&mut sig_val_val_val_val_ret);
        add_val_param(&mut sig_val_val_val_val_ret);
        add_val_param(&mut sig_val_val_val_val_ret);
        if cfg!(windows) {
            sig_val_val_val_val_ret.params.insert(0, AbiParam::special(ptr_type, cranelift_codegen::ir::ArgumentPurpose::StructReturn));
        } else {
            sig_val_val_val_val_ret.returns.push(AbiParam::new(types::I64));
            sig_val_val_val_val_ret.returns.push(AbiParam::new(types::I64));
        }

        // (b1, t1, b2, t2) -> i32
        let mut sig_val_val_ret_i32 = module.make_signature();
        add_val_param(&mut sig_val_val_ret_i32);
        add_val_param(&mut sig_val_val_ret_i32);
        sig_val_val_ret_i32.returns.push(AbiParam::new(types::I32));

        // (b, t) -> i32
        let mut sig_val_ret_i32 = module.make_signature();
        add_val_param(&mut sig_val_ret_i32);
        sig_val_ret_i32.returns.push(AbiParam::new(types::I32));

        let mut sig_random_int = module.make_signature();
        sig_random_int.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_random_int); // min
        add_val_param(&mut sig_random_int); // max
        add_val_param(&mut sig_random_int); // step
        sig_random_int.params.push(AbiParam::new(types::I8)); // has_step

        let mut sig_random_float = module.make_signature();
        sig_random_float.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_random_float); // min
        add_val_param(&mut sig_random_float); // max
        add_val_param(&mut sig_random_float); // step
        sig_random_float.params.push(AbiParam::new(types::I8)); // has_step

        let mut sig_db_init = module.make_signature();
        sig_db_init.params.push(AbiParam::new(ptr_type)); // out
        sig_db_init.params.push(AbiParam::new(ptr_type)); // executor_ptr
        add_val_param(&mut sig_db_init); // ds_val
        
        let sig_terminal_void = module.make_signature();
        
        let mut sig_terminal_exec = module.make_signature();
        sig_terminal_exec.params.push(AbiParam::new(ptr_type));
        
        let mut sig_terminal_cursor = module.make_signature();
        sig_terminal_cursor.params.push(AbiParam::new(types::I8));
        
        let mut sig_terminal_move = module.make_signature();
        add_val_param(&mut sig_terminal_move); // x
        add_val_param(&mut sig_terminal_move); // y

        let mut sig_terminal_run = module.make_signature();
        sig_terminal_run.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_terminal_run); // cmd

        let mut sig_terminal_write = module.make_signature();
        sig_terminal_write.params.push(AbiParam::new(types::I64));
        sig_terminal_write.params.push(AbiParam::new(types::I64));

        let mut sig_pow_int = module.make_signature();
        sig_pow_int.params.push(AbiParam::new(ptr_type)); // out
        sig_pow_int.params.push(AbiParam::new(types::I64));
        sig_pow_int.params.push(AbiParam::new(types::I64));

        let mut sig_pow_float = module.make_signature();
        sig_pow_float.params.push(AbiParam::new(ptr_type)); // out
        sig_pow_float.params.push(AbiParam::new(types::F64));
        sig_pow_float.params.push(AbiParam::new(types::F64));

        let mut sig_call_rec = module.make_signature();
        sig_call_rec.params.push(AbiParam::new(ptr_type)); // out
        sig_call_rec.params.push(AbiParam::new(types::I64)); // func_idx
        sig_call_rec.params.push(AbiParam::new(ptr_type)); // args_ptr
        sig_call_rec.params.push(AbiParam::new(types::I8)); // arg_count
        sig_call_rec.params.push(AbiParam::new(ptr_type)); // executor_ptr
        sig_call_rec.returns.push(AbiParam::new(types::I32)); // status

        let mut sig_method = module.make_signature();
        sig_method.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_method); // receiver
        sig_method.params.push(AbiParam::new(types::I8));  // kind
        sig_method.params.push(AbiParam::new(ptr_type)); // args_ptr
        sig_method.params.push(AbiParam::new(types::I8));  // arg_count
        sig_method.params.push(AbiParam::new(ptr_type)); // executor_ptr

        let mut sig_method_named = module.make_signature();
        sig_method_named.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_method_named); // receiver
        sig_method_named.params.push(AbiParam::new(types::I8));  // kind
        sig_method_named.params.push(AbiParam::new(ptr_type)); // args_ptr
        sig_method_named.params.push(AbiParam::new(types::I8));  // arg_count
        add_val_param(&mut sig_method_named); // names (bits, tag)
        sig_method_named.params.push(AbiParam::new(ptr_type)); // executor_ptr

        let mut sig_row_get = module.make_signature();
        sig_row_get.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_row_get); // row_bits, tag
        sig_row_get.params.push(AbiParam::new(types::I32)); // col_idx
        
        let mut sig_tbl_get_row = module.make_signature();
        sig_tbl_get_row.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_tbl_get_row); // table
        sig_tbl_get_row.params.push(AbiParam::new(types::I64)); // row_idx

        let mut sig_fiber_create = module.make_signature();
        sig_fiber_create.params.push(AbiParam::new(ptr_type)); // out
        sig_fiber_create.params.push(AbiParam::new(types::I64)); // func_idx
        sig_fiber_create.params.push(AbiParam::new(types::I8));  // base
        sig_fiber_create.params.push(AbiParam::new(types::I8));  // arg_count
        sig_fiber_create.params.push(AbiParam::new(ptr_type)); // executor_ptr
        sig_fiber_create.params.push(AbiParam::new(ptr_type)); // locals_ptr

        let mut sig_json_bind = module.make_signature();
        sig_json_bind.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_json_bind); // json
        add_val_param(&mut sig_json_bind); // path

        let mut sig_json_bind_const = module.make_signature();
        sig_json_bind_const.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_json_bind_const); // json
        sig_json_bind_const.params.push(AbiParam::new(ptr_type)); // path_ptr
        sig_json_bind_const.params.push(AbiParam::new(types::I64)); // path_len

        let mut sig_get_member = module.make_signature();
        sig_get_member.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_get_member); // obj
        sig_get_member.params.push(AbiParam::new(ptr_type)); // name_ptr
        sig_get_member.params.push(AbiParam::new(types::I64)); // name_len

        let mut sig_set_member = module.make_signature();
        add_val_param(&mut sig_set_member); // obj
        sig_set_member.params.push(AbiParam::new(ptr_type)); // name_ptr
        sig_set_member.params.push(AbiParam::new(types::I64)); // name_len
        add_val_param(&mut sig_set_member); // value
        
        let mut sig_set_fiber_state = module.make_signature();
        sig_set_fiber_state.params.push(AbiParam::new(ptr_type)); // executor_ptr
        sig_set_fiber_state.params.push(AbiParam::new(types::I64)); // fiber bits (not used as value, but just i64)
        sig_set_fiber_state.params.push(AbiParam::new(types::I32)); // state

        let mut sig_report_guard_failure = module.make_signature();
        sig_report_guard_failure.params.push(AbiParam::new(ptr_type)); // executor_ptr
        sig_report_guard_failure.params.push(AbiParam::new(types::I64)); // failing_ip

        let mut sig_wait = module.make_signature();
        sig_wait.params.push(AbiParam::new(types::I64));

        let mut sig_ptr_u32_u32_ret_val = module.make_signature();
        sig_ptr_u32_u32_ret_val.params.push(AbiParam::new(ptr_type)); // out
        sig_ptr_u32_u32_ret_val.params.push(AbiParam::new(ptr_type)); // executor_ptr
        sig_ptr_u32_u32_ret_val.params.push(AbiParam::new(types::I32));
        sig_ptr_u32_u32_ret_val.params.push(AbiParam::new(types::I32));

        let mut sig_coll_init = module.make_signature();
        sig_coll_init.params.push(AbiParam::new(ptr_type)); // out
        sig_coll_init.params.push(AbiParam::new(ptr_type)); // executor_ptr
        sig_coll_init.params.push(AbiParam::new(ptr_type)); // elements_ptr
        sig_coll_init.params.push(AbiParam::new(types::I32)); // count

        let mut sig_table_init_v2 = module.make_signature();
        sig_table_init_v2.params.push(AbiParam::new(ptr_type)); // out
        sig_table_init_v2.params.push(AbiParam::new(types::I32)); // skeleton_idx
        sig_table_init_v2.params.push(AbiParam::new(types::I32)); // base
        sig_table_init_v2.params.push(AbiParam::new(types::I32)); // row_count
        sig_table_init_v2.params.push(AbiParam::new(types::I32)); // col_count
        sig_table_init_v2.params.push(AbiParam::new(ptr_type)); // locals_ptr
        sig_table_init_v2.params.push(AbiParam::new(ptr_type)); // constants_ptr

        let mut sig_net_call = module.make_signature();
        sig_net_call.params.push(AbiParam::new(ptr_type)); // out
        sig_net_call.params.push(AbiParam::new(types::I64)); // method_idx
        add_val_param(&mut sig_net_call); // url_bits, url_tag
        add_val_param(&mut sig_net_call); // body_bits, body_tag
        sig_net_call.params.push(AbiParam::new(ptr_type)); // consts

        let mut sig_yield = module.make_signature();
        sig_yield.params.push(AbiParam::new(ptr_type)); // exec_ptr
        add_val_param(&mut sig_yield); // val_bits, val_tag
        sig_yield.params.push(AbiParam::new(types::I64)); // next_ip
        sig_yield.params.push(AbiParam::new(ptr_type)); // out_ptr
        sig_yield.returns.push(AbiParam::new(types::I32));

        let mut sig_http_serve = module.make_signature();
        sig_http_serve.params.push(AbiParam::new(types::I32)); // func_idx
        add_val_param(&mut sig_http_serve); // port
        add_val_param(&mut sig_http_serve); // host
        add_val_param(&mut sig_http_serve); // routes
        sig_http_serve.params.push(AbiParam::new(ptr_type)); // exec_ptr
        sig_http_serve.returns.push(AbiParam::new(types::I32));

        let mut sig_http_respond = module.make_signature();
        sig_http_respond.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_http_respond); // status
        add_val_param(&mut sig_http_respond); // body
        add_val_param(&mut sig_http_respond); // headers
        sig_http_respond.params.push(AbiParam::new(ptr_type)); // exec_ptr
        sig_http_respond.returns.push(AbiParam::new(types::I32));

        let mut sig_db_init = module.make_signature();
        sig_db_init.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_db_init); // engine
        add_val_param(&mut sig_db_init); // path
        sig_db_init.params.push(AbiParam::new(ptr_type)); // locals_ptr
        sig_db_init.params.push(AbiParam::new(types::I32)); // tables_base_reg
        sig_db_init.params.push(AbiParam::new(types::I32)); // table_count
        sig_db_init.params.push(AbiParam::new(ptr_type)); // executor_ptr

        let mut sig_ptr_u32_ret_void = module.make_signature();
        sig_ptr_u32_ret_void.params.push(AbiParam::new(ptr_type));
        sig_ptr_u32_ret_void.params.push(AbiParam::new(types::I32));

        let mut sig_exec_ret_void = module.make_signature();
        sig_exec_ret_void.params.push(AbiParam::new(ptr_type));

        let mut sig_exec_ret_i32 = module.make_signature();
        sig_exec_ret_i32.params.push(AbiParam::new(ptr_type));
        sig_exec_ret_i32.returns.push(AbiParam::new(cranelift_codegen::ir::types::I32));

        let mut sig_exec_val_ret_void = module.make_signature();
        sig_exec_val_ret_void.params.push(AbiParam::new(ptr_type));
        add_val_param(&mut sig_exec_val_ret_void);

        let mut sig_val_exec_ret = module.make_signature();
        sig_val_exec_ret.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_val_exec_ret); // fib_bits, fib_tag
        sig_val_exec_ret.params.push(AbiParam::new(ptr_type)); // exec_ptr

        let mut sig_val_val_val_ret_i32 = module.make_signature();
        add_val_param(&mut sig_val_val_val_ret_i32);
        add_val_param(&mut sig_val_val_val_ret_i32);
        add_val_param(&mut sig_val_val_val_ret_i32);
        sig_val_val_val_ret_i32.returns.push(AbiParam::new(types::I32));

        let mut sig_val_i64_ret_i32 = module.make_signature();
        add_val_param(&mut sig_val_i64_ret_i32);
        sig_val_i64_ret_i32.params.push(AbiParam::new(types::I64));
        sig_val_i64_ret_i32.returns.push(AbiParam::new(types::I32));

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
        add_val_param(&mut sig_val_val_i64_ret_i64); // arr_bits, arr_tag
        sig_val_val_i64_ret_i64.params.push(AbiParam::new(types::I64)); // idx
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

        let mut sig_val_val_i64_i64_ret = module.make_signature();
        sig_val_val_i64_i64_ret.params.push(AbiParam::new(ptr_type)); // out
        add_val_param(&mut sig_val_val_i64_i64_ret); // bits, tag
        sig_val_val_i64_i64_ret.params.push(AbiParam::new(types::I64));
        sig_val_val_i64_i64_ret.params.push(AbiParam::new(types::I64));

        let mut sig_val_val_ret_i32 = module.make_signature();
        add_val_param(&mut sig_val_val_ret_i32);
        add_val_param(&mut sig_val_val_ret_i32);
        sig_val_val_ret_i32.returns.push(AbiParam::new(types::I32));

        let mut sig_val_i64_ret_i64 = module.make_signature();
        add_val_param(&mut sig_val_i64_ret_i64);
        sig_val_i64_ret_i64.params.push(AbiParam::new(types::I64));
        sig_val_i64_ret_i64.returns.push(AbiParam::new(types::I64));

        Self {
            xcx_jit_string_upper: module.declare_function("xcx_jit_string_upper", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_string_lower: module.declare_function("xcx_jit_string_lower", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_string_trim: module.declare_function("xcx_jit_string_trim", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_string_slice: module.declare_function("xcx_jit_string_slice", Linkage::Import, &sig_val_val_i64_i64_ret).unwrap(),
            xcx_jit_string_replace: module.declare_function("xcx_jit_string_replace", Linkage::Import, &sig_val_val_val_val_ret).unwrap(),
            xcx_jit_string_index_of: module.declare_function("xcx_jit_string_index_of", Linkage::Import, &sig_val_val_ret_i64).unwrap(),
            xcx_jit_string_last_index_of: module.declare_function("xcx_jit_string_last_index_of", Linkage::Import, &sig_val_val_ret_i64).unwrap(),
            xcx_jit_string_to_int: module.declare_function("xcx_jit_string_to_int", Linkage::Import, &sig_val_ret_i64).unwrap(),
            xcx_jit_string_to_float: module.declare_function("xcx_jit_string_to_float", Linkage::Import, &sig_val_ret_f64).unwrap(),
            
            xcx_jit_map_size: module.declare_function("xcx_jit_map_size", Linkage::Import, &sig_val_ret_i64).unwrap(),
            xcx_jit_map_contains: module.declare_function("xcx_jit_map_contains", Linkage::Import, &sig_val_val_ret_i32).unwrap(),
            xcx_jit_map_get: module.declare_function("xcx_jit_map_get", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_map_insert: module.declare_function("xcx_jit_map_insert", Linkage::Import, &sig_val_val_val_ret_i32).unwrap(),
            xcx_jit_map_remove: module.declare_function("xcx_jit_map_remove", Linkage::Import, &sig_val_val_ret_i32).unwrap(),
            xcx_jit_map_clear: module.declare_function("xcx_jit_map_clear", Linkage::Import, &sig_val_ret_i32).unwrap(),
            xcx_jit_map_keys: module.declare_function("xcx_jit_map_keys", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_map_values: module.declare_function("xcx_jit_map_values", Linkage::Import, &sig_val_val_ret).unwrap(),
            

            xcx_jit_random_int: module.declare_function("xcx_jit_random_int", Linkage::Import, &sig_random_int).unwrap(),
            xcx_jit_random_float: module.declare_function("xcx_jit_random_float", Linkage::Import, &sig_random_float).unwrap(),
            xcx_jit_pow_int: module.declare_function("xcx_jit_pow_int", Linkage::Import, &sig_pow_int).unwrap(),
            xcx_jit_pow_float: module.declare_function("xcx_jit_pow_float", Linkage::Import, &sig_pow_float).unwrap(),
            xcx_jit_int_concat: module.declare_function("xcx_jit_int_concat", Linkage::Import, &sig_pow_int).unwrap(),
            xcx_jit_has: module.declare_function("xcx_jit_has", Linkage::Import, &sig_val_val_ret_bool).unwrap(),
            xcx_jit_random_choice: module.declare_function("xcx_jit_random_choice", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_array_size: module.declare_function("xcx_jit_array_size", Linkage::Import, &sig_val_ret_i64).unwrap(),
            xcx_jit_array_get: module.declare_function("xcx_jit_array_get", Linkage::Import, &sig_val_val_i64_ret).unwrap(),
            xcx_jit_array_push: module.declare_function("xcx_jit_array_push", Linkage::Import, &sig_val_val_ret_void).unwrap(),
            xcx_jit_array_update: module.declare_function("xcx_jit_array_update", Linkage::Import, &sig_val_i64_val_ret_i32).unwrap(),
            xcx_jit_array_set_bool: module.declare_function("xcx_jit_array_set_bool", Linkage::Import, &sig_val_i64_u8_ret_i32).unwrap(),
            xcx_jit_array_get_bool: module.declare_function("xcx_jit_array_get_bool", Linkage::Import, &sig_val_val_i64_ret_i64).unwrap(),
            xcx_jit_call_recursive: module.declare_function("xcx_jit_call_recursive", Linkage::Import, &sig_call_rec).unwrap(),
            xcx_jit_set_size: module.declare_function("xcx_jit_set_size", Linkage::Import, &sig_val_ret_i64).unwrap(),
            xcx_jit_set_contains: module.declare_function("xcx_jit_set_contains", Linkage::Import, &sig_val_val_ret_bool).unwrap(),
            xcx_jit_set_values: module.declare_function("xcx_jit_set_values", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_inc_ref: module.declare_function("xcx_jit_inc_ref", Linkage::Import, &sig_val_ret_void).unwrap(),
            xcx_jit_dec_ref: module.declare_function("xcx_jit_dec_ref", Linkage::Import, &sig_val_ret_void).unwrap(),
            xcx_jit_dec_ref_range: module.declare_function("xcx_jit_dec_ref_range", Linkage::Import, &sig_ptr_u32_ret_void).unwrap(),
            xcx_jit_method_dispatch: module.declare_function("xcx_jit_method_dispatch", Linkage::Import, &sig_method).unwrap(),
            xcx_jit_method_dispatch_named: module.declare_function("xcx_jit_method_dispatch_named", Linkage::Import, &sig_method_named).unwrap(),
            xcx_jit_fiber_is_done: module.declare_function("xcx_jit_fiber_is_done", Linkage::Import, &sig_val_ret_bool).unwrap(),
            xcx_jit_fiber_next: module.declare_function("xcx_jit_fiber_next", Linkage::Import, &sig_val_exec_ret).unwrap(),
            xcx_jit_fiber_run: module.declare_function("xcx_jit_fiber_run", Linkage::Import, &sig_val_exec_ret_bool).unwrap(),
            xcx_jit_add: module.declare_function("xcx_jit_add", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_sub: module.declare_function("xcx_jit_sub", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_mul: module.declare_function("xcx_jit_mul", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_div: module.declare_function("xcx_jit_div", Linkage::Import, &sig_binop_exec).unwrap(),
            xcx_jit_mod: module.declare_function("xcx_jit_mod", Linkage::Import, &sig_binop_exec).unwrap(),
            
            xcx_jit_abort_div: {
                let mut sig = module.make_signature();
                sig.params.push(AbiParam::new(ptr_type));
                module.declare_function("xcx_jit_abort_div", Linkage::Import, &sig).unwrap()
            },
            xcx_jit_has_errors: {
                let mut sig = module.make_signature();
                sig.params.push(AbiParam::new(ptr_type));
                sig.returns.push(AbiParam::new(cranelift::prelude::types::I32));
                module.declare_function("xcx_jit_has_errors", Linkage::Import, &sig).unwrap()
            },
            
            xcx_jit_neg: module.declare_function("xcx_jit_neg", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_eq: module.declare_function("xcx_jit_eq", Linkage::Import, &sig_val_val_val_ret).unwrap(), // returns bool-value
            xcx_jit_ne: module.declare_function("xcx_jit_ne", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_gt: module.declare_function("xcx_jit_gt", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_lt: module.declare_function("xcx_jit_lt", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_ge: module.declare_function("xcx_jit_ge", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_le: module.declare_function("xcx_jit_le", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_row_get: module.declare_function("xcx_jit_row_get", Linkage::Import, &sig_row_get).unwrap(),
            xcx_jit_table_size: module.declare_function("xcx_jit_table_size", Linkage::Import, &sig_val_ret_i64).unwrap(),
            xcx_jit_table_get_row: module.declare_function("xcx_jit_table_get_row", Linkage::Import, &sig_tbl_get_row).unwrap(),
            xcx_jit_table_push_row: module.declare_function("xcx_jit_table_push_row", Linkage::Import, &sig_val_val_ret_void).unwrap(),
            xcx_jit_table_clone_skeleton: module.declare_function("xcx_jit_table_clone_skeleton", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_json_bind: module.declare_function("xcx_jit_json_bind", Linkage::Import, &sig_json_bind).unwrap(),
            xcx_jit_json_bind_const: module.declare_function("xcx_jit_json_bind_const", Linkage::Import, &sig_json_bind_const).unwrap(),
            xcx_jit_get_member: module.declare_function("xcx_jit_get_member", Linkage::Import, &sig_get_member).unwrap(),
            xcx_jit_set_fiber_state: module.declare_function("xcx_jit_set_fiber_state", Linkage::Import, &sig_set_fiber_state).unwrap(),
            xcx_jit_report_guard_failure: module.declare_function("xcx_jit_report_guard_failure", Linkage::Import, &sig_report_guard_failure).unwrap(),
            xcx_jit_wait: module.declare_function("xcx_jit_wait", Linkage::Import, &sig_wait).unwrap(),
            xcx_jit_string_length: module.declare_function("xcx_jit_string_length", Linkage::Import, &sig_val_ret_i64).unwrap(),
            xcx_jit_json_parse: module.declare_function("xcx_jit_json_parse", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_json_to_str: module.declare_function("xcx_jit_json_to_str", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_date_now: module.declare_function("xcx_jit_date_now", Linkage::Import, &sig_none_ret_val).unwrap(),
            xcx_jit_perf_ms: {
                let mut sig = module.make_signature();
                sig.params.push(AbiParam::new(ptr_type)); // out
                sig.params.push(AbiParam::new(ptr_type)); // vm
                module.declare_function("xcx_jit_perf_ms", Linkage::Import, &sig).unwrap()
            },
            xcx_jit_perf_us: {
                let mut sig = module.make_signature();
                sig.params.push(AbiParam::new(ptr_type)); // out
                sig.params.push(AbiParam::new(ptr_type)); // vm
                module.declare_function("xcx_jit_perf_us", Linkage::Import, &sig).unwrap()
            },
            xcx_jit_perf_ns: {
                let mut sig = module.make_signature();
                sig.params.push(AbiParam::new(ptr_type)); // out
                sig.params.push(AbiParam::new(ptr_type)); // vm
                module.declare_function("xcx_jit_perf_ns", Linkage::Import, &sig).unwrap()
            },
            xcx_jit_array_init: module.declare_function("xcx_jit_array_init", Linkage::Import, &sig_coll_init).unwrap(),
            xcx_jit_set_init: module.declare_function("xcx_jit_set_init", Linkage::Import, &sig_coll_init).unwrap(),
            xcx_jit_map_init: module.declare_function("xcx_jit_map_init", Linkage::Import, &sig_coll_init).unwrap(),
            xcx_jit_table_init: module.declare_function("xcx_jit_table_init", Linkage::Import, &sig_table_init_v2).unwrap(),
            xcx_jit_method_call_custom: module.declare_function("xcx_jit_method_call_custom", Linkage::Import, &sig_custom).unwrap(),
            xcx_jit_array_get_int: module.declare_function("xcx_jit_array_get_int", Linkage::Import, &sig_val_val_i64_ret_i64).unwrap(),
            xcx_jit_array_set_int: module.declare_function("xcx_jit_array_set_int", Linkage::Import, &sig_val_i64_i64_ret_i32).unwrap(),
            xcx_jit_string_starts_with: module.declare_function("xcx_jit_string_starts_with", Linkage::Import, &sig_val_val_ret_bool).unwrap(),
            xcx_jit_string_ends_with: module.declare_function("xcx_jit_string_ends_with", Linkage::Import, &sig_val_val_ret_bool).unwrap(),
            xcx_jit_array_pop: module.declare_function("xcx_jit_array_pop", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_array_clear: module.declare_function("xcx_jit_array_clear", Linkage::Import, &sig_val_ret_void).unwrap(),
            xcx_jit_array_is_empty: module.declare_function("xcx_jit_array_is_empty", Linkage::Import, &sig_val_ret_i64).unwrap(),
            xcx_jit_array_contains: module.declare_function("xcx_jit_array_contains", Linkage::Import, &sig_val_val_ret_i64).unwrap(),
            xcx_jit_array_find: module.declare_function("xcx_jit_array_find", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_array_insert: module.declare_function("xcx_jit_array_insert", Linkage::Import, &sig_val_i64_val_ret_i32).unwrap(),
            xcx_jit_array_delete: module.declare_function("xcx_jit_array_delete", Linkage::Import, &sig_val_i64_ret_i32).unwrap(),
            xcx_jit_array_sort: module.declare_function("xcx_jit_array_sort", Linkage::Import, &sig_val_ret_i64).unwrap(),
            xcx_jit_array_reverse: module.declare_function("xcx_jit_array_reverse", Linkage::Import, &sig_val_ret_i64).unwrap(),

            xcx_jit_check_recursion: module.declare_function("xcx_jit_check_recursion", Linkage::Import, &sig_exec_ret_i32).unwrap(),
            xcx_jit_dec_recursion: module.declare_function("xcx_jit_dec_recursion", Linkage::Import, &sig_exec_ret_void).unwrap(),

            xcx_jit_print: module.declare_function("xcx_jit_print", Linkage::Import, &sig_val_ret_void).unwrap(),
            xcx_jit_date_field: module.declare_function("xcx_jit_date_field", Linkage::Import, &sig_val_i64_ret_i64).unwrap(),
            xcx_jit_halt_alert: module.declare_function("xcx_jit_halt_alert", Linkage::Import, &sig_val_ret_void).unwrap(),
            xcx_jit_halt_error: module.declare_function("xcx_jit_halt_error", Linkage::Import, &sig_exec_val_ret_void).unwrap(),
            xcx_jit_halt_fatal: module.declare_function("xcx_jit_halt_fatal", Linkage::Import, &sig_val_ret_void).unwrap(),
            xcx_jit_typeof: module.declare_function("xcx_jit_typeof", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_store_read: module.declare_function("xcx_jit_store_read", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_store_write: module.declare_function("xcx_jit_store_write", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_store_append: module.declare_function("xcx_jit_store_append", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_store_exists: module.declare_function("xcx_jit_store_exists", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_store_delete: module.declare_function("xcx_jit_store_delete", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_database_init: module.declare_function("xcx_jit_database_init", Linkage::Import, &sig_db_init).unwrap(),
            xcx_jit_set_member: module.declare_function("xcx_jit_set_member", Linkage::Import, &sig_set_member).unwrap(),
            xcx_jit_env_get: module.declare_function("xcx_jit_env_get", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_env_args: module.declare_function("xcx_jit_env_args", Linkage::Import, &sig_val_ret).unwrap(),
            xcx_jit_crypto_hash: module.declare_function("xcx_jit_crypto_hash", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_crypto_verify: module.declare_function("xcx_jit_crypto_verify", Linkage::Import, &sig_val_val_val_ret_i32).unwrap(),
            xcx_jit_crypto_token: module.declare_function("xcx_jit_crypto_token", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_fiber_create: module.declare_function("xcx_jit_fiber_create", Linkage::Import, &sig_fiber_create).unwrap(),
            
            xcx_jit_terminal_clear: module.declare_function("xcx_jit_terminal_clear", Linkage::Import, &sig_terminal_void).unwrap(),
            xcx_jit_terminal_raw: module.declare_function("xcx_jit_terminal_raw", Linkage::Import, &sig_terminal_exec).unwrap(),
            xcx_jit_terminal_normal: module.declare_function("xcx_jit_terminal_normal", Linkage::Import, &sig_terminal_exec).unwrap(),
            xcx_jit_terminal_cursor: module.declare_function("xcx_jit_terminal_cursor", Linkage::Import, &sig_terminal_cursor).unwrap(),
            xcx_jit_terminal_move: module.declare_function("xcx_jit_terminal_move", Linkage::Import, &sig_terminal_move).unwrap(),
            xcx_jit_terminal_exit: module.declare_function("xcx_jit_terminal_exit", Linkage::Import, &sig_terminal_void).unwrap(),
            xcx_jit_terminal_run: module.declare_function("xcx_jit_terminal_run", Linkage::Import, &sig_terminal_run).unwrap(),
            xcx_jit_terminal_write: module.declare_function("xcx_jit_terminal_write", Linkage::Import, &sig_terminal_write).unwrap(),

            xcx_jit_set_range: module.declare_function("xcx_jit_set_range", Linkage::Import, &sig_random_int).unwrap(),
            xcx_jit_set_remove: module.declare_function("xcx_jit_set_remove", Linkage::Import, &sig_val_val_ret_void).unwrap(),
            xcx_jit_set_union: module.declare_function("xcx_jit_set_union", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_set_intersection: module.declare_function("xcx_jit_set_intersection", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_set_difference: module.declare_function("xcx_jit_set_difference", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_set_sym_difference: module.declare_function("xcx_jit_set_sym_difference", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_json_get: module.declare_function("xcx_jit_json_get", Linkage::Import, &sig_val_val_val_ret).unwrap(),
            xcx_jit_json_set: module.declare_function("xcx_jit_json_set", Linkage::Import, &sig_val_val_val_ret_i32).unwrap(),
            xcx_jit_json_push: module.declare_function("xcx_jit_json_push", Linkage::Import, &sig_val_val_ret_void).unwrap(),
            xcx_jit_json_get_push: module.declare_function("xcx_jit_json_get_push", Linkage::Import, &sig_val_val_val_ret_void).unwrap(),
            xcx_jit_cast_string: module.declare_function("xcx_jit_cast_string", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_cast_bool: module.declare_function("xcx_jit_cast_bool", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_cast_int: module.declare_function("xcx_jit_cast_int", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_cast_float: module.declare_function("xcx_jit_cast_float", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_net_call: module.declare_function("xcx_jit_net_call", Linkage::Import, &sig_net_call).unwrap(),
            xcx_jit_net_request: module.declare_function("xcx_jit_net_request", Linkage::Import, &sig_val_val_ret).unwrap(),
            xcx_jit_yield: module.declare_function("xcx_jit_yield", Linkage::Import, &sig_yield).unwrap(),
            xcx_jit_http_serve: module.declare_function("xcx_jit_http_serve", Linkage::Import, &sig_http_serve).unwrap(),
            xcx_jit_http_respond: module.declare_function("xcx_jit_http_respond", Linkage::Import, &sig_http_respond).unwrap(),
        }
    }

    pub fn import_in_func(&self, module: &mut JITModule, func: &mut codegen::ir::Function) -> ImportedSymbols {
        ImportedSymbols {
            xcx_jit_random_int: module.declare_func_in_func(self.xcx_jit_random_int, func),
            xcx_jit_random_float: module.declare_func_in_func(self.xcx_jit_random_float, func),
            xcx_jit_pow_int: module.declare_func_in_func(self.xcx_jit_pow_int, func),
            xcx_jit_pow_float: module.declare_func_in_func(self.xcx_jit_pow_float, func),
            xcx_jit_int_concat: module.declare_func_in_func(self.xcx_jit_int_concat, func),
            xcx_jit_has: module.declare_func_in_func(self.xcx_jit_has, func),
            xcx_jit_random_choice: module.declare_func_in_func(self.xcx_jit_random_choice, func),
            xcx_jit_array_size: module.declare_func_in_func(self.xcx_jit_array_size, func),
            xcx_jit_array_get: module.declare_func_in_func(self.xcx_jit_array_get, func),
            xcx_jit_array_push: module.declare_func_in_func(self.xcx_jit_array_push, func),
            xcx_jit_array_update: module.declare_func_in_func(self.xcx_jit_array_update, func),
            xcx_jit_fiber_create: module.declare_func_in_func(self.xcx_jit_fiber_create, func),
            
            xcx_jit_terminal_clear: module.declare_func_in_func(self.xcx_jit_terminal_clear, func),
            xcx_jit_terminal_raw: module.declare_func_in_func(self.xcx_jit_terminal_raw, func),
            xcx_jit_terminal_normal: module.declare_func_in_func(self.xcx_jit_terminal_normal, func),
            xcx_jit_terminal_cursor: module.declare_func_in_func(self.xcx_jit_terminal_cursor, func),
            xcx_jit_terminal_move: module.declare_func_in_func(self.xcx_jit_terminal_move, func),
            xcx_jit_terminal_exit: module.declare_func_in_func(self.xcx_jit_terminal_exit, func),
            xcx_jit_terminal_run: module.declare_func_in_func(self.xcx_jit_terminal_run, func),
            xcx_jit_terminal_write: module.declare_func_in_func(self.xcx_jit_terminal_write, func),

            xcx_jit_array_set_bool: module.declare_func_in_func(self.xcx_jit_array_set_bool, func),
            xcx_jit_array_get_bool: module.declare_func_in_func(self.xcx_jit_array_get_bool, func),
            xcx_jit_call_recursive: module.declare_func_in_func(self.xcx_jit_call_recursive, func),
            xcx_jit_net_call: module.declare_func_in_func(self.xcx_jit_net_call, func),
            xcx_jit_net_request: module.declare_func_in_func(self.xcx_jit_net_request, func),
            xcx_jit_yield: module.declare_func_in_func(self.xcx_jit_yield, func),
            xcx_jit_http_serve: module.declare_func_in_func(self.xcx_jit_http_serve, func),
            xcx_jit_http_respond: module.declare_func_in_func(self.xcx_jit_http_respond, func),
            xcx_jit_set_size: module.declare_func_in_func(self.xcx_jit_set_size, func),
            xcx_jit_set_contains: module.declare_func_in_func(self.xcx_jit_set_contains, func),
            xcx_jit_set_values: module.declare_func_in_func(self.xcx_jit_set_values, func),
            xcx_jit_inc_ref: module.declare_func_in_func(self.xcx_jit_inc_ref, func),
            xcx_jit_dec_ref: module.declare_func_in_func(self.xcx_jit_dec_ref, func),
            xcx_jit_dec_ref_range: module.declare_func_in_func(self.xcx_jit_dec_ref_range, func),
            xcx_jit_method_dispatch: module.declare_func_in_func(self.xcx_jit_method_dispatch, func),
            xcx_jit_method_dispatch_named: module.declare_func_in_func(self.xcx_jit_method_dispatch_named, func),
            xcx_jit_fiber_is_done: module.declare_func_in_func(self.xcx_jit_fiber_is_done, func),
            xcx_jit_fiber_next: module.declare_func_in_func(self.xcx_jit_fiber_next, func),
            xcx_jit_fiber_run: module.declare_func_in_func(self.xcx_jit_fiber_run, func),
            xcx_jit_add: module.declare_func_in_func(self.xcx_jit_add, func),
            xcx_jit_sub: module.declare_func_in_func(self.xcx_jit_sub, func),
            xcx_jit_mul: module.declare_func_in_func(self.xcx_jit_mul, func),
            xcx_jit_div: module.declare_func_in_func(self.xcx_jit_div, func),
            xcx_jit_mod: module.declare_func_in_func(self.xcx_jit_mod, func),
            xcx_jit_abort_div: module.declare_func_in_func(self.xcx_jit_abort_div, func),
            xcx_jit_has_errors: module.declare_func_in_func(self.xcx_jit_has_errors, func),
            xcx_jit_neg: module.declare_func_in_func(self.xcx_jit_neg, func),
            xcx_jit_eq: module.declare_func_in_func(self.xcx_jit_eq, func),
            xcx_jit_ne: module.declare_func_in_func(self.xcx_jit_ne, func),
            xcx_jit_gt: module.declare_func_in_func(self.xcx_jit_gt, func),
            xcx_jit_lt: module.declare_func_in_func(self.xcx_jit_lt, func),
            xcx_jit_ge: module.declare_func_in_func(self.xcx_jit_ge, func),
            xcx_jit_le: module.declare_func_in_func(self.xcx_jit_le, func),
            xcx_jit_row_get: module.declare_func_in_func(self.xcx_jit_row_get, func),
            xcx_jit_table_size: module.declare_func_in_func(self.xcx_jit_table_size, func),
            xcx_jit_table_get_row: module.declare_func_in_func(self.xcx_jit_table_get_row, func),
            xcx_jit_table_push_row: module.declare_func_in_func(self.xcx_jit_table_push_row, func),
            xcx_jit_table_clone_skeleton: module.declare_func_in_func(self.xcx_jit_table_clone_skeleton, func),
            xcx_jit_json_bind: module.declare_func_in_func(self.xcx_jit_json_bind, func),
            xcx_jit_json_bind_const: module.declare_func_in_func(self.xcx_jit_json_bind_const, func),
            xcx_jit_get_member: module.declare_func_in_func(self.xcx_jit_get_member, func),
            xcx_jit_set_fiber_state: module.declare_func_in_func(self.xcx_jit_set_fiber_state, func),
            xcx_jit_report_guard_failure: module.declare_func_in_func(self.xcx_jit_report_guard_failure, func),
            xcx_jit_wait: module.declare_func_in_func(self.xcx_jit_wait, func),
            xcx_jit_string_length: module.declare_func_in_func(self.xcx_jit_string_length, func),
            xcx_jit_json_parse: module.declare_func_in_func(self.xcx_jit_json_parse, func),
            xcx_jit_json_to_str: module.declare_func_in_func(self.xcx_jit_json_to_str, func),
            xcx_jit_date_now: module.declare_func_in_func(self.xcx_jit_date_now, func),
            xcx_jit_perf_ms: module.declare_func_in_func(self.xcx_jit_perf_ms, func),
            xcx_jit_perf_us: module.declare_func_in_func(self.xcx_jit_perf_us, func),
            xcx_jit_perf_ns: module.declare_func_in_func(self.xcx_jit_perf_ns, func),
            xcx_jit_array_init: module.declare_func_in_func(self.xcx_jit_array_init, func),
            xcx_jit_set_init: module.declare_func_in_func(self.xcx_jit_set_init, func),
            xcx_jit_map_init: module.declare_func_in_func(self.xcx_jit_map_init, func),
            xcx_jit_table_init: module.declare_func_in_func(self.xcx_jit_table_init, func),
            
            xcx_jit_print: module.declare_func_in_func(self.xcx_jit_print, func),
            xcx_jit_halt_alert: module.declare_func_in_func(self.xcx_jit_halt_alert, func),
            xcx_jit_halt_error: module.declare_func_in_func(self.xcx_jit_halt_error, func),
            xcx_jit_halt_fatal: module.declare_func_in_func(self.xcx_jit_halt_fatal, func),
            xcx_jit_typeof: module.declare_func_in_func(self.xcx_jit_typeof, func),
            xcx_jit_store_read: module.declare_func_in_func(self.xcx_jit_store_read, func),
            xcx_jit_store_write: module.declare_func_in_func(self.xcx_jit_store_write, func),
            xcx_jit_store_append: module.declare_func_in_func(self.xcx_jit_store_append, func),
            xcx_jit_store_exists: module.declare_func_in_func(self.xcx_jit_store_exists, func),
            xcx_jit_store_delete: module.declare_func_in_func(self.xcx_jit_store_delete, func),
            xcx_jit_database_init: module.declare_func_in_func(self.xcx_jit_database_init, func),
            xcx_jit_set_member: module.declare_func_in_func(self.xcx_jit_set_member, func),
            xcx_jit_env_get: module.declare_func_in_func(self.xcx_jit_env_get, func),
            xcx_jit_env_args: module.declare_func_in_func(self.xcx_jit_env_args, func),
            xcx_jit_crypto_hash: module.declare_func_in_func(self.xcx_jit_crypto_hash, func),
            xcx_jit_crypto_verify: module.declare_func_in_func(self.xcx_jit_crypto_verify, func),
            xcx_jit_crypto_token: module.declare_func_in_func(self.xcx_jit_crypto_token, func),
            xcx_jit_set_range: module.declare_func_in_func(self.xcx_jit_set_range, func),
            xcx_jit_set_remove: module.declare_func_in_func(self.xcx_jit_set_remove, func),
            xcx_jit_set_union: module.declare_func_in_func(self.xcx_jit_set_union, func),
            xcx_jit_set_intersection: module.declare_func_in_func(self.xcx_jit_set_intersection, func),
            xcx_jit_set_difference: module.declare_func_in_func(self.xcx_jit_set_difference, func),
            xcx_jit_set_sym_difference: module.declare_func_in_func(self.xcx_jit_set_sym_difference, func),
            xcx_jit_json_get: module.declare_func_in_func(self.xcx_jit_json_get, func),
            xcx_jit_json_set: module.declare_func_in_func(self.xcx_jit_json_set, func),
            xcx_jit_json_push: module.declare_func_in_func(self.xcx_jit_json_push, func),
            xcx_jit_json_get_push: module.declare_func_in_func(self.xcx_jit_json_get_push, func),
            xcx_jit_cast_string: module.declare_func_in_func(self.xcx_jit_cast_string, func),
            xcx_jit_cast_bool: module.declare_func_in_func(self.xcx_jit_cast_bool, func),
            xcx_jit_cast_int: module.declare_func_in_func(self.xcx_jit_cast_int, func),
            xcx_jit_cast_float: module.declare_func_in_func(self.xcx_jit_cast_float, func),
            xcx_jit_method_call_custom: module.declare_func_in_func(self.xcx_jit_method_call_custom, func),
            xcx_jit_array_get_int: module.declare_func_in_func(self.xcx_jit_array_get_int, func),
            xcx_jit_array_set_int: module.declare_func_in_func(self.xcx_jit_array_set_int, func),
            xcx_jit_string_starts_with: module.declare_func_in_func(self.xcx_jit_string_starts_with, func),
            xcx_jit_string_ends_with: module.declare_func_in_func(self.xcx_jit_string_ends_with, func),
            
            xcx_jit_string_upper: module.declare_func_in_func(self.xcx_jit_string_upper, func),
            xcx_jit_string_lower: module.declare_func_in_func(self.xcx_jit_string_lower, func),
            xcx_jit_string_trim: module.declare_func_in_func(self.xcx_jit_string_trim, func),
            xcx_jit_string_slice: module.declare_func_in_func(self.xcx_jit_string_slice, func),
            xcx_jit_string_replace: module.declare_func_in_func(self.xcx_jit_string_replace, func),
            xcx_jit_string_index_of: module.declare_func_in_func(self.xcx_jit_string_index_of, func),
            xcx_jit_string_last_index_of: module.declare_func_in_func(self.xcx_jit_string_last_index_of, func),
            xcx_jit_string_to_int: module.declare_func_in_func(self.xcx_jit_string_to_int, func),
            xcx_jit_string_to_float: module.declare_func_in_func(self.xcx_jit_string_to_float, func),
            xcx_jit_map_size: module.declare_func_in_func(self.xcx_jit_map_size, func),
            xcx_jit_map_contains: module.declare_func_in_func(self.xcx_jit_map_contains, func),
            xcx_jit_map_get: module.declare_func_in_func(self.xcx_jit_map_get, func),
            xcx_jit_map_insert: module.declare_func_in_func(self.xcx_jit_map_insert, func),
            xcx_jit_map_remove: module.declare_func_in_func(self.xcx_jit_map_remove, func),
            xcx_jit_map_clear: module.declare_func_in_func(self.xcx_jit_map_clear, func),
            xcx_jit_map_keys: module.declare_func_in_func(self.xcx_jit_map_keys, func),
            xcx_jit_map_values: module.declare_func_in_func(self.xcx_jit_map_values, func),
            xcx_jit_array_pop: module.declare_func_in_func(self.xcx_jit_array_pop, func),
            xcx_jit_array_clear: module.declare_func_in_func(self.xcx_jit_array_clear, func),
            xcx_jit_array_is_empty: module.declare_func_in_func(self.xcx_jit_array_is_empty, func),
            xcx_jit_array_contains: module.declare_func_in_func(self.xcx_jit_array_contains, func),
            xcx_jit_array_find: module.declare_func_in_func(self.xcx_jit_array_find, func),
            xcx_jit_array_insert: module.declare_func_in_func(self.xcx_jit_array_insert, func),
            xcx_jit_array_delete: module.declare_func_in_func(self.xcx_jit_array_delete, func),
            xcx_jit_array_sort: module.declare_func_in_func(self.xcx_jit_array_sort, func),
            xcx_jit_array_reverse: module.declare_func_in_func(self.xcx_jit_array_reverse, func),
            xcx_jit_date_field: module.declare_func_in_func(self.xcx_jit_date_field, func),
            xcx_jit_check_recursion: module.declare_func_in_func(self.xcx_jit_check_recursion, func),
            xcx_jit_dec_recursion: module.declare_func_in_func(self.xcx_jit_dec_recursion, func),
        }
    }
}

pub struct ImportedSymbols {
    pub xcx_jit_random_int: FuncRef,
    pub xcx_jit_random_float: FuncRef,
    pub xcx_jit_pow_int: FuncRef,
    pub xcx_jit_pow_float: FuncRef,
    pub xcx_jit_int_concat: FuncRef,
    pub xcx_jit_has: FuncRef,
    pub xcx_jit_random_choice: FuncRef,
    pub xcx_jit_array_size: FuncRef,
    pub xcx_jit_array_get: FuncRef,
    pub xcx_jit_array_push: FuncRef,
    pub xcx_jit_array_update: FuncRef,
    pub xcx_jit_array_set_bool: FuncRef,
    pub xcx_jit_array_get_bool: FuncRef,
    pub xcx_jit_call_recursive: FuncRef,
    pub xcx_jit_set_size: FuncRef,
    pub xcx_jit_set_contains: FuncRef,
    pub xcx_jit_set_values: FuncRef,
    pub xcx_jit_inc_ref: FuncRef,
    pub xcx_jit_dec_ref: FuncRef,
    pub xcx_jit_dec_ref_range: FuncRef,
    pub xcx_jit_method_dispatch: FuncRef,
    pub xcx_jit_method_dispatch_named: FuncRef,
    pub xcx_jit_fiber_is_done: FuncRef,
    pub xcx_jit_fiber_next: FuncRef,
    pub xcx_jit_fiber_run: FuncRef,
    pub xcx_jit_add: FuncRef,
    pub xcx_jit_sub: FuncRef,
    pub xcx_jit_mul: FuncRef,
    pub xcx_jit_div: FuncRef,
    pub xcx_jit_mod: FuncRef,
    pub xcx_jit_abort_div: FuncRef,
    pub xcx_jit_has_errors: FuncRef,
    pub xcx_jit_neg: FuncRef,
    pub xcx_jit_eq: FuncRef,
    pub xcx_jit_ne: FuncRef,
    pub xcx_jit_gt: FuncRef,
    pub xcx_jit_lt: FuncRef,
    pub xcx_jit_ge: FuncRef,
    pub xcx_jit_le: FuncRef,
    pub xcx_jit_row_get: FuncRef,
    pub xcx_jit_table_size: FuncRef,
    pub xcx_jit_table_get_row: FuncRef,
    pub xcx_jit_table_push_row: FuncRef,
    pub xcx_jit_table_clone_skeleton: FuncRef,
    pub xcx_jit_json_bind: FuncRef,
    pub xcx_jit_json_bind_const: FuncRef,
    pub xcx_jit_get_member: FuncRef,
    pub xcx_jit_set_fiber_state: FuncRef,
    pub xcx_jit_report_guard_failure: FuncRef,
    pub xcx_jit_wait: FuncRef,
    pub xcx_jit_string_length: FuncRef,
    pub xcx_jit_json_parse: FuncRef,
    pub xcx_jit_json_to_str: FuncRef,
    pub xcx_jit_date_now: FuncRef,
    pub xcx_jit_perf_ms: FuncRef,
    pub xcx_jit_perf_us: FuncRef,
    pub xcx_jit_perf_ns: FuncRef,
    pub xcx_jit_array_init: FuncRef,
    pub xcx_jit_set_init: FuncRef,
    pub xcx_jit_map_init: FuncRef,
    pub xcx_jit_table_init: FuncRef,
    pub xcx_jit_net_call: FuncRef,
    pub xcx_jit_net_request: FuncRef,
    
    pub xcx_jit_terminal_clear: FuncRef,
    pub xcx_jit_terminal_raw: FuncRef,
    pub xcx_jit_terminal_normal: FuncRef,
    pub xcx_jit_terminal_cursor: FuncRef,
    pub xcx_jit_terminal_move: FuncRef,
    pub xcx_jit_terminal_exit: FuncRef,
    pub xcx_jit_terminal_run: FuncRef,
    pub xcx_jit_terminal_write: FuncRef,

    pub xcx_jit_print: FuncRef,
    pub xcx_jit_halt_alert: FuncRef,
    pub xcx_jit_halt_error: FuncRef,
    pub xcx_jit_halt_fatal: FuncRef,
    pub xcx_jit_typeof: FuncRef,
    pub xcx_jit_store_read: FuncRef,
    pub xcx_jit_store_write: FuncRef,
    pub xcx_jit_store_append: FuncRef,
    pub xcx_jit_store_exists: FuncRef,
    pub xcx_jit_store_delete: FuncRef,
    pub xcx_jit_database_init: FuncRef,
    pub xcx_jit_set_member: FuncRef,
    pub xcx_jit_env_get: FuncRef,
    pub xcx_jit_env_args: FuncRef,
    pub xcx_jit_crypto_hash: FuncRef,
    pub xcx_jit_crypto_verify: codegen::ir::FuncRef,
    pub xcx_jit_crypto_token: codegen::ir::FuncRef,
    pub xcx_jit_fiber_create: codegen::ir::FuncRef,
    pub xcx_jit_set_range: codegen::ir::FuncRef,
    pub xcx_jit_set_remove: FuncRef,
    pub xcx_jit_set_union: FuncRef,
    pub xcx_jit_set_intersection: FuncRef,
    pub xcx_jit_set_difference: FuncRef,
    pub xcx_jit_set_sym_difference: FuncRef,
    pub xcx_jit_json_get: FuncRef,
    pub xcx_jit_json_set: FuncRef,
    pub xcx_jit_json_push: FuncRef,
    pub xcx_jit_json_get_push: FuncRef,
    pub xcx_jit_cast_string: FuncRef,
    pub xcx_jit_cast_bool: FuncRef,
    pub xcx_jit_cast_int: FuncRef,
    pub xcx_jit_cast_float: FuncRef,
    pub xcx_jit_method_call_custom: FuncRef,
    pub xcx_jit_array_get_int: FuncRef,
    pub xcx_jit_array_set_int: FuncRef,
    pub xcx_jit_string_starts_with: FuncRef,
    pub xcx_jit_string_ends_with: FuncRef,
    pub xcx_jit_string_upper: FuncRef,
    pub xcx_jit_string_lower: FuncRef,
    pub xcx_jit_string_trim: FuncRef,
    pub xcx_jit_string_slice: FuncRef,
    pub xcx_jit_string_replace: FuncRef,
    pub xcx_jit_string_index_of: FuncRef,
    pub xcx_jit_string_last_index_of: FuncRef,
    pub xcx_jit_string_to_int: FuncRef,
    pub xcx_jit_string_to_float: FuncRef,
    pub xcx_jit_map_size: FuncRef,
    pub xcx_jit_map_contains: FuncRef,
    pub xcx_jit_map_get: FuncRef,
    pub xcx_jit_map_insert: FuncRef,
    pub xcx_jit_map_remove: FuncRef,
    pub xcx_jit_map_clear: FuncRef,
    pub xcx_jit_map_keys: FuncRef,
    pub xcx_jit_map_values: FuncRef,
    pub xcx_jit_array_pop: FuncRef,
    pub xcx_jit_array_clear: FuncRef,
    pub xcx_jit_array_is_empty: FuncRef,
    pub xcx_jit_array_contains: FuncRef,
    pub xcx_jit_array_find: FuncRef,
    pub xcx_jit_array_insert: FuncRef,
    pub xcx_jit_array_delete: FuncRef,
    pub xcx_jit_array_sort: FuncRef,
    pub xcx_jit_array_reverse: FuncRef,
    pub xcx_jit_date_field: FuncRef,
    pub xcx_jit_check_recursion: FuncRef,
    pub xcx_jit_dec_recursion: FuncRef,
    pub xcx_jit_yield: FuncRef,
    pub xcx_jit_http_serve: FuncRef,
    pub xcx_jit_http_respond: FuncRef,
}
