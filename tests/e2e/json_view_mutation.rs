use std::process::Command;

/// Regression (ISSUE-013): JSON views and bind semantics.
/// (a) `.get(i).set(...)` on a fetched array element must be visible in the
///     parent (live views) — the old write-on-first-mutation COW disconnected
///     the fetched view from its parent.
/// (b) `.bind(path, target)` in expression position must write the target
///     variable AND yield `true` (documented `-> b` signature).
/// (c) Independent parses of the same text must stay independent — the parse
///     caches hand out owned deep copies, so mutating one parse never leaks
///     into another.
fn run_view_mutation(tag: &str, engine_flag: Option<&str>) -> (String, bool) {
    let src = "json: out <<< [] >>>;\n\
               out.push(<<< {} >>>);\n\
               out.get(0).set(\"text\", \"buy milk\");\n\
               >! \"SETVIEW=\" + out.toStr();\n\
               json: arr <<< [\"hello\"] >>>;\n\
               s: item;\n\
               b: ok = arr.bind(\"0\", item);\n\
               >! \"BINDOK=\" + ok + \";ITEM=\" + item;\n\
               s: txt = \"{\\\"n\\\": 1}\";\n\
               json: p1 = json.parse(txt);\n\
               json: p2 = json.parse(txt);\n\
               p1.set(\"n\", 99);\n\
               >! \"CACHEP2=\" + p2.toStr();\n";

    let dir = std::env::temp_dir().join("xcx_json_view_mutation_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join(format!("view_mutation_{}.xcx", tag));
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

fn assert_view_mutation_output(stdout: &str) {
    assert!(
        stdout.contains("SETVIEW=[{\"text\":\"buy milk\"}]"),
        "set through a fetched element was lost — parent not updated; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("BINDOK=true;ITEM=hello"),
        "bind in expression position must write the target and yield true; stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("CACHEP2={\"n\":1}"),
        "mutating one parse leaked into an independent parse of the same text; stdout: {}",
        stdout
    );
}

#[test]
fn fetched_json_element_set_is_visible_in_parent() {
    let (stdout, success) = run_view_mutation("jit", None);
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert_view_mutation_output(&stdout);
}

#[test]
fn fetched_json_element_set_is_visible_in_parent_no_jit() {
    let (stdout, success) = run_view_mutation("nojit", Some("--no-jit"));
    assert!(success, "xcx exited with failure; stdout: {}", stdout);
    assert_view_mutation_output(&stdout);
}
