//! JIT/interpreter parity pack.
//!
//! Every program below encodes semantics documented in
//! `documentation/language/*.md` and must produce **identical stdout** (and
//! exit status) in JIT mode and `--no-jit` mode. A divergence here is a
//! codegen bug by definition — do not "fix" a failing program to make the
//! test pass; the divergence itself is the finding.

use crate::common;

/// operators.md: precedence, truncating division, modulo, `^`, `++`.
const ARITH: &str = r#"
>! 10 + 3 * 2;
>! (10 + 3) * 2;
>! 10 / 3;
>! 10 % 3;
>! 2 ^ 10;
>! 48 ++ 77;
>! -7;
"#;

/// operators.md + string_methods.md: concatenation with auto-conversion,
/// `HAS`, method chaining, `.length` on unicode, escapes.
const STRINGS: &str = r#"
s: greeting = "Hi " + "User";
>! greeting;
>! "Value: " + 42;
>! "Search" HAS "ear";
s: clean = "  XCX-Language  ".trim().lower().replace("-", "_");
>! clean;
>! "Programming".indexOf("gram");
>! "Programming".slice(0, 4);
>! "25".toInt() + 1;
array:s: parts = "a,b,c".split(",");
>! parts.size();
>! "zażółć".length;
>! "line1\nline2".length;
"#;

/// control_flow.md: inclusive range for, `@step`, while, `continue`/`break`,
/// if/elseif/else with aliases.
const CONTROL: &str = r#"
for i in 1 to 3 do;
    >! i;
end;
for j in 0 to 6 @step 2 do;
    >! j;
end;
i: cnt = 0;
while (cnt < 3) do;
    cnt = cnt + 1;
    >! cnt;
end;
for n in 1 to 5 do;
    if (n % 2 == 0) then; continue; end;
    if (n == 5) then; break; end;
    >! n;
end;
i: score = 75;
if (score >= 90) then;
    >! "A";
elif (score >= 75) then;
    >! "B";
else;
    >! "F";
end;
"#;

/// collections.md: sort/reverse in-place, push/pop, insert/update/delete,
/// find/contains/isEmpty, slice.
const ARRAYS: &str = r#"
array:i: nums {5, 2, 8, 1};
nums.sort();
nums.reverse();
nums.push(99);
i: last = nums.pop();
>! last;
nums.insert(1, 15);
nums.update(0, 5);
nums.delete(3);
>! nums.get(0);
>! nums.get(1);
>! nums.size();
>! nums.contains(5);
>! nums.find(15);
>! nums.isEmpty();
array:i: sl = nums.slice(0, 2);
>! sl.size();
"#;

/// collections.md: set ranges, automatic deduplication, the four set
/// operations (sizes only — iteration order is not documented).
const SETS: &str = r#"
set:N: setA {1,,5};
set:N: setB {3,,7};
set:N: u = setA UNION setB;
set:N: i2 = setA INTERSECTION setB;
set:N: d = setA DIFFERENCE setB;
set:N: sd = setA SYMMETRIC_DIFFERENCE setB;
>! u.size();
>! i2.size();
>! d.size();
>! sd.size();
>! setA.contains(3);
>! setA.len();
set:N: evens {0,,10 @step 2};
>! evens.size();
set:S: words {"a", "b", "a"};
>! words.size();
"#;

/// collections.md: map get/insert/contains/remove/keys and `toJson()`
/// (sizes only — key order is not documented).
const MAPS: &str = r#"
map: ages {
    schema = [s <-> i]
    data = [ "alice" :: 30, "bob" :: 25 ]
};
>! ages.get("alice");
ages.insert("carol", 40);
>! ages.size();
>! ages.contains("bob");
>! ages.remove("bob");
>! ages.len();
>! ages.size();
json: j = ages.toJson();
>! j.size();
array:s: ks = ages.keys();
>! ks.size();
"#;

