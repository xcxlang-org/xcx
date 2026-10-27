//! Direct-call synchronization must preserve globals through call chains,
//! recursion and executor-mediated callbacks while skipping unrelated slots.
#[path = "common/mod.rs"]
mod common;

use std::collections::HashSet;
use std::sync::Arc;
use xcx::jit::analysis::analyze_global_slot_accesses;
use xcx::vm::opcode::{Chunk, MethodKind, OpCode};

fn chunk(code: Vec<OpCode>) -> Arc<Chunk> {
    Arc::new(Chunk::new(code, vec![], false, 4, "summary".into(), 0))
}

#[test]
fn access_summary_closes_recursive_call_graph() {
    let functions = vec![
        chunk(vec![OpCode::GetVar { dst: 0, idx: 7 },
            OpCode::Call { dst: 0, func_idx: 1, base: 0, arg_count: 0 },
            OpCode::Return { src: 0 }]),
        chunk(vec![OpCode::SetVar { idx: 9, src: 0 },
            OpCode::Call { dst: 0, func_idx: 0, base: 0, arg_count: 0 },
            OpCode::Return { src: 0 }]),
        chunk(vec![OpCode::ReturnVoid]),
    ];
    let result = analyze_global_slot_accesses(&functions);
    let expected = Some(HashSet::from([7, 9]));
    assert_eq!(result[0], expected);
    assert_eq!(result[1], expected);
    assert_eq!(result[2], Some(HashSet::new()));
}

#[test]
fn indirect_or_unknown_callee_requires_full_publication() {
    let functions = vec![
        chunk(vec![OpCode::Call { dst: 0, func_idx: 1, base: 0, arg_count: 0 },
            OpCode::ReturnVoid]),
        chunk(vec![OpCode::MethodCall { dst: 0, kind: MethodKind::Run, base: 0, arg_count: 0 },
            OpCode::ReturnVoid]),
        chunk(vec![OpCode::Call { dst: 0, func_idx: 99, base: 0, arg_count: 0 },
            OpCode::ReturnVoid]),
    ];
    assert_eq!(analyze_global_slot_accesses(&functions), vec![None, None, None]);
}

fn assert_modes(name: &str, source: &str, expected: &str) {
    for (suffix, flags) in [
        ("jit", vec!["--no-inline"]),
        ("int", vec!["--no-inline", "--no-jit"]),
    ] {
        let result = common::run_source(&format!("{name}_{suffix}"), source, &flags);
        assert!(result.ok(), "{}\n{}", result.stdout, result.stderr);
        assert_eq!(common::normalize(&result.stdout).trim(), expected);
    }
}

#[test]
fn direct_calls_publish_transitive_reads_and_preserve_writes() {
    assert_modes("global_call_chain", r#"
i: incoming = 0;
i: changed = 0;
i: total = 0;
func read_value(-> i) { return incoming; }
func relay(-> i) { return read_value(); }
func increment(-> i) { changed = changed + 1; return changed; }
for n in 1 to 32 do
    incoming = n;
    total = total + relay() + increment();
end
>! total;
>! incoming;
>! changed;
"#, "1056\n32\n32");
}

#[test]
fn recursive_calls_publish_globals_before_executor_fallback() {
    assert_modes("global_recursive_calls", r#"
i: shared = 0;
func even_step(i: remaining -> i) {
    if (remaining == 0) then return shared; end
    shared = shared + 1;
    return odd_step(remaining - 1);
}
func odd_step(i: remaining -> i) {
    if (remaining == 0) then return shared; end
    shared = shared + 1;
    return even_step(remaining - 1);
}
shared = 40;
>! even_step(8);
>! shared;
"#, "48\n48");
}

#[test]
fn fiber_callback_observes_pending_caller_globals() {
    assert_modes("global_fiber_callback", r#"
i: shared = 0;
fiber advance() { shared = shared + 1; yield; }
func apply(-> i) {
    fiber: work = advance();
    work.run();
    return shared;
}
shared = 40;
>! apply();
"#, "41");
}

#[test]
fn cold_array_bounds_paths_keep_error_diagnostics() {
    for (name, source) in [
        ("get", "array:b: flags {true}; >! flags.get(2);"),
        ("update", "array:b: flags {true}; flags.update(2, false);"),
    ] {
        for (mode, flags) in [("jit", vec![]), ("int", vec!["--no-jit"])] {
            let result = common::run_source(&format!("cold_bounds_{name}_{mode}"), source, &flags);
            assert!(!result.ok(), "out-of-bounds {name} unexpectedly succeeded");
            assert!(result.stderr.contains("R303"), "{}", result.stderr);
        }
    }
}

#[test]
fn conditional_integer_updates_match_interpreter() {
    assert_modes("conditional_integer_updates", r#"
i: global_count = 100;
func count(i: rounds -> i) {
    i: local_count = 200;
    for n in 1 to rounds do
        if (n % 3 == 0) then local_count = local_count + 1; end
        if (n % 5 == 0) then local_count = local_count - 1; end
    end
    return local_count;
}
for n in 1 to 1000 do
    if (n % 3 == 0) then global_count = global_count + 1; end
    if (n % 5 == 0) then global_count = global_count - 1; end
end
>! global_count;
>! count(1000);
"#, "233\n333");
}
