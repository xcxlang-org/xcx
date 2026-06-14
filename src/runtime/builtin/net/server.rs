use std::sync::Arc;
use std::io::Write;
use crate::vm::value::Value;
use crate::vm::core::vm::{VM, OpResult, SharedContext};

/// Starts an HTTP server.
pub fn serve_impl(
    _func_idx: u32,
    port: u16,
    host: String,
    routes: Value,
    ctx: &SharedContext,
    vm_arc: &Arc<VM>,
) -> OpResult {
    let addr = format!("{}:{}", host, port);

    let _ = std::io::stdout().flush();
    let server = match tiny_http::Server::http(&addr) {
        Ok(s) => {

            let _ = std::io::stdout().flush();
            s
        },
        Err(e) => {
            eprintln!("XCX Server ERROR: Failed to start on {}: {}", addr, e);
            let _ = std::io::stdout().flush();
            return OpResult::Halt;
        }
    };

    let ctx_base = ctx.clone();
    let vm = vm_arc.clone();

    loop {
        // XCX: Check shutdown signal gracefully without blocking forever
        if crate::vm::core::executor::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        let request = match server.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(Some(req)) => req,
            Ok(None) => continue,
            Err(_) => break, // Server unblocked or error
        };
        let vm = vm.clone();
        let ctx_sub = ctx_base.clone();
        let req_arc = Arc::new(std::sync::Mutex::new(Some(request)));
        let req_arc_for_lookup = req_arc.clone();
        
        let routes_copy = routes;
        if routes_copy.is_ptr() { unsafe { routes_copy.inc_ref(); } }

        std::thread::spawn(move || {
            let mut handler_chunk: Option<Arc<crate::vm::opcode::Chunk>> = None;
            let route_key = {
                let guard = req_arc_for_lookup.lock().unwrap();
                if let Some(req) = guard.as_ref() {
                    let url_str = req.url().to_string();
                    let url_parts: Vec<&str> = url_str.split('?').collect();
                    let raw = format!("{} {}", req.method(), url_parts[0]);
                    // Normalize multiple whitespace to single space
                    raw.split_whitespace().collect::<Vec<&str>>().join(" ")
                } else { String::new() }
            };

            let mut matched_val: Option<Value> = None;

            if routes_copy.is_map() {
                let map_rc = routes_copy.as_map();
                let map = map_rc.read();
                for (k, v) in map.elements.iter() {
                    let k_str: String = k.to_string().split_whitespace().collect::<Vec<&str>>().join(" ");
                    let is_match = if k_str == route_key {
                        true
                    } else if k_str == "*" {
                        true
                    } else if k_str.ends_with('*') {
                        let prefix = k_str[..k_str.len() - 1].trim();
                        route_key.starts_with(prefix)
                    } else {
                        false
                    };
                    if is_match {
                        matched_val = Some(*v);
                        break;
                    }
                }
            } else if routes_copy.is_array() {
                let arr_rc = routes_copy.as_array();
                let arr = arr_rc.read();
                'outer: for item in arr.elements.iter() {
                    if item.is_map() {
                        let map_rc = item.as_map();
                        let map = map_rc.read();
                        for (k, v) in map.elements.iter() {
                            let k_str: String = k.to_string().split_whitespace().collect::<Vec<&str>>().join(" ");
                            let is_match = if k_str == route_key {
                                true
                            } else if k_str == "*" {
                                true
                            } else if k_str.ends_with('*') {
                                let prefix = k_str[..k_str.len() - 1].trim();
                                route_key.starts_with(prefix)
                            } else {
                                false
                            };
                            if is_match {
                                matched_val = Some(*v);
                                break 'outer;
                            }
                        }
                    }
                }
            }

            if let Some(v) = matched_val {
                if v.is_func() {
                    let idx = if v.tag == crate::vm::value::TAG_FUNC_PTR {
                        v.as_function().chunk_idx
                    } else {
                        v.as_function_idx()
                    };
                    handler_chunk = Some(ctx_sub.functions[idx as usize].clone());
                } else if v.is_fiber() {
                    let fiber_lock = v.as_fiber();
                    let fiber = fiber_lock.read();
                    handler_chunk = Some(ctx_sub.functions[fiber.func_id].clone());
                }
            }

            if let Some(chunk) = handler_chunk {
                let mut req_guard = req_arc.lock().unwrap();
                let req_obj = if let Some(req) = req_guard.as_mut() {
                    let mut headers_obj = Vec::new();
                    for header in req.headers() {
                        headers_obj.push((std::sync::Arc::new(header.field.to_string().to_lowercase()), crate::vm::object::JsonVal::String(Arc::new(header.value.to_string()))));
                    }
                    let mut body_bytes = Vec::new();
                    let _ = req.as_reader().read_to_end(&mut body_bytes);
                    let body_str = String::from_utf8_lossy(&body_bytes).into_owned();
                    let body_val = if let Ok(json_body) = serde_json::from_str::<serde_json::Value>(&body_str) {
                        crate::vm::object::JsonVal::from_serde(json_body)
                    } else {
                        crate::vm::object::JsonVal::String(Arc::new(body_str))
                    };

                    let url_str = req.url().to_string();
                    let url_parts: Vec<&str> = url_str.split('?').collect();
                    let path_only = url_parts[0].to_string();
                    
                    let mut query_obj = Vec::new();
                    if url_parts.len() > 1 {
                        for pair in url_parts[1].split('&') {
                            let kv: Vec<&str> = pair.split('=').collect();
                            if kv.len() == 2 {
                                let k = percent_decode(kv[0]);
                                let v = percent_decode(kv[1]);
                                query_obj.push((std::sync::Arc::new(k), crate::vm::object::JsonVal::String(Arc::new(v))));
                            } else if kv.len() == 1 {
                                let k = percent_decode(kv[0]);
                                query_obj.push((std::sync::Arc::new(k), crate::vm::object::JsonVal::String(Arc::new(String::new()))));
                            }
                        }
                    }

                    let client_ip = req.remote_addr()
                        .map(|addr| addr.ip().to_string())
                        .unwrap_or_else(|| "127.0.0.1".to_string());

                    let mut obj = Vec::new();
                    obj.push((std::sync::Arc::new("method".to_string()), crate::vm::object::JsonVal::String(Arc::new(req.method().to_string()))));
                    obj.push((std::sync::Arc::new("path".to_string()), crate::vm::object::JsonVal::String(Arc::new(path_only))));
                    obj.push((std::sync::Arc::new("query".to_string()), crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(query_obj)))));
                    obj.push((std::sync::Arc::new("headers".to_string()), crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(headers_obj)))));
                    obj.push((std::sync::Arc::new("body".to_string()), body_val));
                    obj.push((std::sync::Arc::new("ip".to_string()), crate::vm::object::JsonVal::String(Arc::new(client_ip))));
                    
                    Value::from_json(Arc::new(crate::vm::object::JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(obj))))))
                } else { Value::from_bool(false) };
                drop(req_guard);

                let mut thread_ctx = ctx_sub;
                let req_arc_for_fallback = req_arc.clone();
                thread_ctx.http_req = Some(req_arc);
                vm.run(chunk, thread_ctx, &[req_obj]);
                unsafe { req_obj.dec_ref(); }

                // Fallback: if server handler finished or panicked without sending response
                let mut fallback_guard = req_arc_for_fallback.lock().unwrap();
                if let Some(req) = fallback_guard.take() {
                    let response = tiny_http::Response::from_string("Internal Server Error").with_status_code(500);
                    let _ = req.respond(response);
                }
            } else {
                let mut guard = req_arc_for_lookup.lock().unwrap();
                if let Some(req) = guard.take() {
                    let response = tiny_http::Response::from_string("Not Found").with_status_code(404);
                    let _ = req.respond(response);
                }
            }

            if routes_copy.is_ptr() { unsafe { routes_copy.dec_ref(); } }
        });
    }
    OpResult::Continue
}

fn percent_decode(s: &str) -> String {
    let mut bytes = Vec::new();
    let chars = s.as_bytes().iter().copied().collect::<Vec<u8>>();
    let mut i = 0;
    while i < chars.len() {
        let b = chars[i];
        if b == b'%' && i + 2 < chars.len() {
            let h = chars[i + 1];
            let l = chars[i + 2];
            if let Ok(hex) = std::str::from_utf8(&[h, l]) {
                if let Ok(val) = u8::from_str_radix(hex, 16) {
                    bytes.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        if b == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(b);
        }
        i += 1;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

