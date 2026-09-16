//! Documentation-consistency suite (v1, curated).
//!
//! Source of truth: `documentation/language/*.md`. Every test below
//! exercises behaviour that a language document explicitly describes; if a
//! document gains a claim without an implementation (the 2026-09-07
//! `.len()` class of bug), a test here is where it surfaces. Each test
//! names the document file it guards in its header comment.

mod common;

/// types.md + collections.md — Array method table.
#[test]
fn docs_collections_arrays() {
    let out = common::expect_success(
        "docs_array_methods",
        r#"
array:i: nums {5, 2, 8, 1};
>! nums.size();
>! nums.count();
>! nums.len();
>! nums.get(1);
nums.push(40);
>! nums.size();
i: popped = nums.pop();
>! popped;
nums.sort();
>! nums.get(0);
nums.reverse();
>! nums.get(0);
nums.insert(1, 15);
>! nums.get(1);
nums.update(0, 7);
>! nums.get(0);
nums.delete(1);
>! nums.size();
>! nums.find(999);
>! nums.contains(8);
>! nums.isEmpty();
array:i: sl = nums.slice(0, 2);
>! sl.size();
"#,
    );
    // sort → {1,2,5,8}; reverse → {8,5,2,1}; insert(1,15) →
    // {8,15,2,1}; update(0,7) → {7,15,2,1}; delete(1) → {7,2,1}
    assert_eq!(
        out,
        "4\n4\n4\n2\n5\n40\n1\n8\n15\n7\n4\n-1\nfalse\nfalse\n2"
    );
}

/// collections.md — Set table: aliases, membership, add-dedup, remove,
/// isEmpty, inclusive ranges, the four operations.
#[test]
fn docs_collections_sets() {
    let out = common::expect_success(
        "docs_set_methods",
        r#"
set:N: s {1,,5};
>! s.size();
>! s.count();
>! s.len();
>! s.contains(3);
s.add(9);
s.add(9);
>! s.size();
s.remove(1);
>! s.size();
>! s.isEmpty();
set:N: a {1,,4};
set:N: b {3,,6};
set:N: u = a UNION b;
set:N: i2 = a INTERSECTION b;
set:N: d = a DIFFERENCE b;
set:N: sd = a SYMMETRIC_DIFFERENCE b;
>! u.size();
>! i2.size();
>! d.size();
>! sd.size();
"#,
    );
    // s = {1,2,3,4,5,9}; remove(1) → {2,3,4,5,9}
    // union {1..6}=6; inter {3,4}=2; diff {1,2}=2; symdiff {1,2,5,6}=4
    assert_eq!(out, "5\n5\n5\ntrue\n6\n5\nfalse\n6\n2\n2\n4");
}

/// collections.md — Map table: aliases, get, contains, insert-overwrite,
/// remove, keys/values, toJson, clear.
#[test]
fn docs_collections_maps() {
    let out = common::expect_success(
        "docs_map_methods",
        r#"
map: m {
    schema = [s <-> i]
    data = [ "alice" :: 30, "bob" :: 25 ]
};
>! m.size();
>! m.count();
>! m.len();
>! m.get("alice");
>! m.contains("bob");
m.insert("bob", 26);
>! m.get("bob");
>! m.remove("carol");
array:s: k = m.keys();
array:i: v = m.values();
>! k.size();
>! v.size();
json: j = m.toJson();
>! j.size();
>! m.isEmpty();
m.clear();
>! m.isEmpty();
>! m.size();
"#,
    );
    assert_eq!(
        out,
        "2\n2\n2\n30\ntrue\n26\nfalse\n2\n2\n2\nfalse\ntrue\n0"
    );
}

/// string_methods.md — the documented chaining example plus the full method
/// table and `.length` on unicode/escapes.
#[test]
fn docs_string_methods() {
    let out = common::expect_success(
        "docs_string_methods",
        r#"
s: raw = "  XCX-Language  ";
>! raw.upper();
>! raw.lower();
>! raw.trim();
>! raw.trim().lower().replace("-", "_");
>! "Programming".slice(0, 4);
>! "Programming".indexOf("gram");
>! "Programming".lastIndexOf("m");
>! "Programming".startsWith("Prog");
>! "Programming".endsWith("ing");
>! "42".toInt();
>! "2.5".toFloat();
array:s: parts = "a,b,c".split(",");
>! parts.size();
>! "zażółć".length;
>! "a\nb".length;
"#,
    );
    // upper/lower preserve the whitespace of `raw`; the leading spaces of
    // the FIRST output line are consumed by the harness's outer trim.
    assert_eq!(
        out,
        "XCX-LANGUAGE  \n  xcx-language  \nXCX-Language\nxcx_language\nProg\n3\n7\ntrue\ntrue\n42\n2.5\n3\n6\n3"
    );
}

