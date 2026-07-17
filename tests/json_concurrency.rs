use std::sync::Arc;
use std::thread;
use xcx::vm::object::{JsonObj, JsonVal};

#[test]
fn test_json_concurrency_race() {
    let inner_map = vec![(Arc::new("count".to_string()), JsonVal::Int(0))];
    let root = JsonVal::Object(Arc::new(parking_lot::RwLock::new(inner_map)));
    let json_obj = Arc::new(JsonObj::new(root));

    let num_threads = 4;
    let iterations = 2000;
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let json_clone = json_obj.clone();
        let handle = thread::spawn(move || {
            let mut last_read_val = -1;
            for i in 0..iterations {
                if i % 2 == 0 {
                    json_clone.version.fetch_add(1, std::sync::atomic::Ordering::Release);
                    if let JsonVal::Object(ref o) = json_clone.root {
                        let mut obj = o.write();
                        if let JsonVal::Int(current) = obj[0].1 {
                            obj[0].1 = JsonVal::Int(current + 1);
                        }
                    }
                } else {
                    let mut buf = String::new();
                    let ver = json_clone.version.load(std::sync::atomic::Ordering::Acquire);
                    let cached_ver = json_clone.cached_version.load(std::sync::atomic::Ordering::Acquire);
                    if ver == cached_ver {
                        if let Some(s) = json_clone.cached_str.lock().as_ref() {
                            buf = String::from_utf8_lossy(&s.data).into_owned();
                        }
                    }
                    if buf.is_empty() {
                        let mut serialize_buf = String::with_capacity(256);
                        json_clone.root.to_string_buf(&mut serialize_buf);
                        let string_obj = Arc::new(xcx::vm::object::StringObj::new(serialize_buf.into_bytes()));
                        
                        let mut lock = json_clone.cached_str.lock();
                        if json_clone.version.load(std::sync::atomic::Ordering::Acquire) == ver {
                            *lock = Some(string_obj.clone());
                            json_clone.cached_version.store(ver, std::sync::atomic::Ordering::Release);
                        }
                        buf = String::from_utf8_lossy(&string_obj.data).into_owned();
                    }

                    assert!(buf.starts_with("{\"count\":") && buf.ends_with('}'), "Malformed JSON string detected: {}", buf);
                    let parsed: i64 = buf["{\"count\":".len()..buf.len() - 1].parse().unwrap();
                    assert!(parsed >= last_read_val, "Stale cache read detected: got {}, previously saw {}", parsed, last_read_val);
                    last_read_val = parsed;
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
