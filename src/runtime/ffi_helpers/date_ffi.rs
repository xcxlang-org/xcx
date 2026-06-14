use chrono::{Datelike, Timelike};

// Kind mapping:
// 1 => Year
// 2 => Month
// 3 => Day
// 4 => Hour
// 5 => Minute
// 6 => Second
#[unsafe(no_mangle)]
pub unsafe extern "C" fn xcx_jit_date_field(ts: u64, _tag: u64, kind_raw: i64) -> i64 {
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or_default().with_timezone(&chrono::Local);
    match kind_raw {
        1 => dt.year() as i64,
        2 => dt.month() as i64,
        3 => dt.day() as i64,
        4 => dt.hour() as i64,
        5 => dt.minute() as i64,
        6 => dt.second() as i64,
        7 => dt.timestamp_subsec_millis() as i64,
        _ => 0,
    }
}
