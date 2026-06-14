use cranelift::prelude::*;
use cranelift_jit::JITBuilder;
use cranelift_codegen as codegen;

use crate::vm::core::jit_helpers::*;
use crate::runtime::ffi_helpers::json_ffi::*;


pub fn create_jit_builder() -> JITBuilder {
    let mut flag_builder = codegen::settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    flag_builder.set("is_pic", "false").unwrap();
    flag_builder.set("opt_level", "speed").unwrap();
    flag_builder.set("enable_alias_analysis", "true").unwrap();
    flag_builder.set("regalloc_checker", "false").unwrap();

    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
        panic!("host machine is not supported: {}", msg);
    });
    let isa = isa_builder
        .finish(codegen::settings::Flags::new(flag_builder))
        .unwrap();

    let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    builder.symbol("xcx_jit_random_int", xcx_jit_random_int as *const u8);
    builder.symbol("xcx_jit_random_float", xcx_jit_random_float as *const u8);
    builder.symbol("xcx_jit_pow_int", xcx_jit_pow_int as *const u8);
    builder.symbol("xcx_jit_pow_float", xcx_jit_pow_float as *const u8);
    builder.symbol("xcx_jit_int_concat", xcx_jit_int_concat as *const u8);
    builder.symbol("xcx_jit_has", xcx_jit_has as *const u8);
    builder.symbol("xcx_jit_random_choice", xcx_jit_random_choice as *const u8);
    builder.symbol("xcx_jit_array_size", xcx_jit_array_size as *const u8);
    builder.symbol("xcx_jit_array_get", xcx_jit_array_get as *const u8);
    builder.symbol("xcx_jit_array_push", xcx_jit_array_push as *const u8);
    builder.symbol("xcx_jit_array_update", xcx_jit_array_update as *const u8);
    builder.symbol("xcx_jit_array_set_bool", xcx_jit_array_set_bool as *const u8);
    builder.symbol("xcx_jit_array_get_bool", xcx_jit_array_get_bool as *const u8);
    builder.symbol("xcx_jit_call_recursive", xcx_jit_call_recursive as *const u8);
    builder.symbol("xcx_jit_set_size", xcx_jit_set_size as *const u8);
    builder.symbol("xcx_jit_set_contains", xcx_jit_set_contains as *const u8);
    builder.symbol("xcx_jit_set_values", xcx_jit_set_values as *const u8);
    builder.symbol("xcx_jit_inc_ref", xcx_jit_inc_ref as *const u8);
    builder.symbol("xcx_jit_dec_ref", xcx_jit_dec_ref as *const u8);
    builder.symbol("xcx_jit_dec_ref_range", xcx_jit_dec_ref_range as *const u8);
    builder.symbol("xcx_jit_method_dispatch", xcx_jit_method_dispatch as *const u8);
    builder.symbol("xcx_jit_method_dispatch_named", xcx_jit_method_dispatch_named as *const u8);
    builder.symbol("xcx_jit_method_call_custom", xcx_jit_method_call_custom as *const u8);
    builder.symbol("xcx_jit_add", xcx_jit_add as *const u8);
    builder.symbol("xcx_jit_sub", xcx_jit_sub as *const u8);
    builder.symbol("xcx_jit_mul", xcx_jit_mul as *const u8);
    builder.symbol("xcx_jit_div", xcx_jit_div as *const u8);
    builder.symbol("xcx_jit_mod", xcx_jit_mod as *const u8);
    builder.symbol("xcx_jit_abort_div", xcx_jit_abort_div as *const u8);
    builder.symbol("xcx_jit_has_errors", xcx_jit_has_errors as *const u8);
    builder.symbol("xcx_jit_neg", xcx_jit_neg as *const u8);
    builder.symbol("xcx_jit_eq", xcx_jit_eq as *const u8);
    builder.symbol("xcx_jit_ne", xcx_jit_ne as *const u8);
    builder.symbol("xcx_jit_gt", xcx_jit_gt as *const u8);
    builder.symbol("xcx_jit_lt", xcx_jit_lt as *const u8);
    builder.symbol("xcx_jit_ge", xcx_jit_ge as *const u8);
    builder.symbol("xcx_jit_le", xcx_jit_le as *const u8);
    builder.symbol("xcx_jit_row_get", xcx_jit_row_get as *const u8);
    builder.symbol("xcx_jit_table_size", xcx_jit_table_size as *const u8);
    builder.symbol("xcx_jit_table_get_row", xcx_jit_table_get_row as *const u8);
    builder.symbol("xcx_jit_table_push_row", xcx_jit_table_push_row as *const u8);
    builder.symbol("xcx_jit_get_member", xcx_jit_get_member as *const u8);
    builder.symbol("xcx_jit_set_fiber_state", xcx_jit_set_fiber_state as *const u8);
    builder.symbol("xcx_jit_fiber_create", xcx_jit_fiber_create as *const u8);
    
    builder.symbol("xcx_jit_check_recursion", xcx_jit_check_recursion as *const u8);
    builder.symbol("xcx_jit_dec_recursion", xcx_jit_dec_recursion as *const u8);

    use crate::runtime::ffi_helpers::terminal_ffi::*;
    builder.symbol("xcx_jit_terminal_clear", xcx_jit_terminal_clear as *const u8);
    builder.symbol("xcx_jit_terminal_raw", xcx_jit_terminal_raw as *const u8);
    builder.symbol("xcx_jit_terminal_normal", xcx_jit_terminal_normal as *const u8);
    builder.symbol("xcx_jit_terminal_cursor", xcx_jit_terminal_cursor as *const u8);
    builder.symbol("xcx_jit_terminal_move", xcx_jit_terminal_move as *const u8);
    builder.symbol("xcx_jit_terminal_exit", xcx_jit_terminal_exit as *const u8);
    builder.symbol("xcx_jit_terminal_run", xcx_jit_terminal_run as *const u8);
    builder.symbol("xcx_jit_terminal_write", xcx_jit_terminal_write as *const u8);

    builder.symbol("xcx_jit_report_guard_failure", xcx_jit_report_guard_failure as *const u8);
    builder.symbol("xcx_jit_wait", xcx_jit_wait as *const u8);
    builder.symbol("xcx_jit_fiber_is_done", xcx_jit_fiber_is_done as *const u8);
    builder.symbol("xcx_jit_fiber_next", xcx_jit_fiber_next as *const u8);
    builder.symbol("xcx_jit_fiber_run", xcx_jit_fiber_run as *const u8);
    builder.symbol("xcx_jit_json_bind", xcx_jit_json_bind as *const u8);
    builder.symbol("xcx_jit_json_bind_const", xcx_jit_json_bind_const as *const u8);
    builder.symbol("xcx_jit_get_member", xcx_jit_get_member as *const u8);
    builder.symbol("xcx_jit_string_length", xcx_jit_string_length as *const u8);
    builder.symbol("xcx_jit_json_parse", xcx_jit_json_parse as *const u8);
    builder.symbol("xcx_jit_json_to_str", xcx_jit_json_to_str as *const u8);
    builder.symbol("xcx_jit_date_now", xcx_jit_date_now as *const u8);
    builder.symbol("xcx_jit_perf_ms", xcx_jit_perf_ms as *const u8);
    builder.symbol("xcx_jit_perf_us", xcx_jit_perf_us as *const u8);
    builder.symbol("xcx_jit_perf_ns", xcx_jit_perf_ns as *const u8);
    builder.symbol("xcx_jit_cast_string", xcx_jit_cast_string as *const u8);
    builder.symbol("xcx_jit_cast_int", xcx_jit_cast_int as *const u8);
    builder.symbol("xcx_jit_cast_float", xcx_jit_cast_float as *const u8);

    builder.symbol("xcx_jit_array_init", xcx_jit_array_init as *const u8);
    builder.symbol("xcx_jit_set_init", xcx_jit_set_init as *const u8);
    builder.symbol("xcx_jit_map_init", xcx_jit_map_init as *const u8);
    builder.symbol("xcx_jit_table_init", xcx_jit_table_init as *const u8);

    builder.symbol("xcx_jit_print", xcx_jit_print as *const u8);
    builder.symbol("xcx_jit_halt_alert", xcx_jit_halt_alert as *const u8);
    builder.symbol("xcx_jit_halt_error", xcx_jit_halt_error as *const u8);
    builder.symbol("xcx_jit_halt_fatal", xcx_jit_halt_fatal as *const u8);
    builder.symbol("xcx_jit_typeof", xcx_jit_typeof as *const u8);
    builder.symbol("xcx_jit_store_read", xcx_jit_store_read as *const u8);
    builder.symbol("xcx_jit_store_write", xcx_jit_store_write as *const u8);
    builder.symbol("xcx_jit_store_append", xcx_jit_store_append as *const u8);
    builder.symbol("xcx_jit_store_exists", xcx_jit_store_exists as *const u8);
    builder.symbol("xcx_jit_store_delete", xcx_jit_store_delete as *const u8);
    builder.symbol("xcx_jit_database_init", xcx_jit_database_init as *const u8);
    builder.symbol("xcx_jit_set_member", xcx_jit_set_member as *const u8);
    builder.symbol("xcx_jit_env_get", xcx_jit_env_get as *const u8);
    builder.symbol("xcx_jit_env_args", xcx_jit_env_args as *const u8);
    builder.symbol("xcx_jit_crypto_hash", xcx_jit_crypto_hash as *const u8);
    builder.symbol("xcx_jit_crypto_verify", xcx_jit_crypto_verify as *const u8);
    builder.symbol("xcx_jit_crypto_token", xcx_jit_crypto_token as *const u8);
    builder.symbol("xcx_jit_set_range", xcx_jit_set_range as *const u8);
    builder.symbol("xcx_jit_set_remove", xcx_jit_set_remove as *const u8);
    builder.symbol("xcx_jit_set_union", xcx_jit_set_union as *const u8);
    builder.symbol("xcx_jit_set_intersection", xcx_jit_set_intersection as *const u8);
    builder.symbol("xcx_jit_set_difference", xcx_jit_set_difference as *const u8);
    builder.symbol("xcx_jit_set_sym_difference", xcx_jit_set_sym_difference as *const u8);
    builder.symbol("xcx_jit_json_get", xcx_jit_json_get as *const u8);
    builder.symbol("xcx_jit_json_set", xcx_jit_json_set as *const u8);
    builder.symbol("xcx_jit_json_push", xcx_jit_json_push as *const u8);
    builder.symbol("xcx_jit_json_get_push", xcx_jit_json_get_push as *const u8);
    builder.symbol("xcx_jit_array_get_int", xcx_jit_array_get_int as *const u8);
    builder.symbol("xcx_jit_array_set_int", xcx_jit_array_set_int as *const u8);
    builder.symbol("xcx_jit_string_starts_with", xcx_jit_string_starts_with as *const u8);
    builder.symbol("xcx_jit_string_ends_with", xcx_jit_string_ends_with as *const u8);
    
    use crate::runtime::ffi_helpers::array_ffi::*;
    builder.symbol("xcx_jit_array_pop", xcx_jit_array_pop as *const u8);
    builder.symbol("xcx_jit_array_clear", xcx_jit_array_clear as *const u8);
    builder.symbol("xcx_jit_array_is_empty", xcx_jit_array_is_empty as *const u8);
    builder.symbol("xcx_jit_array_contains", xcx_jit_array_contains as *const u8);
    builder.symbol("xcx_jit_array_find", xcx_jit_array_find as *const u8);
    builder.symbol("xcx_jit_array_insert", xcx_jit_array_insert as *const u8);
    builder.symbol("xcx_jit_array_delete", xcx_jit_array_delete as *const u8);
    builder.symbol("xcx_jit_array_sort", xcx_jit_array_sort as *const u8);
    builder.symbol("xcx_jit_array_reverse", xcx_jit_array_reverse as *const u8);
    
    use crate::runtime::ffi_helpers::date_ffi::*;
    builder.symbol("xcx_jit_date_field", xcx_jit_date_field as *const u8);
    
    use crate::runtime::ffi_helpers::string_ffi::*;
    builder.symbol("xcx_jit_string_upper", xcx_jit_string_upper as *const u8);
    builder.symbol("xcx_jit_string_lower", xcx_jit_string_lower as *const u8);
    builder.symbol("xcx_jit_string_trim", xcx_jit_string_trim as *const u8);
    builder.symbol("xcx_jit_string_slice", xcx_jit_string_slice as *const u8);
    builder.symbol("xcx_jit_string_replace", xcx_jit_string_replace as *const u8);
    builder.symbol("xcx_jit_string_index_of", xcx_jit_string_index_of as *const u8);
    builder.symbol("xcx_jit_string_last_index_of", xcx_jit_string_last_index_of as *const u8);
    builder.symbol("xcx_jit_string_to_int", xcx_jit_string_to_int as *const u8);
    builder.symbol("xcx_jit_string_to_float", xcx_jit_string_to_float as *const u8);
    
    use crate::runtime::ffi_helpers::map_ffi::*;
    builder.symbol("xcx_jit_map_size", xcx_jit_map_size as *const u8);
    builder.symbol("xcx_jit_map_contains", xcx_jit_map_contains as *const u8);
    builder.symbol("xcx_jit_map_get", xcx_jit_map_get as *const u8);
    builder.symbol("xcx_jit_map_insert", xcx_jit_map_insert as *const u8);
    builder.symbol("xcx_jit_map_remove", xcx_jit_map_remove as *const u8);
    builder.symbol("xcx_jit_map_clear", xcx_jit_map_clear as *const u8);
    builder.symbol("xcx_jit_map_keys", xcx_jit_map_keys as *const u8);
    builder.symbol("xcx_jit_map_values", xcx_jit_map_values as *const u8);

    
    use crate::runtime::builtin::net::client::{xcx_jit_net_call, xcx_jit_net_request};
    builder.symbol("xcx_jit_net_call", xcx_jit_net_call as *const u8);
    builder.symbol("xcx_jit_net_request", xcx_jit_net_request as *const u8);
    builder.symbol("xcx_jit_yield", xcx_jit_yield as *const u8);
    builder.symbol("xcx_jit_http_serve", xcx_jit_http_serve as *const u8);
    builder.symbol("xcx_jit_http_respond", xcx_jit_http_respond as *const u8);

    builder
}
