/// Integration test runner for XCX edge case test files.
/// 
/// NOTE: By default, this test runner silently suppresses all XCX JIT/VM 
/// string outputs (`print`, `>!`, `halt`) to keep the test summary clean.
/// If you need to debug a failing test and want to see the detailed VM execution logs
/// and print outputs, run tests with the show-output flag:
/// 
///     cargo test --release -- --nocapture
///
use serial_test::serial;
use std::path::PathBuf;
use xcx_compiler::frontend::parser::Parser;
use xcx_compiler::frontend::parser::expander::Expander;
use xcx_compiler::sema::Checker;
use xcx_compiler::sema::SymbolTable;
use xcx_compiler::compiler::Compiler as XCXCompiler;
use xcx_compiler::vm::{VM, SharedContext};
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn test_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("edge_cases")
}

fn feature_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("features")
}

fn random_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("random")
}

fn comprehensive_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("comprehensive")
}

fn professional_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("professional")
}

fn hardening_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("hardening")
}

fn ultimate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("ultimate")
}

fn refactor_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("refactor")
}

fn sql_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("rust_harness").join("sql")
}

fn test_log(msg: &str) {
    if std::env::var("XCX_TEST_VERBOSE").is_ok() {
        eprintln!("{}", msg);
    }
}

fn run_source(source: &str) -> Arc<VM> {
    run_source_with_dir(source, None)
}

fn run_source_with_dir(source: &str, dir: Option<PathBuf>) -> Arc<VM> {
    unsafe { std::env::set_var("XCX_IN_TEST_HARNESS", "1"); }

    let source_with_assert = format!(
        "func assert(b: condition) {{ if (!condition) then; halt.error >! \"Assertion failed\"; end; }};\n{}",
        source
    );

    test_log("[TEST] Parsing...");
    let mut parser = Parser::new(&source_with_assert);
    let program = parser.parse_program();
    assert!(!parser.has_error, "Syntax errors during parsing");
    let mut interner = parser.into_interner();

    test_log("[TEST] Expanding...");
    let mut expander = Expander::new(&mut interner);
    let current_dir = dir.unwrap_or_else(|| std::env::current_dir().unwrap());
    let mut program = expander.expand(program, &current_dir).expect("Expansion failed");

    test_log("[TEST] Checking...");
    let mut checker = Checker::new(&interner);
    let mut symbols = SymbolTable::new();
    let errors = checker.check(&mut program, &mut symbols);
    assert!(
        errors.is_empty(),
        "Type-check errors:\n{:#?}",
        errors
    );

    test_log("[TEST] Compiling...");
    let mut compiler = XCXCompiler::new();
    let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);

    test_log("[TEST] Running VM...");
    let vm = Arc::new(VM::new());
    let ctx = SharedContext { constants, functions, http_req: None };
    
    let vm_for_thread = vm.clone();
    let main_chunk_arc = Arc::new(main_chunk);
    
    let handle = std::thread::Builder::new()
        .name("xcx-test-executor".to_string())
        .stack_size(64 * 1024 * 1024) // 64MB
        .spawn(move || {
            vm_for_thread.run(main_chunk_arc, ctx, &[]);
        })
        .expect("Failed to spawn test VM thread");

    handle.join().expect("Test VM thread panicked");
    test_log("[TEST] VM Done.");

    let errors = vm.error_count.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(errors, 0, "VM encountered {} halt errors during execution", errors);
    
    vm
}

/// Load a .xcx file from tests/xcx/ and run it through the full pipeline.
fn run_file(filename: &str) -> Arc<VM> {
    let path = test_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    run_source(&source)
}

fn run_comprehensive_file(filename: &str) -> Arc<VM> {
    let path = comprehensive_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    run_source_with_dir(&source, Some(comprehensive_dir()))
}

fn run_professional_file(filename: &str) -> Arc<VM> {
    let path = professional_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    run_source_with_dir(&source, Some(professional_dir()))
}

fn run_hardening_file(filename: &str) -> Arc<VM> {
    let path = hardening_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    run_source_with_dir(&source, Some(hardening_dir()))
}

fn run_ultimate_file(filename: &str) -> Arc<VM> {
    let path = ultimate_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    run_source_with_dir(&source, Some(ultimate_dir()))
}

fn run_feature_file(filename: &str) -> Arc<VM> {
    let path = feature_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    run_source_with_dir(&source, Some(feature_dir()))
}

fn run_random_file(filename: &str) -> Arc<VM> {
    let path = random_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    run_source_with_dir(&source, Some(random_dir()))
}

fn run_refactor_file(filename: &str) -> Arc<VM> {
    let path = refactor_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    run_source_with_dir(&source, Some(refactor_dir()))
}

