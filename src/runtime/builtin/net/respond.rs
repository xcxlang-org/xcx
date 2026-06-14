use std::sync::Arc;
use crate::vm::value::{Value, TAG_STR, TAG_JSON, TAG_ARR, TAG_MAP, TAG_TBL};
use crate::vm::core::vm::OpResult;
use crate::vm::utils::json::value_to_json;

/// Responds to an incoming HTTP request (server-side).
pub fn respond_impl(status: u32, body_val: Value, headers: Value, http_req: Option<Arc<std::sync::Mutex<Option<tiny_http::Request>>>>) -> OpResult {
    let (body_bytes, _is_binary) = if body_val.is_ptr() && body_val.tag == TAG_STR {
        (Arc::new(body_val.as_string().data.clone()), true)
    } else if body_val.is_ptr() && body_val.tag == TAG_JSON {
        let mut buf = String::new();
        body_val.as_json().root.to_string_buf(&mut buf);
        (Arc::new(buf.into_bytes()), false)
    } else if body_val.is_ptr() && (body_val.tag == TAG_ARR || 
                                 body_val.tag == TAG_MAP ||
                                 body_val.tag == TAG_TBL) {
        let mut buf = String::new();
        value_to_json(&body_val).to_string_buf(&mut buf);
        (Arc::new(buf.into_bytes()), false)
    } else {
        (Arc::new(body_val.to_string().into_bytes()), false)
    };
    
    if let Some(req_mutex_arc) = http_req {
        if let Ok(mut req_guard) = req_mutex_arc.lock() {
            if let Some(request) = req_guard.take() {
                let body_owned = body_bytes.to_vec();
                let mut response = tiny_http::Response::from_data(body_owned)
                    .with_status_code(status);
            
                let mut ct_set = false;
                let mut cors_orig = false;
                let mut cors_meth = false;
                let mut cors_head = false;
                
                if headers.is_map() {
                    let map_rc = headers.as_map();
                    let map = map_rc.read();
                    for (k, v) in map.elements.iter() {
                        let ks = k.as_string_lossy();
                        let vs = v.as_string_lossy();
                        match ks.to_lowercase().as_str() {
                            "content-type" => { ct_set = true; }
                            "access-control-allow-origin" => { cors_orig = true; }
                            "access-control-allow-methods" => { cors_meth = true; }
                            "access-control-allow-headers" => { cors_head = true; }
                            _ => {}
                        }
                        if let Ok(h) = tiny_http::Header::from_bytes(ks.as_bytes(), vs.as_bytes()) {
                            response = response.with_header(h);
                        }
                    }
                } else if headers.is_array() {
                    let arr_rc = headers.as_array();
                    let guard = arr_rc.read();
                    for item in guard.elements.iter() {
                        if item.is_map() {
                            let map_rc = item.as_map();
                            let map = map_rc.read();
                            for (k, v) in map.elements.iter() {
                                let ks = k.as_string_lossy();
                                let vs = v.as_string_lossy();
                                match ks.to_lowercase().as_str() {
                                    "content-type" => { ct_set = true; }
                                    "access-control-allow-origin" => { cors_orig = true; }
                                    "access-control-allow-methods" => { cors_meth = true; }
                                    "access-control-allow-headers" => { cors_head = true; }
                                    _ => {}
                                }
                                if let Ok(h) = tiny_http::Header::from_bytes(ks.as_bytes(), vs.as_bytes()) {
                                    response = response.with_header(h);
                                }
                            }
                        }
                    }
                }
                
                if !ct_set {
                    response = response.with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                }
                if !cors_orig {
                    response = response.with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                }
                if !cors_meth {
                    response = response.with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, OPTIONS, DELETE, PATCH"[..]).unwrap());
                }
                if !cors_head {
                    response = response.with_header(tiny_http::Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"Content-Type, Authorization, X-CSRF-TOKEN"[..]).unwrap());
                }
                
                match request.respond(response) {
                    Ok(_) => {},
                    Err(_e) => {},
                }
            } else {

            }
        }
    } else {

    }
    OpResult::Continue
}
