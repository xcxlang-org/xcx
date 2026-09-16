//! Performance/resource regression guards.
//!
//! Machine-independence rules (these make the guards valid on any hardware):
//! - **No absolute wall-clock thresholds.** Programs self-time with the
//!   documented monotonic `perf.ns()` and print their own best-of-N time;
//!   assertions compare *ratios* between two modes of the same binary, so
//!   the hardware cancels out.
//! - The RAM guard asserts a *growth ratio* (peak after 5 parses vs peak
//!   after 1 parse) — that measures allocator behaviour, not machine speed.
//! - Factors are generous (≥5× where the architectural gap is ~25×) so
//!   scheduler/thermal noise can never flip a result; if one of these ever
//!   fails, it is a real regression, not noise.

use crate::common;

use std::time::{Duration, Instant};

const FIB_SELF_TIMED: &str = r#"func fib(i: n -> i) {
    if (n < 2) then;
        return n;
    end;
    return fib(n - 1) + fib(n - 2);
};
i: best = 9223372036854775000;
i: r = 0;
i: k = 0;
while (k < 7) do;
    i: t0 = perf.ns();
    r = fib(25);
    i: dt = perf.ns() - t0;
    if (dt < best) then;
        best = dt;
    end;
    k = k + 1;
end;
>! r;
>! best;
"#;

/// Parses `>! r; >! best;` output: (result, best_ns).
fn parse_result_and_best(stdout: &str) -> (i64, i64) {
    let nums: Vec<i64> = common::normalize(stdout)
        .lines()
        .filter_map(|l| l.trim().parse::<i64>().ok())
        .collect();
    assert!(
        nums.len() >= 2,
        "expected '<result>\\n<best ns>' on stdout, got: {stdout:?}"
    );
    (nums[0], nums[1])
}

#[test]
fn fib_jit_is_substantially_faster_than_interpreter() {
    // Architectural gap (4.4): JIT ~26x faster than --no-jit on fib(25).
    // The guard demands only 5x, so ordinary hardware variation can never
    // fail it — but a fallback-to-interpreter or codegen regression can.
    let jit = common::run_jit("perf_fib_jit", FIB_SELF_TIMED);
    let nojit = common::run_nojit("perf_fib_nojit", FIB_SELF_TIMED);

    assert!(
        jit.ok() && nojit.ok(),
        "fib guard programs must succeed\n--- jit (exit {:?}) ---\n{}\n{}\n--- nojit (exit {:?}) ---\n{}\n{}",
        jit.code,
        jit.stdout,
        jit.stderr,
        nojit.code,
        nojit.stdout,
        nojit.stderr
    );
    let (jit_result, jit_best_ns) = parse_result_and_best(&jit.stdout);
    let (nojit_result, nojit_best_ns) = parse_result_and_best(&nojit.stdout);

    assert_eq!(jit_result, 75025, "fib(25) result diverged in JIT mode");
    assert_eq!(
        nojit_result, 75025,
        "fib(25) result diverged in interpreter mode"
    );
    assert!(jit_best_ns > 0 && nojit_best_ns > 0, "degenerate timings");

    let ratio = nojit_best_ns as f64 / jit_best_ns as f64;
    assert!(
        ratio >= 5.0,
        "JIT fib speedup vs interpreter dropped to {ratio:.2}x (guard: >= 5x)"
    );
}

const PARSE_ONCE: &str = r#"
s: payload = "[";
i: i = 0;
while (i < 20000) do;
    payload = payload + "{\"id\":" + s(i) + ",\"name\":\"User_" + s(i) + "\",\"active\":true,\"tags\":[\"a\",\"b\"]}";
    if (i < 19999) then; payload = payload + ","; end;
    i = i + 1;
end;
payload = payload + "]";
json: data = json.parse(payload);
json: item = data.get(10000);
s: nm = "";
item.bind("name", nm);
>! "parsed: " + nm;
"#;