/// json_http.md: literal, `.set`, `.bind`, `.exists`, `.size()`/`.count()`/`.len()`,
/// path notation (dot + `[i]`), `.push()` on an array node, `.first()`,
/// `.keys()`, `json.parse()` round-trip.
const JSON: &str = r#"
json: cfg <<< {"port": 8080, "debug": false} >>>;
cfg.set("port", 9090);
i: port; cfg.bind("port", port);
>! port;
>! cfg.exists("debug");
>! cfg.exists("missing");
>! cfg.size();
json: nested <<< {"server": {"host": "localhost"}} >>>;
s: host; nested.bind("server.host", host);
>! host;
nested.set("items[0]", <<< {"id": 1} >>>);
nested.set("items[1]", <<< {"id": 2} >>>);
>! nested.get("items").size();
json: data <<< {"items": []} >>>;
json: obj <<< {"id": 1} >>>;
data.get("items").push(obj);
>! data.get("items").size();
json: arr <<< {"q": [10, 20, 30]} >>>;
json: f = arr.get("q").first();
>! f;
array:s: ks = cfg.keys();
>! ks.size();
s: raw = "{\"a\": 41, \"b\": \"x\"}";
json: p = json.parse(raw);
i: a; p.bind("a", a);
>! a;
json: sz <<< {"k1": 1, "k2": 2, "k3": 3} >>>;
>! sz.size();
>! sz.count();
>! sz.len();
"#;

/// functions_fibers.md: typed fibers, `yield from` delegation, `isDone()`,
/// iteration over a fiber.
const FIBERS: &str = r#"
fiber count(i: max -> i) {
    i: x = 0;
    while (x < max) do;
        yield x;
        x = x + 1;
    end;
    return 0;
};

fiber delegator(-> i) {
    fiber:i: sub = count(3);
    yield from sub;
    yield 100;
    return 0;
};

fiber:i: d = delegator();
for val in d do;
    >! val;
end;
>! d.isDone();
"#;

/// functions_fibers.md: functions with/without parameters, recursion.
const FUNCTIONS: &str = r#"
func add(i: x, i: y -> i) {
    return x + y;
};
func factorial(i: n -> i) {
    if (n <= 1) then;
        return 1;
    else;
        return n * factorial(n - 1);
    end;
};
func get_pi(-> f) {
    return 3.14159;
};
>! add(3, 4);
>! factorial(5);
>! get_pi() == 3.14159;
"#;

/// types.md: `i()` truncates toward zero, `f()` widens, `s()` stringifies.
const CASTS: &str = r#"
f: x = 3.7;
>! i(x);
i: m = 42;
f: y = f(m);
>! y;
i: num = 99;
>! s(num);
"#;

/// types.md: `date("YYYY-MM-DD")` and `%Y-%m-%d` display.
const DATES: &str = r#"
date: d = date("2024-12-25");
>! d;
"#;

#[test]
fn parity_arithmetic() {
    assert_eq!(
        common::assert_parity("parity_arith", ARITH),
        "16\n26\n3\n1\n1024\n4877\n-7"
    );
}

#[test]
fn parity_strings() {
    assert_eq!(
        common::assert_parity("parity_strings", STRINGS),
        "Hi User\nValue: 42\ntrue\nxcx_language\n3\nProg\n26\n3\n6\n11"
    );
}

#[test]
fn parity_control_flow() {
    assert_eq!(
        common::assert_parity("parity_control", CONTROL),
        "1\n2\n3\n0\n2\n4\n6\n1\n2\n3\n1\n3\nB"
    );
}

#[test]
fn parity_arrays() {
    assert_eq!(
        common::assert_parity("parity_arrays", ARRAYS),
        "99\n5\n15\n4\ntrue\n1\nfalse\n2"
    );
}

#[test]
fn parity_sets() {
    assert_eq!(
        common::assert_parity("parity_sets", SETS),
        "7\n3\n2\n4\ntrue\n5\n6\n2"
    );
}

#[test]
fn parity_maps() {
    assert_eq!(
        common::assert_parity("parity_maps", MAPS),
        "30\n3\ntrue\ntrue\n2\n2\n2\n2"
    );
}

#[test]
fn parity_json() {
    assert_eq!(
        common::assert_parity("parity_json", JSON),
        "9090\ntrue\nfalse\n2\nlocalhost\n2\n1\n10\n2\n41\n3\n3\n3"
    );
}

#[test]
fn parity_fibers() {
    // The trailing `0` is the fiber's `return 0` value: per functions_fibers.md
    // the for-loop is equivalent to `while (!isDone()) { next() }`, so the
    // return value is delivered as the last element before isDone flips.
    assert_eq!(
        common::assert_parity("parity_fibers", FIBERS),
        "0\n1\n2\n100\n0\ntrue"
    );
}

#[test]
fn parity_functions() {
    assert_eq!(
        common::assert_parity("parity_functions", FUNCTIONS),
        "7\n120\ntrue"
    );
}

#[test]
fn parity_casts() {
    assert_eq!(common::assert_parity("parity_casts", CASTS), "3\n42\n99");
}

#[test]
fn parity_dates() {
    assert_eq!(common::assert_parity("parity_dates", DATES), "2024-12-25");
}
