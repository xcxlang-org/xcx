pub mod client;
pub mod server;
pub mod respond;

use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::core::vm::{VM, OpResult, SharedContext};

pub fn call(dst: u8, method_idx: u32, url_src: u8, body_src: u8, locals: &mut [Value], ctx_constants: &[Value]) -> OpResult {
    client::call(dst, method_idx, url_src, body_src, locals, ctx_constants)
}

pub fn request(dst: u8, arg_src: u8, locals: &mut [Value], http_req_val: Option<Value>) -> OpResult {
    client::request(dst, arg_src, locals, http_req_val)
}

pub fn respond(dst: u8, status_src: u8, body_src: u8, headers_src: u8, locals: &mut [Value], http_req: Option<Arc<std::sync::Mutex<Option<tiny_http::Request>>>>) -> OpResult {
    let status = locals[status_src as usize].as_i64() as u32;
    let body_val = locals[body_src as usize];
    let headers = locals[headers_src as usize];
    let res = respond::respond_impl(status, body_val, headers, http_req);
    // net.respond returns true on success in XCX 4.0
    locals[dst as usize] = Value::from_bool(true);
    res
}

pub fn serve(func_idx: u32, port_src: u8, host_src: u8, _workers_src: u8, routes_src: u8, locals: &mut [Value], ctx: &SharedContext, vm_arc: &Arc<VM>) -> OpResult {

    let port = locals[port_src as usize].as_i64() as u16;
    let host = locals[host_src as usize].to_string();
    let routes = locals[routes_src as usize];
    server::serve_impl(func_idx, port, host, routes, ctx, vm_arc)
}