const PARSE_FIVE: &str = r#"
s: payload = "[";
i: i = 0;
while (i < 20000) do;
    payload = payload + "{\"id\":" + s(i) + ",\"name\":\"User_" + s(i) + "\",\"active\":true,\"tags\":[\"a\",\"b\"]}";
    if (i < 19999) then; payload = payload + ","; end;
    i = i + 1;
end;
payload = payload + "]";
i: n = 0;
while (n < 5) do;
    json: data = json.parse(payload);
    json: item = data.get(10000);
    s: nm = "";
    item.bind("name", nm);
    n = n + 1;
end;
>! "parsed: " + nm;
"#;

/// Peak committed private bytes (MB) of a child, sampled while it runs.
/// Returns the peak; the caller must `wait()` the child afterwards.
#[cfg(windows)]
fn peak_private_mb_sampled(child: &mut std::process::Child) -> u64 {
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    struct ProcessMemoryCountersEx {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }

    unsafe extern "system" {
        fn K32GetProcessMemoryInfo(
            process: *mut core::ffi::c_void,
            counters: *mut ProcessMemoryCountersEx,
            cb: u32,
        ) -> i32;
    }

    // The raw handle value is stable for the child's lifetime; taking it as
    // usize up front avoids holding a borrow across try_wait().
    let handle_usize = child.as_raw_handle() as usize;
    let mut peak_mb: u64 = 0;
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        match child.try_wait().expect("poll child") {
            Some(_status) => break,
            None => {
                let mut pmc = ProcessMemoryCountersEx {
                    cb: std::mem::size_of::<ProcessMemoryCountersEx>() as u32,
                    page_fault_count: 0,
                    peak_working_set_size: 0,
                    working_set_size: 0,
                    quota_peak_paged_pool_usage: 0,
                    quota_paged_pool_usage: 0,
                    quota_non_paged_pool_usage: 0,
                    pagefile_usage: 0,
                    peak_pagefile_usage: 0,
                    private_usage: 0,
                };
                let ok = unsafe {
                    K32GetProcessMemoryInfo(
                        handle_usize as *mut core::ffi::c_void,
                        &mut pmc,
                        pmc.cb,
                    )
                };
                if ok != 0 {
                    let mb = (pmc.private_usage / (1024 * 1024)) as u64;
                    if mb > peak_mb {
                        peak_mb = mb;
                    }
                }
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("json RAM guard program timed out");
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    peak_mb
}

/// The 2026-09-06 RAM-leak regression: 5 parses of the same payload must
/// cost roughly the same peak memory as 1 parse (a fixed run reuses the
/// freed tree). The pre-fix build grew linearly (~48 MB per parse); the
/// ratio guard fails long before that returns.
#[cfg(windows)]
#[test]
fn repeated_json_parse_does_not_grow_memory_linearly() {
    let mut once_child = spawn_run("ram_parse_once", PARSE_ONCE);
    let peak_once = peak_private_mb_sampled(&mut once_child);
    let _ = once_child.wait();

    let mut five_child = spawn_run("ram_parse_five", PARSE_FIVE);
    let peak_five = peak_private_mb_sampled(&mut five_child);
    let _ = five_child.wait();

    assert!(
        peak_once < 400 && peak_five < 400,
        "absolute sanity backstop blown: once={peak_once}MB five={peak_five}MB"
    );

    let limit = (peak_once as f64 * 1.8) as u64 + 30;
    assert!(
        peak_five <= limit,
        "5 parses peaked at {peak_five} MB, more than 1.8x + 30 MB of the \
         1-parse peak ({peak_once} MB) — the json.parse linear-growth leak \
         is back"
    );
}

#[cfg(windows)]
fn spawn_run(name: &str, source: &str) -> std::process::Child {
    let file = common::write_source(name, source);
    std::process::Command::new(common::EXE)
        .arg(&file)
        .current_dir(file.parent().unwrap())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn json RAM guard program")
}
