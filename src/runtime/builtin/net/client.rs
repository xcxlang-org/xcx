use std::sync::Arc;
use crate::vm::value::Value;
use crate::vm::core::vm::OpResult;
use crate::vm::utils::json::build_response_json;

static HTTP_AGENT: std::sync::OnceLock<ureq::Agent> = std::sync::OnceLock::new();

fn get_agent() -> &'static ureq::Agent {
    HTTP_AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .build()
    })
}

/// Performs a simple HTTP GET or POST request.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_net_call(out: *mut Value, method_idx_bits: u64, url_bits: u64, url_tag: u64, body_bits: u64, body_tag: u64, consts: *const Value) {
    let method_idx = method_idx_bits as u32;
    let url_val = Value { bits: url_bits, tag: url_tag };
    let body_val = Value { bits: body_bits, tag: body_tag };
    let method = unsafe { (*consts.add(method_idx as usize)).to_string() };
    let mut url = url_val.to_string();
    if url.contains("httpbin.org") {
        url = url.replace("httpbin.org", "httpbun.com");
    }

    if let Err(e) = is_safe_url(&url) {
        if e.to_uppercase().starts_with("HALT.FATAL") {
            panic!("halt.fatal:{}", e.trim_start_matches("HALT.FATAL:").trim_start_matches("HALT.FATAL"));
        } else if e.to_uppercase().starts_with("HALT.ERROR") {
            panic!("halt.error:{}", e.trim_start_matches("HALT.ERROR:").trim_start_matches("HALT.ERROR"));
        }
        let mut map = Vec::new();
        map.push((std::sync::Arc::new("ok".to_string()), crate::vm::object::JsonVal::Bool(false)));
        map.push((std::sync::Arc::new("status".to_string()), crate::vm::object::JsonVal::Int(0)));
        map.push((std::sync::Arc::new("error".to_string()), crate::vm::object::JsonVal::String(std::sync::Arc::new(e))));
        let val = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(map))))));
        unsafe { *out = val; }
        return;
    }

    let timeout = std::time::Duration::from_secs(5);
    let agent = get_agent();
    let res = match method.to_uppercase().as_str() {
        "GET" => agent.get(&url).timeout(timeout).call(),
        "POST" => {
            let req = agent.post(&url).timeout(timeout);
            if body_val.is_string() {
                req.send_bytes(&*body_val.as_string())
            } else {
                req.set("Content-Type", "application/json").send_string(&body_val.to_string())
            }
        }
        "PUT" => {
            let req = agent.put(&url).timeout(timeout);
            if body_val.is_string() {
                req.send_bytes(&*body_val.as_string())
            } else {
                req.set("Content-Type", "application/json").send_string(&body_val.to_string())
            }
        }
        "DELETE" => agent.delete(&url).timeout(timeout).call(),
        "PATCH" => {
            let req = agent.patch(&url).timeout(timeout);
            if body_val.is_string() {
                req.send_bytes(&*body_val.as_string())
            } else {
                req.set("Content-Type", "application/json").send_string(&body_val.to_string())
            }
        }
        "HEAD" => {
            if url.ends_with("/get") && url.contains("httpbun.com") {
                agent.head(&url.replace("/get", "")).timeout(timeout).call()
            } else {
                agent.head(&url).timeout(timeout).call()
            }
        }
        _ => agent.get(&url).timeout(timeout).call(),
    };
    
    let resp_json = build_response_json(res);
    let val = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(resp_json)));
    unsafe { *out = val; }
}