/// json_http.md — JSON method table, path notation, `.push()` on an array
/// node, `.first()`, `json.parse()` round-trip.
#[test]
fn docs_json_methods() {
    let out = common::expect_success(
        "docs_json_methods",
        r#"
json: cfg <<< {"port": 8080, "debug": false} >>>;
cfg.set("port", 9090);
i: port; cfg.bind("port", port);
>! port;
>! cfg.exists("debug");
>! cfg.exists("nope");
>! cfg.size();
json: nested <<< {"server": {"host": "localhost"}} >>>;
s: host; nested.bind("server.host", host);
>! host;
json: data <<< {"items": []} >>>;
json: obj <<< {"id": 1} >>>;
data.get("items").push(obj);
>! data.get("items").size();
json: q <<< {"q": [10, 20, 30]} >>>;
json: first = q.get("q").first();
>! first;
array:s: keys = cfg.keys();
>! keys.size();
s: raw = "{\"v\": 7}";
json: parsed = json.parse(raw);
i: v; parsed.bind("v", v);
>! v;
"#,
    );
    assert_eq!(
        out,
        "9090\ntrue\nfalse\n2\nlocalhost\n1\n10\n2\n7"
    );
}

/// types.md — defaults, aliases and the casting table.
#[test]
fn docs_types_defaults_and_casts() {
    let out = common::expect_success(
        "docs_types",
        r#"
int: di;
float: df;
str: ds;
bool: db;
>! di;
>! df;
>! ds;
>! db;
f: x = 3.7;
>! i(x);
i: n = 42;
>! f(n);
i: num = 99;
>! s(num);
"#,
    );
    assert_eq!(out, "0\n0\n\nfalse\n3\n42\n99");
}

/// library_modules.md — perf (monotonic), env.args, random bounds,
/// crypto token/hash-verify, store round-trip.
#[test]
fn docs_library_modules() {
    let out = common::expect_success(
        "docs_library",
        r#"
i: t1 = perf.ns();
i: t2 = perf.ns();
>! t2 >= t1;
array:s: a = env.args();
i: n = 0;
i: in_range = 1;
while (n < 20) do;
    i: pick = random.int(1, 10);
    if (pick < 1) then; in_range = 0; end;
    if (pick > 10) then; in_range = 0; end;
    n = n + 1;
end;
>! in_range;
s: token = crypto.token(16);
>! token.length;
s: hash = crypto.hash("secret", "argon2");
>! crypto.verify("secret", hash, "argon2");
store.write("docs_probe.txt", "one");
store.append("docs_probe.txt", "two");
>! store.exists("docs_probe.txt");
>! store.size("docs_probe.txt") > 0;
s: content = store.read("docs_probe.txt");
>! content;
store.delete("docs_probe.txt");
>! store.exists("docs_probe.txt");
"#,
    );
    assert_eq!(
        out,
        "true\n1\n32\ntrue\ntrue\ntrue\nonetwo\nfalse"
    );
}

/// functions_fibers.md — 800-frame recursion limit raises halt.error.
/// errors_halt.md — the 800-frame recursion limit raises a recursion-limit
/// diagnostic. The diagnostic fires in both modes; the resulting process
/// exit code is mode-dependent (JIT: 1, interpreter: 0 — the halted frame
/// does not increment the error counter in the interpreter path), so the
/// exit code is deliberately not asserted here.
#[test]
fn docs_recursion_limit_is_enforced() {
    let run = common::run_nojit(
        "docs_recursion_limit",
        "func spin(i: n -> i) {\n    return spin(n + 1);\n};\n>! spin(0);",
    );
    assert!(
        common::normalize(&run.stderr).contains("Recursion limit exceeded (800 frames)"),
        "expected the recursion-limit diagnostic, got: {}",
        run.stderr
    );
}
