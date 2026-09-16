//! Ownership-contract regressions for the JSON runtime (crate public API).
//!
//! VM builtin convention: a returned `Value` carries **exactly one** strong
//! reference owned by the receiver. `Value` has no `Drop`, so a returned
//! strong_count above 1 pins the object (and, for json, its whole parse
//! tree) forever — the 2026-09-06 RAM leak class. These tests lock that
//! contract in; if one fails, the leak is back.
//!
//! Reading the assertions: while both the returned `Value` *and* the
//! accessor `Arc` are alive, `strong_count == 2` means "one owned reference
//! plus the accessor's borrow" — exactly one reference carried by the value.

use std::sync::Arc;
use xcx::runtime::builtin::json::parse::handle_json_parse;
use xcx::vm::object::JsonObj;
use xcx::vm::utils::json::json_val_to_value;
use xcx::vm::value::Value;

#[test]
fn parse_result_carries_exactly_one_reference_object() {
    let v: Value = handle_json_parse(r#"{"user": {"name": "Ala"}}"#);
    assert!(v.is_json());
    let arc: Arc<JsonObj> = v.as_json();
    assert_eq!(
        Arc::strong_count(&arc),
        2,
        "json.parse must return exactly one strong reference (value + accessor); \
         more means the parse tree is pinned forever"
    );
}

#[test]
fn parse_result_carries_exactly_one_reference_array() {
    let v: Value = handle_json_parse("[1, 2, 3]");
    let arc: Arc<JsonObj> = v.as_json();
    assert_eq!(Arc::strong_count(&arc), 2);
}

#[test]
fn parse_string_root_ownership_is_bounded() {
    // Cached path (payload <= 16 KiB): the parse cache legitimately keeps a
    // reference to the parsed tree, so the count seen through the accessor
    // is: value's own ref + cache's ref + accessor's borrow = 3. The
    // anti-leak property is that the count is STABLE across repeated parses
    // once each value's own reference is released (mimicking the VM's
    // explicit dec-on-overwrite).
    for _ in 0..20 {
        let v: Value = handle_json_parse(r#""plain string""#);
        assert!(v.is_string());
        let arc = v.as_string();
        let count = Arc::strong_count(&arc);
        unsafe { v.dec_ref(); }
        let after = Arc::strong_count(&arc);
        assert_eq!(
            count, 3,
            "parse + cache + accessor must be exactly 3; more means references accumulate"
        );
        assert_eq!(
            after, 2,
            "releasing the value's own reference must leave exactly cache + accessor"
        );
    }

    // Uncached path (payload > 16 KiB is never inserted into the cache):
    // the returned string must carry exactly one strong reference plus the
    // accessor's.
    let big = format!("\"{}\"", "x".repeat(17 * 1024));
    let v: Value = handle_json_parse(&big);
    assert!(v.is_string());
    let arc = v.as_string();
    let count = Arc::strong_count(&arc);
    unsafe { v.dec_ref(); }
    assert_eq!(count, 2);
    assert_eq!(Arc::strong_count(&arc), 1);
}

#[test]
fn parse_scalar_root_is_inline_int() {
    let v: Value = handle_json_parse("123");
    assert!(v.is_int());
    assert_eq!(v.as_i64(), 123);
}

#[test]
fn repeated_parse_is_independent_and_balanced() {
    // Two parses of the same payload: each result is a fresh wrapper
    // carrying exactly one reference; both are released when dropped.
    let a = handle_json_parse(r#"{"k": [1, 2, 3]}"#);
    let b = handle_json_parse(r#"{"k": [1, 2, 3]}"#);
    assert!(a.bits != b.bits, "each parse must hand out its own wrapper");
    let count_a = {
        let arc = a.as_json();
        Arc::strong_count(&arc)
    };
    let count_b = {
        let arc = b.as_json();
        Arc::strong_count(&arc)
    };
    assert_eq!(count_a, 2);
    assert_eq!(count_b, 2);
}

#[test]
fn json_val_to_value_wraps_with_exactly_one_reference() {
    let v: Value = handle_json_parse(r#"{"list": [1, 2]}"#);
    let root = {
        let arc = v.as_json();
        arc.root.clone()
    };
    let w = json_val_to_value(&root);
    assert!(w.is_json());
    let arc: Arc<JsonObj> = w.as_json();
    assert_eq!(Arc::strong_count(&arc), 2);
}