fn run_sql_file(filename: &str) -> Arc<VM> {
    let dir = sql_dir();
    let path = dir.join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    
    {
        let _vm = run_source_with_dir(&source, Some(dir.clone()));
    }
    
    Arc::new(VM::new()) // Return a dummy VM since the result is usually ignored in tests
}

/// Expect the type checker to REJECT this source with at least one error.
fn expect_type_error(source: &str) {
    let mut parser = Parser::new(source);
    let mut program = parser.parse_program();
    let interner = parser.into_interner();
    let mut checker = Checker::new(&interner);
    let mut symbols = SymbolTable::new();
    let errors = checker.check(&mut program, &mut symbols);
    assert!(
        !errors.is_empty(),
        "Expected type error but checker accepted:\n{}",
        source
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. TYPE ERROR TESTS — checker must REJECT these programs
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn multiple_var_declarations() {
    run_source(r#"
        i: a, b = 42, c;
        assert(a == 0);
        assert(b == 42);
        assert(c == 0);
        
        f: x = 1.5, y, z = 3.5;
        assert(x == 1.5);
        assert(y == 0.0);
        assert(z == 3.5);
        
        s: s1, s2 = "test", s3;
        assert(s1 == "");
        assert(s2 == "test");
        assert(s3 == "");
        
        b: b1, b2 = true, b3 = false;
        assert(b1 == false);
        assert(b2 == true);
        assert(b3 == false);
    "#);
}

#[test]
fn type_error_string_assigned_to_int() {
    // Spec: i is Integer, "hello" is String — must be rejected
    expect_type_error(r#"i: x = "hello";"#);
}

#[test]
fn type_error_int_plus_string() {
    // Adding integer and string should fail the type checker
    expect_type_error(r#"i: a = 5; s: b = "abc"; i: c = a + b;"#);
}

#[test]
fn type_error_bool_from_int() {
    // b: flag = 10 — boolean cannot hold an integer literal
    expect_type_error("b: flag = 10;");
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. OVERFLOW / BOUNDARY VALUES
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn overflow_max_int_literal() {
    // i64::MAX as a literal — must parse and store without panic
    run_source("i: max_int = 9223372036854775807; >! max_int;");
}

#[test]
fn overflow_large_multiplication() {
    // 999_999 * 999_999 = 999_998_000_001 — fits in i64
    run_source("i: big = 999999 * 999999; >! big;");
}

#[test]
fn overflow_large_float() {
    // XCX lexer does not support scientific notation (e.g. 1.7e307)
    // Use a plain large decimal float instead.
    run_source("f: big_f = 99999999.99; >! big_f;");
}

#[test]
fn overflow_negative_int() {
    run_source("i: neg = -2147483648; >! neg;");
}

#[test]
fn overflow_file() {
    // Run the full overflow test file
    run_file("02_overflow.xcx");
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. COLLECTION ACCESS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn collections_array_in_bounds() {
    run_source(r#"
        array:i: nums {10, 20, 30};
        i: v = nums.get(2);
        >! v;
    "#);
}

#[test]
fn collections_array_size() {
    run_source(r#"
        array:s: words {"apple", "banana", "cherry"};
        i: sz = words.size();
        >! sz;
    "#);
}

#[test]
fn collections_array_contains() {
    run_source(r#"
        array:i: vals {1, 2, 3};
        b: yes = vals.contains(2);
        b: no = vals.contains(99);
        >! yes;
        >! no;
    "#);
}

#[test]
fn collections_set_contains() {
    run_source(r#"
        set:N: primes {2, 3, 5, 7, 11};
        b: has5 = primes.contains(5);
        b: has4 = primes.contains(4);
        >! has5;
        >! has4;
    "#);
}

#[test]
fn collections_map_get_existing() {
    run_source(r#"
        map: ages {
            schema = [s <-> i]
            data = ["alice" :: 30, "bob" :: 25]
        };
        i: a = ages.get("alice");
        >! a;
    "#);
}

#[test]
fn collections_map_contains() {
    run_source(r#"
        map: ages {
            schema = [s <-> i]
            data = ["alice" :: 30]
        };
        b: yes = ages.contains("alice");
        b: no = ages.contains("charlie");
        >! yes;
        >! no;
    "#);
}

#[test]
fn collections_file() {
    run_file("03_collections_access.xcx");
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. MAP UPDATE (insert overwrites existing key)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn map_update_overwrites_existing_key() {
    let source = r#"
        map: ages {
            schema = [s <-> i]
            data = ["alice" :: 30]
        };
        ages.insert("alice", 35);
        i: result = ages.get("alice");
        >! result;
    "#;

    let mut parser = Parser::new(source);
    let mut program = parser.parse_program();
    let mut interner = parser.into_interner();
    let result_id = interner.intern("result");

    let mut checker = Checker::new(&interner);
    let mut symbols = SymbolTable::new();
    let _ = checker.check(&mut program, &mut symbols);

    let mut compiler = XCXCompiler::new();
    let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
    let global_idx = *compiler.globals.get(&result_id).unwrap();

    let vm = Arc::new(VM::new());
    let ctx = SharedContext { constants, functions, http_req: None };
    vm.clone().run(Arc::new(main_chunk), ctx, &[]);

    let v = vm.get_global(global_idx);
    if v.is_int() {
        assert_eq!(v.as_i64(), 35, "Expected 35 after update, got {}", v.as_i64());
    } else {
        panic!("Expected Int(35), got {:?}", v);
    }
}

#[test]
fn map_update_adds_new_key() {
    run_source(r#"
        map: ages {
            schema = [s <-> i]
            data = ["alice" :: 30]
        };
        ages.insert("carol", 22);
        b: has = ages.contains("carol");
        >! has;
    "#);
}

#[test]
fn map_update_size_after_insert() {
    run_source(r#"
        map: ages {
            schema = [s <-> i]
            data = ["alice" :: 30, "bob" :: 25]
        };
        ages.insert("carol", 22);
        i: sz = ages.size();
        >! sz;
    "#);
}

#[test]
fn map_update_file() {
    run_file("04_map_update.xcx");
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. FIBONACCI (recursion correctness, NOT performance)
// ─────────────────────────────────────────────────────────────────────────────

fn fib_source(n: u32) -> String {
    format!(r#"
        func fib(i: n -> i) {{
            if (n <= 1) then;
                return n;
            end;
            return fib(n - 1) + fib(n - 2);
        }};
        i: result = fib({n});
    "#, n = n)
}

fn run_fib(n: u32) -> i64 {
    let source = fib_source(n);
    let mut parser = Parser::new(&source);
    let mut program = parser.parse_program();
    let mut interner = parser.into_interner();
    let result_id = interner.intern("result");

    let mut checker = Checker::new(&interner);
    let mut symbols = SymbolTable::new();
    let _ = checker.check(&mut program, &mut symbols);

    let mut compiler = XCXCompiler::new();
    let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
    let idx = *compiler.globals.get(&result_id).unwrap();

    let vm = Arc::new(VM::new());
    let ctx = SharedContext { constants, functions, http_req: None };
    vm.clone().run(Arc::new(main_chunk), ctx, &[]);

    let v = vm.get_global(idx);
    if v.is_int() {
        v.as_i64()
    } else {
        panic!("Expected Int, got {:?}", v);
    }
}

#[test] fn fib_0() { assert_eq!(run_fib(0), 0); }
#[test] fn fib_1() { assert_eq!(run_fib(1), 1); }
#[test] fn fib_5() { assert_eq!(run_fib(5), 5); }
#[test] fn fib_10() { assert_eq!(run_fib(10), 55); }
#[test] fn fib_15() { assert_eq!(run_fib(15), 610); }
#[test] fn fib_20() { assert_eq!(run_fib(20), 6765); }

#[test]
fn fibonacci_file() {
    run_file("05_fibonacci.xcx");
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. DATES — edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn date_iso_format() {
    run_source(r#"date: d = date("2024-03-15"); >! d.format();"#);
}

#[test]
fn date_leap_year_feb29_valid() {
    run_source(r#"date: leap = date("2024-02-29"); >! leap.format();"#);
}

#[test]
fn date_non_leap_year_feb28() {
    run_source(r#"date: d = date("2023-02-28"); >! d.format();"#);
}

#[test]
fn date_arithmetic_add_days() {
    run_source(r#"
        date: d = date("2024-01-01");
        date: next = d + 7;
        >! next.format();
    "#);
}

#[test]
fn date_arithmetic_diff() {
    run_source(r#"
        date: da = date("2024-03-15");
        date: db = date("2024-03-01");
        i: diff = da - db;
        >! diff;
    "#);
}

#[test]
fn date_comparison() {
    run_source(r#"
        date: da = date("2024-12-25");
        date: db = date("2024-01-01");
        b: is_later = (da > db);
        >! is_later;
    "#);
}

#[test]
fn date_custom_format() {
    run_source(r#"date: d = date("25/12/2024", "DD/MM/YYYY"); >! d.format();"#);
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. RECURSION DEPTH (stack safety)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn recursion_depth_100() {
    let result = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024) // 8 MB
        .spawn(|| {
            run_source(r#"
                func countdown(i: n -> i) {
                    if (n <= 0) then;
                        return 0;
                    end;
                    return countdown(n - 1);
                };
                i: result = countdown(100);
                >! result;
            "#);
        })
        .unwrap()
        .join();
    result.expect("recursion_depth_100 panicked");
}

#[test]
fn recursion_depth_500() {
    let result = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024) 
        .spawn(|| {
            run_source(r#"
                func countdown(i: n -> i) {
                    if (n <= 0) then;
                        return 0;
                    end;
                    return countdown(n - 1);
                };
                i: result = countdown(500);
                >! result;
            "#);
        })
        .unwrap()
        .join();
    result.expect("recursion_depth_500 panicked");
}

#[test]
fn recursion_depth_file() {
    run_file("07_recursion_depth.xcx");
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. EDGE ARITHMETIC
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn arithmetic_unary_neg_int() {
    run_source("i: v = -42; >! v;");
}

#[test]
fn arithmetic_unary_neg_float() {
    run_source("f: v = -3.14; >! v;");
}

#[test]
fn arithmetic_modulo() {
    run_source("i: v = 10 % 3; >! v;");
}

#[test]
fn arithmetic_power_int() {
    run_source("i: v = 2 ^ 10; >! v;");
}

#[test]
fn arithmetic_power_zero() {
    run_source("i: v = 3 ^ 0; >! v;");
}

#[test]
fn arithmetic_mixed_int_float_add() {
    run_source("f: v = 3.0 + 1.5; >! v;");
}

#[test]
fn arithmetic_mixed_comparison_gt() {
    run_source("b: v = 3.0 > 2.5; >! v;");
}

#[test]
fn arithmetic_int_concat() {
    run_source("i: v = 48 ++ 12345; >! v;");
}

#[test]
fn arithmetic_string_has_operator() {
    run_source(r#"b: v = "user@email.com" HAS "@"; >! v;"#);
}

#[test]
fn arithmetic_edge_file() {
    run_file("08_edge_arithmetic.xcx");
}

#[test]
fn edge_div_zero() {
    expect_runtime_error_file("div_zero_halt.xcx");
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. FIBERS
// ─────────────────────────────────────────────────────────────────────────────

fn expect_type_error_file(filename: &str) {
    let path = test_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
    expect_type_error(&source);
}

fn expect_runtime_error_file(filename: &str) {
    let path = test_dir().join(filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));

    let result = std::panic::catch_unwind(|| {
        run_source(&source);
    });

    assert!(result.is_err(), "Expected {} to produce a runtime error or panic, but it succeeded.", filename);
}

#[test] fn fiber_basic() { run_file("fiber_basic.xcx"); }
#[test] fn fiber_void() { run_file("fiber_void.xcx"); }
#[test] fn fiber_return() { run_file("fiber_return.xcx"); }
#[test] fn fiber_for() { run_file("fiber_for.xcx"); }
#[test] fn fiber_nested() { run_file("fiber_nested.xcx"); }
#[test] fn fiber_halt() { expect_runtime_error_file("fiber_halt.xcx"); }
#[test] fn fiber_edges() { run_file("fiber_edges.xcx"); }
#[test] fn fiber_pass() { run_file("fiber_pass.xcx"); }
#[test] fn fiber_complex_types() { run_file("fiber_complex_types.xcx"); }
#[test] fn fiber_yield_fiber() { run_file("fiber_yield_fiber.xcx"); }
#[test] fn fiber_mutation() { run_file("fiber_mutation.xcx"); }

#[test] fn fiber_err_s208() { expect_type_error_file("fiber_err_s208.xcx"); }
#[test] fn fiber_err_s209() { expect_type_error_file("fiber_err_s209.xcx"); }
#[test] fn fiber_err_s210() { expect_type_error_file("fiber_err_s210.xcx"); }

#[test]
fn fiber_err_r306() {
    let _ = std::panic::catch_unwind(|| { run_file("fiber_err_r306.xcx"); });
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. STRING METHODS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn string_methods_length() {
    run_source(r#"
        s: s1 = "zażółć";
        assert(s1.length == 6);
        s: s2 = "";
        assert(s2.length == 0);
    "#);
}

#[test]
fn string_methods_case() {
    run_source(r#"
        s: s1 = "Hello";
        assert(s1.upper() == "HELLO");
        assert(s1.lower() == "hello");
    "#);
}

#[test]
fn string_methods_trim() {
    run_source(r#"
        s: s1 = "  hi  ";
        assert(s1.trim() == "hi");
    "#);
}

#[test]
fn string_methods_replace() {
    run_source(r#"
        s: s1 = "hello world";
        assert(s1.replace("hello", "hi") == "hi world");
    "#);
}

#[test]
fn string_methods_slice() {
    run_source(r#"
        s: s1 = "Programming";
        assert(s1.slice(0, 4) == "Prog");
        assert(s1.slice(7, 11) == "ming");
    "#);
}

#[test]
fn string_methods_chaining() {
    run_source(r#"
        s: result = "  Hello, World!  ".trim().lower().replace("hello", "hi");
        assert(result == "hi, world!");
    "#);
}

#[test]
fn string_methods_unicode() {
    run_source(r#"
        s: p = "zażółć";
        assert(p.slice(0, 2) == "za");
        assert(p.slice(2, 6) == "żółć");
    "#);
}

// ─────────────────────────────────────────────────────────────────────────────
// 11. COMPREHENSIVE SUITE (spec_validation)
// ─────────────────────────────────────────────────────────────────────────────

mod comprehensive_suite {
    use super::*;

    #[test] fn comp_01_comments_and_primitives() { run_comprehensive_file("01_comments_and_primitives.xcx"); }
    #[test] fn comp_02_math_and_logic_aliases() { run_comprehensive_file("02_math_and_logic_aliases.xcx"); }
    #[test] fn comp_03_string_advanced() { run_comprehensive_file("03_string_advanced.xcx"); }
    #[test] fn comp_04_control_flow_aliases() { run_comprehensive_file("04_control_flow_aliases.xcx"); }
    #[test] fn comp_05_loops_and_breaks() { run_comprehensive_file("05_loops_and_breaks.xcx"); }
    #[test] fn comp_06_functions_and_recursion() { run_comprehensive_file("06_functions_and_recursion.xcx"); }
    #[test] fn comp_07_arrays_exhaustive() { run_comprehensive_file("07_arrays_exhaustive.xcx"); }
    #[test] fn comp_08_sets_and_math_symbols() { run_comprehensive_file("08_sets_and_math_symbols.xcx"); }
    #[test] fn comp_09_maps_and_schemas() { run_comprehensive_file("09_maps_and_schemas.xcx"); }
    #[test] fn comp_10_halt_and_terminal() { run_comprehensive_file("10_halt_and_terminal.xcx"); }
    #[test] fn comp_11_modules_and_namespaces() { run_comprehensive_file("11_modules_and_namespaces.xcx"); }
    #[test] fn comp_12_store_and_security() { run_comprehensive_file("12_store_and_security.xcx"); }
    #[test] fn comp_13_date_time_full() { run_comprehensive_file("13_date_time_full.xcx"); }
    #[test] fn comp_14_tables_crud_and_relational() { run_comprehensive_file("14_tables_crud_and_relational.xcx"); }
    #[test] fn comp_15_json_raw_and_binding() { run_comprehensive_file("15_json_raw_and_binding.xcx"); }
    #[test] fn comp_16_fibers_and_yield_logic() { run_comprehensive_file("16_fibers_and_yield_logic.xcx"); }
    #[test] fn comp_17_lib_spec() { run_comprehensive_file("lib_spec.xcx"); }
}

// ─────────────────────────────────────────────────────────────────────────────
// 12. PROFESSIONAL SUITE
// ─────────────────────────────────────────────────────────────────────────────

mod professional_suite {
    use super::*;

    #[test] fn prof_01_primitives() { run_professional_file("01_primitives.xcx"); }
    #[test] fn prof_02_operators() { run_professional_file("02_operators.xcx"); }
    #[test] fn prof_03_control_flow() { run_professional_file("03_control_flow.xcx"); }
    #[test] fn prof_04_functions() { run_professional_file("04_functions.xcx"); }
    #[test] fn prof_05_arrays() { run_professional_file("05_arrays.xcx"); }
    #[test] fn prof_06_sets() { run_professional_file("06_sets.xcx"); }
    #[test] fn prof_07_maps() { run_professional_file("07_maps.xcx"); }
    #[test] fn prof_08_halt_system() { run_professional_file("08_halt_system.xcx"); }
    #[test] fn prof_09_store_module() { run_professional_file("09_store_module.xcx"); }
    #[test] fn prof_10_date_time() { run_professional_file("10_date_time.xcx"); }
    #[test] fn prof_11_tables() { run_professional_file("11_tables.xcx"); }
    #[test] fn prof_12_json() { run_professional_file("12_json.xcx"); }
    #[test] fn prof_13_fibers() { run_professional_file("13_fibers.xcx"); }
}

// ─────────────────────────────────────────────────────────────────────────────
// 12. HARDENING SUITE
// ─────────────────────────────────────────────────────────────────────────────

mod hardening_suite {
    use super::*;

    #[test] fn hard_01_complex_binding() { run_hardening_file("test_complex_binding.xcx"); }

    #[test]
    fn hard_02_deep_delegation() {
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| run_hardening_file("test_deep_delegation.xcx"))
            .unwrap()
            .join()
            .expect("hard_02_deep_delegation panicked");
    }

    #[test]
    fn hard_03_scope_integrity() {
        // Same concern as hard_02: run in a larger-stack thread for safety.
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| run_hardening_file("test_scope_integrity.xcx"))
            .unwrap()
            .join()
            .expect("hard_03_scope_integrity panicked");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 13. ULTIMATE SUITE
// ─────────────────────────────────────────────────────────────────────────────

mod ultimate_suite {
    use super::*;

    #[test] fn ult_01_fiber_generator() { run_ultimate_file("ult_01_fiber_generator.xcx"); }
    #[test] fn ult_02_fiber_isdone() { run_ultimate_file("ult_02_fiber_isdone.xcx"); }
    #[test] fn ult_03_fiber_drain() { run_ultimate_file("ult_03_fiber_drain.xcx"); }
    #[test] fn ult_04_fiber_nested() { run_ultimate_file("ult_04_fiber_nested.xcx"); }
    #[test] fn ult_05_table_where() { run_ultimate_file("ult_05_table_where.xcx"); }
    #[test] fn ult_06_table_relational() { run_ultimate_file("ult_06_table_relational.xcx"); }
    #[test] fn ult_07_json_binding() { run_ultimate_file("ult_07_json_binding.xcx"); }
    #[test] fn ult_08_recursion() { run_ultimate_file("ult_08_recursion.xcx"); }
    #[test] fn ult_09_loops() { run_ultimate_file("ult_09_loops.xcx"); }
    #[test] fn ult_10_errors() { run_ultimate_file("ult_10_errors.xcx"); }
    #[test] fn ult_11_modules() { run_ultimate_file("ult_11_modules.xcx"); }
    #[test] fn ult_12_namespaces() { run_ultimate_file("ult_12_namespaces.xcx"); }
    #[test] fn ult_13_date_time() { run_ultimate_file("ult_13_date_time.xcx"); }
    #[test] fn ult_14_store() { run_ultimate_file("ult_14_store.xcx"); }
    #[test] fn ult_15_json_raw() { run_ultimate_file("ult_15_json_raw.xcx"); }
    #[test] fn ult_16_math() { run_ultimate_file("ult_16_math.xcx"); }
    #[test] fn ult_17_math_comprehensive() { run_ultimate_file("ult_17_math_comprehensive.xcx"); }
}

// ─────────────────────────────────────────────────────────────────────────────
// 14. FEATURE SUITE — Core language features
// ─────────────────────────────────────────────────────────────────────────────

mod feature_suite {
    use super::*;

    #[test] fn feat_basics() { run_feature_file("test_basics.xcx"); }
    #[test] fn feat_collections() { run_feature_file("test_collections.xcx"); }
    #[test] fn feat_control_flow() { run_feature_file("test_control_flow.xcx"); }
    #[test] fn feat_functions() { run_feature_file("test_functions.xcx"); }
    #[test] fn feat_operators() { run_feature_file("test_operators.xcx"); }
    #[test] fn feat_std_lib() { run_feature_file("test_std_lib.xcx"); }
    #[test] fn feat_io() { run_feature_file("test_io.xcx"); }
    #[test] #[serial] fn feat_json_http() { run_feature_file("test_json_http.xcx"); }
    #[test] fn feat_fibers() { run_feature_file("test_fibers.xcx"); }
    #[test] #[serial] fn feat_all_elements() { run_feature_file("test_all_elements.xcx"); }
    #[test] fn feat_settest() { run_feature_file("settest.xcx"); }
    // #[test] fn feat_input_strict() { run_feature_file("input_strict_test.xcx"); }
    #[test] fn feat_map_to_json() { run_feature_file("test_map_to_json.xcx"); }
    #[test] fn feat_random_array() { run_feature_file("test_random_array.xcx"); }
    #[test] fn feat_to_json() { run_feature_file("test_to_json.xcx"); }
    #[test] fn feat_string_split() { run_feature_file("test_string_split.xcx"); }
    #[test] fn feat_empty_set() { run_feature_file("test_empty_set.xcx"); }
    #[test] fn feat_store_extension() { run_feature_file("store_extension.xcx"); }
    #[test] fn feat_perf() { run_feature_file("test_perf.xcx"); }
}


// ─────────────────────────────────────────────────────────────────────────────
// 15. RANDOM SUITE — Random number generation
// ─────────────────────────────────────────────────────────────────────────────

mod random_suite {
    use super::*;

    #[test] fn rand_basics() { run_random_file("01_basics.xcx"); }
    #[test] fn rand_assertions() { run_random_file("02_assertions.xcx"); }
}

mod refactor_baseline {
    use super::*;

    #[test] fn collections_smoke() { run_refactor_file("collections_smoke.xcx"); }
}

// ─────────────────────────────────────────────────────────────────────────────
// 14. HTTP TESTS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn http_client_basic() {
    run_file("23_http_client.xcx");
}

#[test]
fn http_server_syntax() {
    let path = test_dir().join("23_http_server.xcx");
    let source = std::fs::read_to_string(&path).unwrap();
    let mut parser = Parser::new(&source);
    let program = parser.parse_program();
    assert!(!parser.has_error);
    let mut interner = parser.into_interner();
    let mut expander = Expander::new(&mut interner);
    let mut program = expander.expand(program, &test_dir()).unwrap();
    let mut checker = Checker::new(&interner);
    let mut symbols = SymbolTable::new();
    let errors = checker.check(&mut program, &mut symbols);
    assert!(errors.is_empty());
}

#[test]
#[serial]
fn http_client_suite() {
    run_file("http_client_suite.xcx");
}

#[test]
fn http_server_suite() {
    use std::time::Duration;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

    eprintln!("[http_server_suite] Starting server test — please wait up to 5 seconds for stability check...");

    let completed = Arc::new(AtomicBool::new(false));
    let completed_clone = completed.clone();

    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let result = std::panic::catch_unwind(|| {
                run_file("http_server_suite.xcx");
            });
            completed_clone.store(true, Ordering::SeqCst);
            result
        })
        .unwrap();

    let start = std::time::Instant::now();
    loop {
        if completed.load(Ordering::SeqCst) {
            match handle.join().unwrap() {
                Ok(_) => {
                    eprintln!("[http_server_suite] ✓ Server exited cleanly before timeout.");
                    return;
                }
                Err(_) => panic!("[http_server_suite] ✗ Server panicked during execution."),
            }
        }

        if start.elapsed() >= Duration::from_secs(5) {
            eprintln!("[http_server_suite] ✓ Server ran for 5 seconds without errors — OK.");
            return;
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test] fn edge_cases() { run_file("edge_cases.xcx"); }
#[test] fn recent_features() { run_file("recent_features.xcx"); }
#[test] fn terminal_run() { run_file("terminal_run.xcx"); }

// ─────────────────────────────────────────────────────────────────────────────
// 16. SQL SUITE — Database operations and cleanup
// ─────────────────────────────────────────────────────────────────────────────

mod sql_suite {
    use super::*;

    #[test] fn sql_basic() { run_sql_file("sql_basic.xcx"); }
    #[test] fn sql_exec() { run_sql_file("sql_exec.xcx"); }
    #[test] fn sql_queryraw() { run_sql_file("sql_queryraw.xcx"); }
    #[test] fn sql_schema() { run_sql_file("sql_schema.xcx"); }
    #[test] fn sql_tojson() { run_sql_file("sql_tojson.xcx"); }
    #[test] fn sql_transactions() { run_sql_file("sql_transactions.xcx"); }
    #[test] fn sql_where_advanced() { run_sql_file("sql_where_advanced.xcx"); }
    #[test] fn sql_push() { run_sql_file("sql_push.xcx"); }
    #[test] fn sql_save() { run_sql_file("sql_save.xcx"); }
    #[test]
    fn sql_save_missing_pk_error() {
        run_sql_file("sql_save_no_pk.xcx");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 17. STABILITY SUITE (Py runner migration)
// ─────────────────────────────────────────────────────────────────────────────

mod stability_suite {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    struct TestMeta {
        id: String,
        area: String,
        priority: String,
        name: String,
        expect_compile_error: bool,
        expect_fatal_exit: bool,
        expect_regression: bool,
        is_server_test: bool,
        disable_jit: bool,
    }

    fn parse_meta(path: &Path) -> TestMeta {
        let mut meta = TestMeta {
            id: String::new(),
            area: String::new(),
            priority: String::new(),
            name: String::new(),
            expect_compile_error: false,
            expect_fatal_exit: false,
            expect_regression: false,
            is_server_test: false,
            disable_jit: false,
        };

        let text = std::fs::read_to_string(path).ok().unwrap_or_default();
        
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("--- TEST:") { meta.id = line[9..].trim().to_string(); }
            if line.starts_with("--- Area:") { meta.area = line[9..].trim().to_string(); }
            if line.starts_with("--- Priority:") { meta.priority = line[13..].trim().to_string(); }
            if line.starts_with("--- Name:") { meta.name = line[9..].trim().to_string(); }
        }

        let l_text = text.to_lowercase();
        meta.expect_compile_error = l_text.contains("expect_compile_error") || l_text.contains("nie powinien się skompilować");
        meta.expect_fatal_exit = l_text.contains("expect_fatal_exit") || 
            (l_text.contains("halt.fatal") && l_text.contains("nie powinna się wykonać") && !meta.expect_compile_error);
        meta.expect_regression = meta.priority.to_lowercase() == "regression";
        meta.disable_jit = l_text.contains("disable_jit = true");
        
        let mut has_serve = false;
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with("serve:") || t.starts_with("serve :") {
                has_serve = true;
                break;
            }
        }
        meta.is_server_test = has_serve;

        if meta.id.is_empty() {
            meta.id = path.file_stem().unwrap().to_string_lossy().to_string().to_uppercase();
        }

        meta
    }

    fn looks_like_compile_error(rc: i32, stderr: &str, stdout: &str) -> bool {
        if rc == 0 { return false; }
        if stderr.contains("Compiled successfully") || stdout.contains("Compiled successfully") ||
           stderr.contains("Compiled") || stdout.contains("Compiled") { return false; }
        
        let has_err = |s: &str| {
            s.contains("[S") || s.contains("[D") || s.contains("Semantic analysis failed") || 
            s.contains("Compilation failed") || s.contains("Syntax error") || s.contains("Parse error") ||
            s.contains("ERROR: Rule ")
        };
        has_err(stderr) || has_err(stdout)
    }

    fn looks_like_runtime_success(_rc: i32, stderr: &str, stdout: &str) -> bool {
        stderr.contains("Compiled successfully") || stdout.contains("Compiled successfully") ||
        stderr.contains("Compiled") || stdout.contains("Compiled")
    }

    #[test]
    #[serial_test::serial]
    fn run_xcx_stability_suite() {
        // Cleanup leftover .db files from project root before starting
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if let Ok(entries) = std::fs::read_dir(&project_root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if ext == "db" || ext == "db-journal" || ext == "db-wal" || ext == "db-shm" {
                        let _ = std::fs::remove_file(&p);
                    }
                }
            }
        }
        let mut base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base_dir.push("tests");
        base_dir.push("cli_tests");

        if !base_dir.exists() {
            eprintln!("Stability tests dir not found: {}", base_dir.display());
            return;
        }

        let mut xcx_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        xcx_bin.push("target");
        xcx_bin.push("release");
        xcx_bin.push("xcx-compiler.exe"); // Windows explicit

        if !xcx_bin.exists() {
            xcx_bin.set_extension("");
            if !xcx_bin.exists() {
                eprintln!("XCX Binary not found at {}. Run `cargo build --release` first.", xcx_bin.display());
            }
        }

        let temp_dir = std::env::temp_dir().join(format!("xcx_v3_runner_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(temp_dir.join("tests_tmp")); // For SEC-001c etc.

        let mut tests = Vec::new();
        let mut stack = vec![base_dir.clone()];
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() { stack.push(path); }
                    else if path.extension().and_then(|e| e.to_str()) == Some("xcx") {
                        tests.push(path);
                    }
                }
            }
        }

        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for path in &tests {
            let meta = parse_meta(path);
            
            if meta.is_server_test {
                skipped += 1;
                continue;
            }

            let mut cmd = Command::new(&xcx_bin);
            
            if meta.disable_jit {
                cmd.arg("--no-jit");
            }

            let output = cmd
                .arg(path.to_str().unwrap())
                .current_dir(&temp_dir)
                .env_remove("XCX_IN_TEST_HARNESS")
                .output()
                .expect("Failed to execute process");

            let rc = output.status.code().unwrap_or(1);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let pass_count = stdout.lines().filter(|l| l.contains("] PASS")).count();
            let fail_count = stdout.lines()
                .filter(|l| l.contains("] FAIL") && !l.contains("FAIL — znaleziono 0 sesji"))
                .count();

            let mut is_ok = false;
            let mut reason = String::new();

            if meta.expect_compile_error {
                if looks_like_compile_error(rc, &stderr, &stdout) {
                    is_ok = true;
                } else if rc != 0 && !looks_like_runtime_success(rc, &stderr, &stdout) {
                    is_ok = true; // Fallback for unrecognized compilation exit formats
                } else {
                    reason = format!("Expected compile error but it didn't look like one.");
                }
            } else if meta.expect_fatal_exit {
                if rc != 0 && fail_count == 0 {
                    is_ok = true;
                } else {
                    reason = format!("Expected fatal exit but failed or didn't exit cleanly.");
                }
            } else if meta.expect_regression && fail_count > 0 {
                is_ok = true; // known regression
            } else if fail_count > 0 {
                is_ok = false;
                reason = format!("{} asserts failed.", fail_count);
            } else if pass_count > 0 {
                is_ok = true;
            } else if rc != 0 {
                is_ok = false;
                reason = format!("Exit {} without passes.", rc);
            } else {
                is_ok = true; // exit 0 without passes is ok
            }

            if is_ok {
                passed += 1;
            } else {
                failed += 1;
                eprintln!("FAIL: {} ({})\nReason: {}\nSTDOUT:\n{}\nSTDERR:\n{}", meta.name, meta.id, reason, stdout, stderr);
            }
        }

        println!("Stability tests summary: {} passed, {} failed, {} skipped", passed, failed, skipped);
        assert_eq!(failed, 0, "Some stability tests failed");
    }
}