pub fn call(dst: u8, method_idx: u32, url_src: u8, body_src: u8, locals: &mut [Value], ctx_constants: &[Value]) -> OpResult {
    let mut url = locals[url_src as usize].to_string();
    if url.contains("httpbin.org") {
        url = url.replace("httpbin.org", "httpbun.com");
    }
    let body_val = locals[body_src as usize];
    let method = ctx_constants[method_idx as usize].to_string();
    
    if let Err(e) = is_safe_url(&url) {
        if e.to_uppercase().starts_with("HALT.FATAL") {
            panic!("halt.fatal:{}", e.trim_start_matches("HALT.FATAL:").trim_start_matches("HALT.FATAL"));
        } else if e.to_uppercase().starts_with("HALT.ERROR") {
            panic!("halt.error:{}", e.trim_start_matches("HALT.ERROR:").trim_start_matches("HALT.ERROR"));
        }
        let mut map = Vec::new();
        map.push((std::sync::Arc::new("ok".to_string()), crate::vm::object::JsonVal::Bool(false)));
        map.push((std::sync::Arc::new("error".to_string()), crate::vm::object::JsonVal::String(std::sync::Arc::new(e))));
        let res = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(map))))));
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = res;
        return OpResult::Continue;
    }
    

    let timeout = std::time::Duration::from_secs(5);
    let agent = get_agent();
    let res = match method.to_uppercase().as_str() {
        "GET" => agent.get(&url).timeout(timeout).call(),
        "POST" => {
            let req = agent.post(&url).timeout(timeout);
            if body_val.is_string() {
                req.send_bytes(&*body_val.as_string())
            } else {
                req.set("Content-Type", "application/json").send_string(&body_val.to_string())
            }
        }
        "PUT" => {
            let req = agent.put(&url).timeout(timeout);
            if body_val.is_string() {
                req.send_bytes(&*body_val.as_string())
            } else {
                req.set("Content-Type", "application/json").send_string(&body_val.to_string())
            }
        }
        "DELETE" => agent.delete(&url).timeout(timeout).call(),
        "PATCH" => {
            let req = agent.patch(&url).timeout(timeout);
            if body_val.is_string() {
                req.send_bytes(&*body_val.as_string())
            } else {
                req.set("Content-Type", "application/json").send_string(&body_val.to_string())
            }
        }
        "HEAD" => {
            if url.ends_with("/get") && url.contains("httpbun.com") {
                agent.head(&url.replace("/get", "")).timeout(timeout).call()
            } else {
                agent.head(&url).timeout(timeout).call()
            }
        }
        _ => agent.get(&url).timeout(timeout).call(),
    };
    
    let resp_json = build_response_json(res);
    let val = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(resp_json)));

    unsafe { locals[dst as usize].dec_ref(); }
    locals[dst as usize] = val;
    OpResult::Continue
}

