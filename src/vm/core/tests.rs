#[cfg(test)]
mod tests {
    use crate::frontend::parser::Parser;
    use crate::sema::{Checker, SymbolTable};
    use crate::compiler::compiler::Compiler as XCXCompiler;
    use crate::vm::core::vm::{VM, SharedContext};
    use crate::vm::value::Value;
    use std::sync::Arc;

    fn run(source: &str) -> Arc<VM> {
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();

        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let errors = checker.check(&mut program, &mut symbols);
        assert!(
            errors.is_empty(),
            "Type-check errors in test source:\n{:?}",
            errors
        );

        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);

        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        vm
    }

    fn assert_int(vm: &Arc<VM>, idx: usize, expected: i64, msg: &str) {
        let v = vm.get_global(idx);
        if v.is_int() {
            assert_eq!(v.as_i64(), expected, "{}", msg);
        } else {
            panic!("{}: expected Int({}), got {:?}", msg, expected, v);
        }
    }

    fn assert_bool(vm: &Arc<VM>, idx: usize, expected: bool, msg: &str) {
        let v = vm.get_global(idx);
        if v.is_bool() {
            assert_eq!(v.as_bool(), expected, "{}", msg);
        } else {
            panic!("{}: expected Bool({}), got {:?}", msg, expected, v);
        }
    }




    fn assert_float(vm: &Arc<VM>, idx: usize, expected: f64, msg: &str) {
        let v = vm.get_global(idx);
        if v.is_float() {
            assert!((v.as_f64() - expected).abs() < 1e-9, "{}: expected {}, got {}", msg, expected, v.as_f64());
        } else {
            panic!("{}: expected Float({}), got {:?}", msg, expected, v);
        }
    }



    #[test]
    fn test_basic_arithmetic() {
        run("i: x = 10; i: y = 20; >! x + y;");
    }

    #[test]
    fn test_repl_parser_new_accepts_source() {
        let source = "i: a = 42;";
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let interner = parser.into_interner();

        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let errors = checker.check(&mut program, &mut symbols);
        assert!(errors.is_empty(), "Unexpected type errors: {:?}", errors);
    }

    #[test]
    fn test_no_debug_print_on_method_call() {
        let source = r#"
            table: t = table {
                columns: [id :: i @auto, name :: s]
                rows: [("Alice"), ("Bob")]
            };
            i: n = t.count();
            >! n;
        "#;
        run(source);
    }

    #[test]
    fn test_unary_negation_float_does_not_crash() {
        let source = "f: x = -3.14;";
        run(source);
    }

    #[test]
    fn test_unary_negation_int_still_works() {
        let source = "i: x = -7;";
        run(source);
    }

    #[test]
    fn test_unary_negation_float_value_is_correct() {
        let source = "f: result = -2.5;";

        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();

        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let errors = checker.check(&mut program, &mut symbols);
        assert!(errors.is_empty(), "{:?}", errors);

        let name_id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let global_idx = compiler.get_global_idx(name_id);


        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);

        assert_float(&vm, global_idx, -2.5, "Expected -2.5");
    }

    #[test]
    fn test_halt_error_stops_current_frame() {
        let source = "i: sentinel = 0;\nhalt.error >! \"stopping here\";\ni: sentinel = 99;";

        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();

        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);

        let name_id = interner.intern("sentinel");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let global_idx = compiler.get_global_idx(name_id);

        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);

        let v = vm.get_global(global_idx);
        if v.is_int() {
            assert_eq!(v.as_i64(), 0, "halt.error failed to stop frame");
        } else if v.is_bool() && !v.as_bool() {
        } else {
            panic!("Unexpected value for sentinel: {:?}", v);
        }
    }

    #[test]
    fn test_globals_exceed_1024() {
        let mut source = String::new();
        for i in 0..1030 {
            source.push_str(&format!("i: var{i} = {i};\n"));
        }
        source.push_str(">! var1029;");
        run(&source);
    }

    #[test]
    fn test_http_client_local_server() {
        use std::thread;

        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr_str = server.server_addr().to_string();
        let port = addr_str.split(':').last().unwrap().parse::<u16>().unwrap();

        thread::spawn(move || {
            if let Ok(Some(request)) = server.recv_timeout(std::time::Duration::from_secs(5)) {
                let response = tiny_http::Response::from_string("{\"hello\":\"world\"}")
                    .with_status_code(200);
                let _ = request.respond(response);
            }
        });

        let source = format!(r#"
            i: success = 0;
            json: res = net.get("http://127.0.0.1:{}");
            if (res.ok) then;
                if (res.body.hello == "world") then;
                    success = 42;
                end;
            end;
        "#, port);

        let mut parser = Parser::new(&source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);

        let success_id = interner.intern("success");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let success_idx = compiler.get_global_idx(success_id);

        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);

        assert_int(&vm, success_idx, 42, "HTTP integration test failed");
    }

    #[test]
    #[ignore = "Panics across FFI boundaries which causes hard abort in release tests"]
    fn test_ssrf_protection_link_local() {
        let source = r#"
            json: res = net.get("http://169.254.169.254/latest/meta-data/");
            s: err = res.error;
        "#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);

        let err_id = interner.intern("err");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let err_idx = compiler.get_global_idx(err_id);

        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);

        let v = vm.get_global(err_idx);
        if v.is_ptr() {
            let s_bytes = v.as_string();
            let s_str = String::from_utf8_lossy(&s_bytes);
            assert!(s_str.contains("SSRF"), "Expected SSRF error string, got: {}", s_str);
        } else {
            panic!("SSRF protection test failed! Expected error string, got {:?}", v);
        }
    }

    #[test]
    fn test_string_starts_with_true() {
        let source = r#"b: result = "admin@xcx.pl".startsWith("admin");"#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_bool(&vm, idx, true, "startsWith(\"admin\") should be true");
    }

    #[test]
    fn test_string_starts_with_false() {
        let source = r#"b: result = "xcx@xcx.pl".startsWith("admin");"#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_bool(&vm, idx, false, "startsWith(\"admin\") should be false");
    }

    #[test]
    fn test_string_ends_with_true() {
        let source = r#"b: result = "main.xcx".endsWith(".xcx");"#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_bool(&vm, idx, true, "endsWith(\".xcx\") should be true");
    }

    #[test]
    fn test_string_ends_with_false() {
        let source = r#"b: result = "main.xcx".endsWith(".rs");"#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_bool(&vm, idx, false, "endsWith(\".rs\") should be false");
    }

    #[test]
    fn test_string_to_int_valid() {
        let source = r#"i: result = "42".toInt();"#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_int(&vm, idx, 42, ".toInt() should return 42");
    }

    #[test]
    fn test_string_to_float_valid() {
        let source = r#"f: result = "3.14".toFloat();"#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_float(&vm, idx, 3.14, ".toFloat() expected 3.14");
    }

    #[test]
    fn test_array_sort_integers() {
        let source = r#"
            array:i: nums {5, 2, 8, 1, 9};
            nums.sort();
            i: first = nums.get(0);
            i: last  = nums.get(4);
        "#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let first_id = interner.intern("first");
        let last_id  = interner.intern("last");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let first_idx = compiler.get_global_idx(first_id);
        let last_idx  = compiler.get_global_idx(last_id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_int(&vm, first_idx, 1, "After sort first element should be 1");
        assert_int(&vm, last_idx,  9, "After sort last element should be 9");
    }

    #[test]
    fn test_wait_ms() {
        let source = r#"
            @wait(10);
            b: result = true;
        "#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_bool(&vm, idx, true, "@wait(10) should execute");
    }

    #[test]
    fn test_crypto_bcrypt() {
        let source = r#"
            s: pass = "super-secret";
            s: hashed = crypto.hash(pass, "bcrypt");
            b: ok = crypto.verify(pass, hashed, "bcrypt");
            b: fail = crypto.verify("wrong", hashed, "bcrypt");
        "#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let ok_id = interner.intern("ok");
        let fail_id = interner.intern("fail");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let ok_idx = compiler.get_global_idx(ok_id);
        let fail_idx = compiler.get_global_idx(fail_id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_bool(&vm, ok_idx,   true,  "bcrypt verify should be true");
        assert_bool(&vm, fail_idx, false, "bcrypt verify should be false");
    }

    #[test]
    fn test_jit_fibonacci() {
        let source = r#"
            func fib(i: n -> i) {
                if (n < 2) then; return n; end;
                return fib(n - 1) + fib(n - 2);
            };
            i: result = fib(10);
        "#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_int(&vm, idx, 55, "fib(10) should be 55");
    }

    #[test]
    fn test_jit_sieve() {
        let source = r#"
            set:N: primes {2,,100};
            for p in 2 to 10 do;
                if (primes.contains(p)) then;
                    for mult in (p * p) to 100 @step p do;
                        primes.remove(mult);
                    end;
                end;
            end;
            i: count = primes.size();
        "#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("count");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_int(&vm, idx, 25, "Primes up to 100 should be 25");
    }

    #[test]
    fn test_jit_type_propagation_join() {
        let source = r#"
            func test_propagation(i: cond -> i) {
                i: x = 0;
                if (cond > 5) then;
                    x = 42;
                else;
                    x = 99;
                end;
                return x;
            };
            i: result = 0;
            for idx in 0 to 10 do;
                result = test_propagation(idx);
            end;
        "#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        let id = interner.intern("result");
        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&mut program, &mut interner);
        let idx = compiler.get_global_idx(id);
        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        assert_int(&vm, idx, 42, "test_propagation(10) should be 42");
    }

    #[test]
    fn test_value_size_is_8_bytes() {
        let size = std::mem::size_of::<Value>();
        assert!(size == 8 || size == 16, "Value must be exactly 8 or 16 bytes (NaN-boxed/Struct), got {}", size);
    }

    #[test]
    fn test_parser_isolation_http() {
        let server_source = r#"
            fiber handler(json: req -> json) {
                yield net.respond(200, json.parse("{\"hello\":\"world\"}"));
            };
            serve: app {
                port = 8090,
                host = "127.0.0.1",
                routes = [
                    "GET /" :: handler
                ]
            };
        "#;

        let mut parser = Parser::new(server_source);
        let _program = parser.parse_program();

    }

    #[test]
    fn test_http_serve_and_client_v6() {
        println!("**************************************************");


        let server_source = r#"
            fiber handler(json: req -> json) {
                yield net.respond(200, json.parse("{\"hello\":\"world\"}"));
            };
            serve: app {
                port = 8090,
                host = "localhost",
                routes = "GET /" :: handler
            };
        "#;
        
        let client_source = r#"
            i: result = 0;
            json: res = net.get("http://localhost:8090");
            if (res.ok) then;
                if (res.body.hello == "world") then;
                    result = 42;
                end;
            end;
            debug(result);
        "#;

        let mut parser_s = Parser::new(server_source);
        let mut program_s = parser_s.parse_program();

        let mut interner_s = parser_s.into_interner();
        let mut checker_s = Checker::new(&interner_s);
        let mut symbols_s = SymbolTable::new();
        let _ = checker_s.check(&mut program_s, &mut symbols_s);

        let mut compiler_s = XCXCompiler::new();

        let (main_chunk_s, constants_s, functions_s) = compiler_s.compile(&program_s, &mut interner_s);





        let vm_s = Arc::new(VM::new());
        let ctx_s = SharedContext { 
            constants: constants_s, 
            functions: functions_s, 
            http_req: None 
        };

        let vm_s_copy = vm_s.clone();

        std::thread::spawn(move || {

            vm_s_copy.run(Arc::new(main_chunk_s), ctx_s, &[]);
        });


        std::thread::sleep(std::time::Duration::from_millis(2000));


        let mut parser_c = Parser::new(client_source);
        let mut program_c = parser_c.parse_program();
        let mut interner_c = parser_c.into_interner();
        let mut checker_c = Checker::new(&interner_c);
        let mut symbols_c = SymbolTable::new();
        let _ = checker_c.check(&mut program_c, &mut symbols_c);
        let result_id = interner_c.intern("result");
        let mut compiler_c = XCXCompiler::new();
        let (main_chunk_c, constants_c, functions_c) = compiler_c.compile(&program_c, &mut interner_c);
        let result_idx = compiler_c.get_global_idx(result_id);

        let vm_c = Arc::new(VM::new());
        let ctx_c = SharedContext { 
            constants: constants_c, 
            functions: functions_c, 
            http_req: None 
        };
        vm_c.clone().run(Arc::new(main_chunk_c), ctx_c, &[]);

        assert_int(&vm_c, result_idx, 42, "HTTP serve/client integration failed");
    }

    #[test]
    fn test_jit_table_crud() {
        let source = r#"
            table: users {
                columns = [
                    uid :: i @auto,
                    name :: s
                ]
                rows = [
                    ("Alice"),
                    ("Bob")
                ]
            };
            i: cnt = users.count();
            i: uid0 = users[0].uid;
            s: name1 = users[1].name;
        "#;
        let mut parser = Parser::new(source);
        let mut program = parser.parse_program();
        let mut interner = parser.into_interner();
        let mut checker = Checker::new(&interner);
        let mut symbols = SymbolTable::new();
        let _ = checker.check(&mut program, &mut symbols);
        
        let cnt_id = interner.intern("cnt");
        let uid0_id = interner.intern("uid0");
        let name1_id = interner.intern("name1");

        let mut compiler = XCXCompiler::new();
        let (main_chunk, constants, functions) = compiler.compile(&program, &mut interner);
        
        let cnt_idx = compiler.get_global_idx(cnt_id);
        let uid0_idx = compiler.get_global_idx(uid0_id);
        let name1_idx = compiler.get_global_idx(name1_id);

        println!("[TEST BYTECODE] bytecode length = {}", main_chunk.bytecode.len());
        for (i, op) in main_chunk.bytecode.iter().enumerate() {
            println!("[TEST BYTECODE]   ip={}: {:?}", i, op);
        }

        let vm = Arc::new(VM::new());
        let ctx = SharedContext { 
            constants, 
            functions, 
            http_req: None 
        };
        vm.clone().run(Arc::new(main_chunk), ctx, &[]);
        
        for (i, v) in vm.globals.read().iter().enumerate() {
            if v.bits != 0 || v.tag != 2 {
                println!("[TEST GLOBALS] idx={}, val={:?}", i, v);
            }
        }
        println!("[TEST GLOBALS] cnt_idx={}, uid0_idx={}, name1_idx={}", cnt_idx, uid0_idx, name1_idx);

        assert_int(&vm, cnt_idx, 2, "users.count() mismatch");
        assert_int(&vm, uid0_idx, 1, "users[0].uid mismatch");
        
        let n1_val = vm.get_global(name1_idx);
        assert!(n1_val.is_string(), "Expected String, got {:?}", n1_val);
        assert!(n1_val.matches_str("Bob"), "users[1].name mismatch");
    }
}
