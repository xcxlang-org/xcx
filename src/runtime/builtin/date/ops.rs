use std::sync::Arc;
use crate::vm::object::StringObj;

use crate::vm::core::vm::{VM, OpResult};
use crate::vm::core::executor::Executor;
use crate::vm::opcode::MethodKind;
use crate::vm::value::Value;

impl Executor {
    pub fn handle_date_method(
        &mut self,
        dst: u8,
        ts: i64,
        kind: MethodKind,
        _args: &[Value],
        _names: Option<&[String]>,
        ip: usize,
        locals: &mut [Value],
        _vm_arc: &Arc<VM>,
    ) -> OpResult {
        use chrono::{Datelike, Timelike};
        let dt = chrono::DateTime::from_timestamp_millis(ts).unwrap().with_timezone(&chrono::Local);
        match kind {
            MethodKind::Year => {
                let res = Value::from_i64(dt.year() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Month => {
                let res = Value::from_i64(dt.month() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Day => {
                let res = Value::from_i64(dt.day() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Hour => {
                let res = Value::from_i64(dt.hour() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Minute => {
                let res = Value::from_i64(dt.minute() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Second => {
                let res = Value::from_i64(dt.second() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Ms => {
                let res = Value::from_i64(dt.timestamp_subsec_millis() as i64);
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::Format => {
                let tokens = [
                    ("YYYY", "%Y"),
                    ("MM", "%m"),
                    ("DD", "%d"),
                    ("HH", "%H"),
                    ("mm", "%M"),
                    ("ss", "%S"),
                    ("SSS", "%3f"),
                    ("ms", "%3f"),
                    ("M", "%-m"),
                    ("D", "%-d"),
                ];

                let result = if !_args.is_empty() && _args[0].is_string() {
                    let fmt_str = _args[0].as_string_lossy();
                    let mut res = String::new();
                    let mut remaining = fmt_str.as_str();
                    while !remaining.is_empty() {
                        let mut matched = false;
                        for (xcx_tok, chrono_tok) in tokens.iter() {
                            if remaining.starts_with(xcx_tok) {
                                res.push_str(chrono_tok);
                                remaining = &remaining[xcx_tok.len()..];
                                matched = true;
                                break;
                            }
                        }
                        if !matched {
                            let ch = remaining.chars().next().unwrap();
                            if ch == '%' {
                                res.push_str("%%");
                            } else {
                                res.push(ch);
                            }
                            remaining = &remaining[ch.len_utf8()..];
                        }
                    }
                    res
                } else {
                    "%Y-%m-%d".to_string()
                };

                let formatted = dt.format(&result).to_string();
                let res = Value::from_string(Arc::new(StringObj::new(formatted.into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            MethodKind::ToStr => {
                let res = Value::from_string(Arc::new(StringObj::new(dt.format("%Y-%m-%d %H:%M:%S").to_string().into_bytes())));
                unsafe { locals[dst as usize].dec_ref(); }
                locals[dst as usize] = res;
            }
            _ => { 
                eprintln!("Method {:?} not supported for Date{}", kind, self.current_span_info(ip)); 
                self.vm.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return OpResult::Halt; 
            }
        }
        OpResult::Continue
    }
}