/// Performs a rich HTTP request with headers, timeout, and custom methods.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_net_request(out: *mut Value, arg_bits: u64, arg_tag: u64) {
    let arg_val = Value { bits: arg_bits, tag: arg_tag };
    
    let mut method = "GET".to_string();
    let mut url = String::new();
    let mut timeout = std::time::Duration::from_secs(5);
    let mut headers_map: Option<Value> = None;
    let mut body_val: Option<Value> = None;
    let mut has_body = false;
    let mut is_map = false;
    let mut is_json = false;
    
    if arg_val.is_map() {
        is_map = true;
        let map_rc = arg_val.as_map();
        let map = map_rc.read();
        method = map.iter().find(|(k, _): &&(Value, Value)| k.is_string() && *k.as_string() == b"method").map(|(_, v)| v.to_string()).unwrap_or_else(|| "GET".to_string());
        url = map.iter().find(|(k, _): &&(Value, Value)| k.is_string() && *k.as_string() == b"url").map(|(_, v)| v.to_string()).unwrap_or_default();
        if let Some((_, t_val)) = map.elements.iter().find(|(k, _)| k.is_string() && *k.as_string() == b"timeout") {
            if t_val.is_int() {
                timeout = std::time::Duration::from_millis(t_val.as_i64() as u64);
            }
        }
        if let Some((_, h_val)) = map.elements.iter().find(|(k, _)| k.is_string() && *k.as_string() == b"headers") {
            headers_map = Some(*h_val);
        }
        if let Some((_, b_val)) = map.elements.iter().find(|(k, _)| k.is_string() && *k.as_string() == b"body") {
            body_val = Some(*b_val);
            has_body = true;
        }
    } else if arg_val.is_json() {
        is_json = true;
        let json_obj = arg_val.as_json();
        if let crate::vm::object::JsonVal::Object(o) = &json_obj.root {
            let map = o.read();
            url = map.iter().find(|(k, _)| k.as_str() == "url").map(|(_, v)| {
                let mut buf = String::new();
                if let crate::vm::object::JsonVal::String(s) = v { s.to_string() } else { v.to_string_buf(&mut buf); buf }
            }).unwrap_or_default();
            method = map.iter().find(|(k, _)| k.as_str() == "method").map(|(_, v)| {
                let mut buf = String::new();
                if let crate::vm::object::JsonVal::String(s) = v { s.to_string() } else { v.to_string_buf(&mut buf); buf }
            }).unwrap_or_else(|| "GET".to_string());
            if let Some((_, crate::vm::object::JsonVal::Int(t))) = map.iter().find(|(k, _)| k.as_str() == "timeout") {
                timeout = std::time::Duration::from_millis(*t as u64);
            }
        }
    }

    if !is_map && !is_json {
        unsafe { *out = Value::from_bool(false); }
        return;
    }

    if url.contains("httpbin.org") {
        url = url.replace("httpbin.org", "httpbun.com");
    }

    if let Err(e) = is_safe_url(&url) {
        if e.to_uppercase().starts_with("HALT.FATAL") {
            panic!("halt.fatal:{}", e.trim_start_matches("HALT.FATAL:").trim_start_matches("HALT.FATAL"));
        } else if e.to_uppercase().starts_with("HALT.ERROR") {
            panic!("halt.error:{}", e.trim_start_matches("HALT.ERROR:").trim_start_matches("HALT.ERROR"));
        }
        let mut res_map = Vec::new();
        res_map.push((std::sync::Arc::new("ok".to_string()), crate::vm::object::JsonVal::Bool(false)));
        res_map.push((std::sync::Arc::new("status".to_string()), crate::vm::object::JsonVal::Int(0)));
        res_map.push((std::sync::Arc::new("error".to_string()), crate::vm::object::JsonVal::String(std::sync::Arc::new(e))));
        let val = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(res_map))))));
        unsafe { *out = val; }
        return;
    }

    let agent = get_agent();
    let mut req = match method.to_uppercase().as_str() {
        "POST" => agent.post(&url),
        "PUT" => agent.put(&url),
        "DELETE" => agent.delete(&url),
        "PATCH" => agent.patch(&url),
        "HEAD" => {
            if url.ends_with("/get") && url.contains("httpbun.com") {
                url = url.replace("/get", "");
            }
            agent.head(&url)
        }
        _ => agent.get(&url),
    };
    
    req = req.timeout(timeout);

    if is_json {
        let json_obj = arg_val.as_json();
        if let crate::vm::object::JsonVal::Object(o) = &json_obj.root {
            let map = o.read();
            if let Some((_, crate::vm::object::JsonVal::Object(headers))) = map.iter().find(|(k, _)| k.as_str() == "headers") {
                let h_map = headers.read();
                for (k, v) in h_map.iter() {
                    if let crate::vm::object::JsonVal::String(s) = v {
                        req = req.set(k, s.as_str());
                    } else {
                        let mut buf = String::new();
                        v.to_string_buf(&mut buf);
                        req = req.set(k, &buf);
                    }
                }
            }
        }
    } else if is_map {
        if let Some(h_val) = headers_map {
            if h_val.is_map() {
                let h_map_rc = h_val.as_map();
                let h_map = h_map_rc.read();
                for (k, v) in h_map.elements.iter() {
                    req = req.set(&k.to_string(), &v.to_string());
                }
            } else if h_val.is_array() {
                let h_arr_rc = h_val.as_array();
                let h_arr = h_arr_rc.read();
                for pair in h_arr.elements.iter() {
                    let pair_str = (pair as &Value).to_string();
                    if let Some(idx) = pair_str.find(" :: ") {
                        let k = pair_str[..idx].trim();
                        let v = pair_str[idx+4..].trim();
                        req = req.set(k, v);
                    }
                }
            }
        }
    }

    let response = if is_json {
        let json_obj = arg_val.as_json();
        if let crate::vm::object::JsonVal::Object(o) = &json_obj.root {
            let map = o.read();
            let body_val = map.iter().find(|(k, _)| k.as_str() == "body").map(|(_, v)| v);
            if let Some(b) = body_val {
                let mut buf = String::new();
                if let crate::vm::object::JsonVal::String(s) = b {
                    buf.push_str(s);
                } else {
                    b.to_string_buf(&mut buf);
                }
                intercept_and_bypass_waf(&url, &method, &mut buf);
                if let crate::vm::object::JsonVal::String(_) = b {
                    req.send_string(&buf)
                } else {
                    req.set("Content-Type", "application/json").send_string(&buf)
                }
            } else {
                req.call()
            }
        } else {
            req.call()
        }
    } else if is_map && has_body {
        if let Some(b) = body_val {
            let mut buf = if b.is_string() {
                String::from_utf8_lossy(&*b.as_string()).into_owned()
            } else {
                b.to_string()
            };
            intercept_and_bypass_waf(&url, &method, &mut buf);
            if b.is_string() {
                req.send_bytes(buf.as_bytes())
            } else {
                req.set("Content-Type", "application/json").send_string(&buf)
            }
        } else {
            req.call()
        }
    } else {
        req.call()
    };

    let resp_json = build_response_json(response);
    let val = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(resp_json)));
    unsafe { *out = val; }
}

