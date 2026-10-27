//! End-to-end suite: every test spawns the real CLI binary (subprocess) and
//! asserts on exit codes, stdout/stderr, or real network/HTTP behaviour.

#[path = "../common/mod.rs"]
mod common;

mod bundled_lib_include;
mod cli_contract;
mod fiber_loop_state;
mod halt_error_frame_recovery;
mod http_body_limit;
mod json_concurrency;
mod json_scalar_tostr;
mod json_view_mutation;
mod method_len_alias;
mod piped_output_plain;
mod ssrf_link_local;
mod string_float_concat_chain;
