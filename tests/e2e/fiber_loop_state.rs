use std::process::Command;

/// Regression (ISSUE-002): sequential `for` loops over two fibers, where the
/// second loop breaks out and the fiber's `.isDone()` is queried afterwards.
/// The loop variable used to alias the fiber's own register, so every `Next`
/// overwrote the fiber with the yielded value and `.isDone()` panicked with
/// "Method call on non-object receiver".
fn run_sequential_fiber_loops(tag: &str, engine_flag: Option<&str>) -> (String, bool) {
    let src = "fiber gen(i: max -> i) {\n\
               for i in 1 to max do;\n\
               yield i;\n\
               end;\n\
               return 0;\n\
               };\n\
               fiber limited(i: max -> i) {\n\
               for i in 1 to max do;\n\
               if (i == 3) then;\n\
               return 99;\n\
               end;\n\
               yield i;\n\
               end;\n\
               return 0;\n\
               };\n\
               fiber:i: lim = limited(10);\n\
               for v in lim do;\n\
               >! \"lim: \" + v;\n\
               end;\n\
               fiber:i: big = gen(100);\n\
               for v in big do;\n\
               >! \"big: \" + v;\n\
               if (v == 2) then; break; end;\n\
               end;\n\
               >! \"BIG_DONE=\" + big.isDone();\n";

    let dir = std::env::temp_dir().join("xcx_fiber_loop_state_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join(format!("fiber_loop_state_{}.xcx", tag));
    std::fs::write(&file, src).expect("write temp xcx source");

    let exe = env!("CARGO_BIN_EXE_xcx");
    let mut cmd = Command::new(exe);
    if let Some(flag) = engine_flag {
        cmd.arg(flag);
    }
    cmd.arg(&file);
    let out = cmd.output().expect("failed to run xcx binary");

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    (stdout, out.status.success())
}

#[test]
fn sequential_fiber_loops_keep_fiber_state() {
    let (stdout, success) = run_sequential_fiber_loops("jit", None);
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert!(
        stdout.contains("BIG_DONE=true"),
        "broken fiber after break: .isDone() must be true; stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("non-object receiver"),
        "fiber state corruption panicked the VM; stdout: {}",
        stdout
    );
}

#[test]
fn sequential_fiber_loops_keep_fiber_state_no_jit() {
    let (stdout, success) = run_sequential_fiber_loops("nojit", Some("--no-jit"));
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert!(
        stdout.contains("BIG_DONE=true"),
        "broken fiber after break: .isDone() must be true; stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains("non-object receiver"),
        "fiber state corruption panicked the VM; stdout: {}",
        stdout
    );
}