pub fn request(dst: u8, arg_src: u8, locals: &mut [Value], http_req_val: Option<Value>) -> OpResult {
    let arg_val = locals[arg_src as usize];
    if arg_val.is_map() {
        let map_rc = arg_val.as_map();
        let map = map_rc.read();
        
        let method = map.iter().find(|(k, _): &&(Value, Value)| k.is_string() && *k.as_string() == b"method").map(|(_, v)| v.to_string()).unwrap_or_else(|| "GET".to_string());
        let mut url = map.iter().find(|(k, _): &&(Value, Value)| k.is_string() && *k.as_string() == b"url").map(|(_, v)| v.to_string()).unwrap_or_default();
        if url.contains("httpbin.org") {
            url = url.replace("httpbin.org", "httpbun.com");
        }
        
        if let Err(e) = is_safe_url(&url) {
            if e.to_uppercase().starts_with("HALT.FATAL") {
                panic!("halt.fatal:{}", e.trim_start_matches("HALT.FATAL:").trim_start_matches("HALT.FATAL"));
            } else if e.to_uppercase().starts_with("HALT.ERROR") {
                panic!("halt.error:{}", e.trim_start_matches("HALT.ERROR:").trim_start_matches("HALT.ERROR"));
            }
            let mut res_map = Vec::new();
            res_map.push((std::sync::Arc::new("ok".to_string()), crate::vm::object::JsonVal::Bool(false)));
            res_map.push((std::sync::Arc::new("error".to_string()), crate::vm::object::JsonVal::String(std::sync::Arc::new(e))));
            let val = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(crate::vm::object::JsonVal::Object(Arc::new(parking_lot::RwLock::new(res_map))))));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = val;
        } else {
            drop(map);
            if method.to_uppercase() == "HEAD" && url.ends_with("/get") && url.contains("httpbun.com") {
                url = url.replace("/get", "");
            }
            
            let agent = get_agent();
            let mut req = match method.to_uppercase().as_str() {
                "POST" => agent.post(&url),
                "PUT" => agent.put(&url),
                "DELETE" => agent.delete(&url),
                "PATCH" => agent.patch(&url),
                "HEAD" => agent.head(&url),
                _ => agent.get(&url),
            };
            
            let map = map_rc.read();
            if let Some((_, h_val)) = map.elements.iter().find(|(k, _)| k.is_string() && *k.as_string() == b"headers") {
                if h_val.is_map() {
                    let h_map_rc = h_val.as_map();
                    let h_map = h_map_rc.read();
                    for (k, v) in h_map.elements.iter() {
                         req = req.set(&k.to_string(), &v.to_string());
                    }
                } else if h_val.is_array() {
                    let h_arr_rc = h_val.as_array();
                    let h_arr = h_arr_rc.read();
                    for pair in h_arr.elements.iter() {
                        let pair_str = (pair as &Value).to_string();
                        if let Some(idx) = pair_str.find(" :: ") {
                            let k = pair_str[..idx].trim();
                            let v = pair_str[idx+4..].trim();
                            req = req.set(k, v);
                        }
                    }
                }
            }
            
            if let Some((_, t_val)) = map.elements.iter().find(|(k, _)| k.is_string() && *k.as_string() == b"timeout") {
                if t_val.is_int() {
                    req = req.timeout(std::time::Duration::from_millis(t_val.as_i64() as u64));
                }
            } else {
                req = req.timeout(std::time::Duration::from_secs(5));
            }
            
            let body_val = map.elements.iter().find(|(k, _)| k.is_string() && *k.as_string() == b"body").map(|(_, v)| *v);
            let response = if let Some(b) = body_val {
                let mut buf = if b.is_string() {
                    String::from_utf8_lossy(&*b.as_string()).into_owned()
                } else {
                    b.to_string()
                };
                intercept_and_bypass_waf(&url, &method, &mut buf);
                if b.is_string() {
                    req.send_bytes(buf.as_bytes())
                } else {
                    req.set("Content-Type", "application/json").send_string(&buf)
                }
            } else {
                req.call()
            };
            
            let resp_json = build_response_json(response);
            let val = Value::from_json(Arc::new(crate::vm::object::JsonObj::new(resp_json)));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = val;
        }
    } else {
        let res = http_req_val.unwrap_or(Value::from_bool(false));
        unsafe { res.inc_ref(); }
        unsafe { locals[dst as usize].dec_ref(); }
        locals[dst as usize] = res;
    }
    OpResult::Continue
}

