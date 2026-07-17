use std::sync::Arc;
use std::sync::atomic::Ordering;
use crate::vm::value::Value;
use crate::vm::opcode::OpCode;
use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::object::StringObj;

pub fn handle<'a>(
    exec: &mut Executor,
    op: OpCode,
    locals: &mut [Value],
    vm_arc: &'a Arc<VM>,
) -> Option<OpResult> {
    match op {
        // Store Operations
        OpCode::StoreRead { dst, base } => Some(crate::runtime::builtin::store::read(dst, base, locals)),
        OpCode::StoreWrite { dst, base } => Some(crate::runtime::builtin::store::write(dst, base, locals)),
        OpCode::StoreAppend { dst, base } => Some(crate::runtime::builtin::store::append(dst, base, locals)),
        OpCode::StoreExists { dst, base } => Some(crate::runtime::builtin::store::exists(dst, base, locals)),
        OpCode::StoreDelete { dst, base } => Some(crate::runtime::builtin::store::delete(dst, base, locals)),
        OpCode::StoreList { dst, base } => Some(crate::runtime::builtin::store::list(dst, base, locals)),
        OpCode::StoreIsDir { dst, base } => Some(crate::runtime::builtin::store::is_dir(dst, base, locals)),
        OpCode::StoreSize { dst, base } => Some(crate::runtime::builtin::store::size(dst, base, locals)),
        OpCode::StoreMkdir { dst, base } => Some(crate::runtime::builtin::store::mkdir(dst, base, locals)),
        OpCode::StoreGlob { dst, base } => Some(crate::runtime::builtin::store::glob(dst, base, locals)),
        OpCode::StoreZip { dst, base } => Some(crate::runtime::builtin::store::zip(dst, base, locals)),
        OpCode::StoreUnzip { dst, base } => Some(crate::runtime::builtin::store::unzip(dst, base, locals)),

        // Database
        OpCode::DatabaseInit { dst, engine_src, path_src, tables_base_reg, table_count } => {
            let engine = locals[engine_src as usize].to_string();
            let path = locals[path_src as usize].to_string();
            let mut tables = Vec::new();
            for i in 0..table_count {
                tables.push(locals[(tables_base_reg + i as u8) as usize].to_string());
            }
            Some(exec.handle_database_init(dst, engine, path, &tables, 0, locals)) // FIXME: ip = 0? Need to pass ip if needed for error report
        }

        // Env
        OpCode::EnvGet { dst, src } => {
            let key = locals[src as usize].to_string();
            let val = std::env::var(key).unwrap_or_default();
            let res = Value::from_string(Arc::new(StringObj::new(val.into_bytes())));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            Some(OpResult::Continue)
        }
        OpCode::EnvArgs { dst } => {
            let args: Vec<String> = std::env::args().collect();
            let res = Value::from_string_array(Arc::new(args));
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            Some(OpResult::Continue)
        }

        // JSON access
        OpCode::JsonBindLocal { dst, json_src, path_src } => {
            let json = locals[json_src as usize];
            let path_val = locals[path_src as usize];
            let path_borrow = unsafe { path_val.as_str_borrow() };
            let path_temp;
            let path = match path_borrow {
                Some(s) => s,
                None => { path_temp = path_val.to_string(); &path_temp }
            };
            let is_simple = !path.is_empty()
                && path.bytes().all(|b| b != b'.' && b != b'[' && b != b']' && b != b'/');
            let result = if is_simple && json.is_json() {
                let json_rc = json.as_json();
                match &json_rc.root {
                    crate::vm::object::JsonVal::Object(o) => {
                        let o_read = unsafe { &*(*o).data_ptr() };
                        o_read.iter()
                            .find(|(k, _)| k.as_str() == path)
                            .map(|(_, v)| crate::vm::utils::json::json_val_to_value(v))
                    }
                    _ => crate::runtime::builtin::json::access::get_path_value_xcx(json, path),
                }
            } else {
                crate::runtime::builtin::json::access::get_path_value_xcx(json, path)
            };
            if let Some(val) = result {
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = val;
                Some(OpResult::Continue)
            } else {
                eprintln!("R404: JSON path '{}' not found for .bind()", path);
                vm_arc.error_count.fetch_add(1, Ordering::SeqCst);
                Some(OpResult::Halt)
            }
        }
        OpCode::JsonBind { idx, json_src, path_src } => {
            let json = locals[json_src as usize];
            let path = locals[path_src as usize].to_string();
            if let Some(val) = crate::runtime::builtin::json::access::get_path_value_xcx(json, &path) {
                vm_arc.set_global(idx as usize, val);
                Some(OpResult::Continue)
            } else {
                eprintln!("R404: JSON path '{}' not found for .bind()", path);
                vm_arc.error_count.fetch_add(1, Ordering::SeqCst);
                Some(OpResult::Halt)
            }
        }
        OpCode::JsonInjectLocal { table_reg, json_src, mapping_src } => {
            let table = locals[table_reg as usize];
            let json = locals[json_src as usize];
            let mapping = locals[mapping_src as usize];
            unsafe { table.inc_ref(); json.inc_ref(); mapping.inc_ref(); }
            Some(crate::runtime::builtin::json::inject::json_inject_table_impl(exec, table, json, mapping))
        }
        OpCode::JsonInject { table_idx, json_src, mapping_src } => {
            let table = vm_arc.get_global(table_idx as usize);
            let json = locals[json_src as usize];
            let mapping = locals[mapping_src as usize];
            unsafe { table.inc_ref(); json.inc_ref(); mapping.inc_ref(); }
            Some(crate::runtime::builtin::json::inject::json_inject_table_impl(exec, table, json, mapping))
        }
        OpCode::JsonFastGetPush { json_src, path_src, val_src } => {
            let json = locals[json_src as usize];
            let val = locals[val_src as usize];
            if !json.is_json() {
                eprintln!("R306: Invalid root object for push");
                vm_arc.error_count.fetch_add(1, Ordering::SeqCst);
                return Some(OpResult::Halt);
            }
            let path_borrow = unsafe { locals[path_src as usize].as_str_borrow() };
            let path_temp;
            let path_str = match path_borrow {
                Some(s) => s,
                None => { path_temp = locals[path_src as usize].to_string(); &path_temp }
            };
            
            let json_ptr = {
                if json.tag == crate::vm::value::nan_boxing::TAG_ARENA {
                     crate::vm::value::heap_object::arena_ptr::<crate::vm::object::JsonObj>(&json) as *mut crate::vm::object::JsonObj
                } else {
                     json.unpack_ptr::<crate::vm::object::JsonObj>() as *mut crate::vm::object::JsonObj
                }
            };
            
            let is_simple = path_str.bytes().all(|b| b != b'/' && b != b'.' && b != b'[' && b != b']');
            unsafe {
                (*json_ptr).version.fetch_add(1, std::sync::atomic::Ordering::Release);
                if is_simple {
                    if let crate::vm::object::JsonVal::Object(o) = &(*json_ptr).root {
                        let o_read = o.read();
                        if let Some((_, v)) = o_read.iter().find(|(k, _)| k.as_str() == path_str) {
                            if let crate::vm::object::JsonVal::Array(arr) = v {
                                arr.write().push(crate::vm::utils::json::value_to_json(&val));
                                return Some(OpResult::Continue);
                            }
                        }
                    }
                } else {
                    let pointer = crate::runtime::builtin::json::access::normalize_json_path(path_str);
                    if let Some(v) = (*json_ptr).root.pointer(&pointer) {
                        if let crate::vm::object::JsonVal::Array(arr) = v {
                            arr.write().push(crate::vm::utils::json::value_to_json(&val));
                            return Some(OpResult::Continue);
                        }
                    }
                }
            }
            eprintln!("halt.error: push target is not an array");
            vm_arc.error_count.fetch_add(1, Ordering::SeqCst);
            Some(OpResult::Halt)
        }

        // Terminal
        OpCode::TerminalClear { dst } => {
            let res = crate::runtime::builtin::io::clear();
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(true);
            Some(res)
        }
        OpCode::TerminalRaw { dst } => {
            let res = crate::runtime::builtin::io::raw_mode(exec);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(true);
            Some(res)
        }
        OpCode::TerminalNormal { dst } => {
            let res = crate::runtime::builtin::io::normal_mode(exec);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(true);
            Some(res)
        }
        OpCode::TerminalCursor { dst, on } => {
            let res = crate::runtime::builtin::io::cursor(on);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(true);
            Some(res)
        }
        OpCode::TerminalMove { dst, x_src, y_src } => {
            let res = crate::runtime::builtin::io::move_cursor(x_src, y_src, locals);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(true);
            Some(res)
        }
        OpCode::TerminalExit { dst } => {
            let res = crate::runtime::builtin::io::exit();
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(true);
            Some(res)
        }
        OpCode::TerminalRun { dst, cmd_src } => {
            let cmd = locals[cmd_src as usize].to_string();
            let output = if cfg!(windows) {
                std::process::Command::new("cmd").arg("/C").arg(cmd).output()
            } else {
                std::process::Command::new("sh").arg("-c").arg(cmd).output()
            };
            let res = match output {
                Ok(o) => Value::from_string(Arc::new(StringObj::new(o.stdout))),
                Err(_) => Value::from_bool(false),
            };
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            Some(OpResult::Continue)
        }
        OpCode::TerminalWrite { dst, src } => {
            let _s = locals[src as usize].to_string();
            crate::runtime::builtin::io::write_buffered(&_s);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = Value::from_bool(true);
            Some(OpResult::Continue)
        }

        // IO
        OpCode::Print { src } => {
            let _s = locals[src as usize].to_string();
            #[cfg(unix)]
            crate::runtime::builtin::io::write_buffered(&(_s + "\x1b[K\r\n"));
            #[cfg(not(unix))]
            crate::runtime::builtin::io::write_buffered(&(_s + "\x1b[K\n"));
            Some(OpResult::Continue)
        }
        OpCode::Input { dst, ty } => Some(crate::runtime::builtin::io::input(dst, ty, locals, exec, vm_arc)),
        OpCode::InputKey { dst } => Some(crate::runtime::builtin::io::input::read_key(dst, locals, exec)),
        OpCode::InputKeyWait { dst } => Some(crate::runtime::builtin::io::input::wait_key(dst, locals, exec, vm_arc)),
        OpCode::InputReady { dst } => Some(crate::runtime::builtin::io::input::is_ready(dst, locals)),

        // Crypto
        OpCode::CryptoHash { dst, pass_src, alg_src } => Some(crate::runtime::builtin::crypto::hash(dst, pass_src, alg_src, locals)),
        OpCode::CryptoVerify { dst, pass_src, hash_src, alg_src } => Some(crate::runtime::builtin::crypto::verify(dst, pass_src, hash_src, alg_src, locals)),
        OpCode::CryptoToken { dst, len_src } => Some(crate::runtime::builtin::crypto::token(dst, len_src, locals)),

        // Net
        OpCode::HttpCall { dst, method_idx, url_src, body_src } => {
            let constants = exec.ctx.constants.clone();
            Some(crate::runtime::builtin::net::call(dst, method_idx, url_src, body_src, locals, &constants))
        }
        OpCode::HttpServe { func_idx, port_src, host_src, workers_src, routes_src } => {
            Some(crate::runtime::builtin::net::serve(func_idx, port_src, host_src, workers_src, routes_src, locals, &exec.ctx, vm_arc))
        }
        OpCode::HttpRespond { dst, status_src, body_src, headers_src } => {
            Some(crate::runtime::builtin::net::respond(dst, status_src, body_src, headers_src, locals, exec.ctx.http_req.clone()))
        }
        OpCode::HttpRequest { dst, arg_src } => Some(crate::runtime::builtin::net::request(dst, arg_src, locals, None)),

        OpCode::JsonParse { dst, src } => {
            let val = locals[src as usize];
            let res = crate::runtime::builtin::json::parse::json_parse_impl(val);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            Some(OpResult::Continue)
        }
        OpCode::DateNow { dst } => {
            let now = chrono::Utc::now().timestamp_millis();
            let res = Value::from_date(now);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            Some(OpResult::Continue)
        }
        OpCode::PerfMs { dst } => {
            let elapsed = vm_arc.start_instant.elapsed().as_millis() as i64;
            let res = Value::from_i64(elapsed);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            Some(OpResult::Continue)
        }
        OpCode::PerfUs { dst } => {
            let elapsed = vm_arc.start_instant.elapsed().as_micros() as i64;
            let res = Value::from_i64(elapsed);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            Some(OpResult::Continue)
        }
        OpCode::PerfNs { dst } => {
            let elapsed = vm_arc.start_instant.elapsed().as_nanos() as i64;
            let res = Value::from_i64(elapsed);
            unsafe { locals[dst as usize].dec_ref(); }
            locals[dst as usize] = res;
            Some(OpResult::Continue)
        }

        OpCode::HaltAlert { src } => {
            let _msg = locals[src as usize].to_string();
            // eprintln!("XCX Alert: {}", _msg);
            Some(OpResult::Continue)
        }
        OpCode::HaltError { src } => {
            let msg = locals[src as usize].to_string();
            eprintln!("XCX Error: {}", msg);
            Some(OpResult::Halt)
        }
        OpCode::HaltFatal { src } => {
            let _msg = locals[src as usize].to_string();
            eprintln!("XCX Fatal: {}", _msg);
            std::process::exit(1);
        }

        _ => None,
    }
}
