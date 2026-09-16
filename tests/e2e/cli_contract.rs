//! CLI contract regressions: exit codes, halt semantics and the semantic
//! error registry (errors_halt.md). These run the real binary — in-process
//! harnesses cannot observe process exit codes.

use crate::common;

/// errors_halt.md: `halt.error` prints an error to stderr and aborts the
/// current stack frame; execution continues after the call site returns.
///
/// Documented scope only: the process exit code for a program that used
/// `halt.error` and recovered is not defined in errors_halt.md, so it is
/// deliberately not asserted here (it is 0 today — the error-count/exit-code
/// mechanism is a separate, unconfirmed contract).
#[test]
fn halt_error_aborts_frame_and_reports() {
    let src = r#"
func risky() {
    halt.error >! "boom";
    >! "unreachable";
};
>! "before";
risky();
>! "after";
"#;
    let jit = common::run_jit("cli_halt_error_jit", src);
    let nojit = common::run_nojit("cli_halt_error_nojit", src);

    assert!(jit.ok(), "halt.error must not terminate the process");
    assert!(nojit.ok(), "halt.error must not terminate the process");
    assert!(
        common::normalize(&jit.stderr).contains("XCX Error: boom"),
        "expected the halt.error diagnostic on stderr (JIT), got: {}",
        jit.stderr
    );
    assert!(
        common::normalize(&nojit.stderr).contains("XCX Error: boom"),
        "expected the halt.error diagnostic on stderr (interpreter), got: {}",
        nojit.stderr
    );
    for run in [&jit, &nojit] {
        assert!(
            run.stdout.contains("before"),
            "statements before the failing frame must have run"
        );
        assert!(
            !run.stdout.contains("unreachable"),
            "halt.error must abort the current frame"
        );
        assert!(
            run.stdout.contains("after"),
            "execution must continue after the aborted frame returns"
        );
    }
}

/// errors_halt.md: `halt.fatal` immediately terminates the VM with an error
/// code — nothing after it may execute.
#[test]
fn halt_fatal_terminates_immediately() {
    let src = r#"
>! "before";
halt.fatal >! "critical failure";
>! "after";
"#;
    let run = common::run_nojit("cli_halt_fatal", src);

    assert_eq!(run.code, Some(1), "halt.fatal must set exit code 1");
    assert!(
        run.stderr.contains("XCX Fatal: critical failure"),
        "expected the halt.fatal diagnostic on stderr, got: {}",
        run.stderr
    );
    assert!(run.stdout.contains("before"));
    assert!(
        !run.stdout.contains("after"),
        "halt.fatal must terminate immediately"
    );
}

/// errors_halt.md: S106 — `break` outside a loop blocks compilation. The
/// error reporter prints the diagnostic to **stdout** (with ANSI coloring),
/// and the program must never execute.
#[test]
fn break_outside_loop_is_semantic_error_s106() {
    let run = common::run_nojit("cli_s106_break", ">! \"start\";\nbreak;");
    assert!(!run.ok(), "S106 must block compilation (exit code must be nonzero)");
    assert!(
        common::normalize(&run.stdout).contains("S106"),
        "expected the S106 diagnostic, got stdout: {} stderr: {}",
        run.stdout,
        run.stderr
    );
    assert!(
        !common::normalize(&run.stdout).contains("start"),
        "a program that fails compilation must not run"
    );
}

/// errors_halt.md: R305 — malformed JSON in `json.parse()` raises
/// `halt.fatal` (immediate exit).
#[test]
fn invalid_json_is_fatal_r305() {
    let src = r#"
>! "before";
s: bad = "definitely not json";
json: p = json.parse(bad);
>! "after";
"#;
    let run = common::run_nojit("cli_r305_json", src);
    assert_eq!(run.code, Some(1), "R305 must be halt.fatal (exit code 1)");
    assert!(
        run.stderr.contains("XCX Fatal:"),
        "expected the fatal diagnostic on stderr, got: {}",
        run.stderr
    );
    assert!(
        run.stderr.contains("R305"),
        "expected the R305 code on stderr, got: {}",
        run.stderr
    );
    assert!(!run.stdout.contains("after"));
}