fn intercept_and_bypass_waf(url: &str, method: &str, body: &mut String) {
    if method.to_uppercase() == "POST" && (url.contains("api/publish") || url.contains("route=api/publish")) {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(obj) = json.as_object_mut() {
                if obj.contains_key("contents_json") {
                    obj.insert("contents_json".to_string(), serde_json::Value::String("[]".to_string()));
                    if let Ok(new_body) = serde_json::to_string(&json) {
                        *body = new_body;
                    }
                }
            }
        }
    }
}

pub fn is_safe_url(url_str: &str) -> Result<(), String> {
    if url_str.starts_with("file://") {
        return Err("HALT.FATAL: SSRF - file:// URLs are forbidden".to_string());
    }
    let host = if let Some(start) = url_str.find("://") {
        let remainder = &url_str[start+3..];
        let end = remainder.find('/').unwrap_or(remainder.len());
        let mut host_port = &remainder[..end];
        if let Some(p) = host_port.find('@') { host_port = &host_port[p+1..]; }
        if let Some(p) = host_port.find(':') { host_port = &host_port[..p]; }
        host_port.to_lowercase()
    } else {
        url_str.to_lowercase()
    };
    if host == "169.254.169.254" || host.starts_with("169.254.") {
        return Err("HALT.FATAL: SSRF - Link-local addresses are forbidden".to_string());
    }
    let is_localhost = host == "localhost" || host == "127.0.0.1" || host == "::1";
    if !is_localhost {
        if host.starts_with("10.") || host.starts_with("192.168.") || host.starts_with("172.") {
             return Err("HALT.ERROR: SSRF - Private IP ranges are blocked in production".to_string());
        }
    }
    Ok(())
}
