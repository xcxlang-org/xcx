use std::io::{self, Write};
use crate::frontend::lexer::Lexer;
use crate::frontend::parser::Parser;
use crate::sema::{Checker, SymbolTable};
use crate::compiler::Compiler;
use crate::vm::VM;
use std::sync::Arc;

pub struct Repl {
    vm: Arc<VM>,
    symbols: SymbolTable<'static>,
    compiler: Compiler,
    interner: crate::intern::Interner,
}

impl Repl {
    pub fn new(disable_jit: bool) -> Self {
        let mut vm = VM::new();
        vm.disable_jit = disable_jit;
        Self {
            vm: Arc::new(vm),
            symbols: SymbolTable::new(),
            compiler: Compiler::new(),
            interner: crate::intern::Interner::new(),
        }
    }

    pub fn run(&mut self) {
        println!("XCX Interactive Mode (REPL)");
        println!("Type code directly (use arrows to navigate). Type '!exec' on a new line to execute.");
        println!("Type '!help' for assistance or '.terminal !exit;' to quit.");

        let reader = crate::repl::input::InputReader::new();
        loop {
            let input_str = match reader.read_input() {
                Some(s) => s,
                None => break,
            };

            let trimmed = input_str.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('!') {
                if self.handle_command(trimmed) {
                    break;
                }
                continue;
            }

            self.execute(trimmed);
        }
    }

    fn handle_command(&self, command: &str) -> bool {
        match command {
            "!help" => {
                self.print_help();
                false
            }
            "!exit" => {
                println!("Goodbye!");
                true
            }
            "!clear" => {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush().unwrap();
                false
            }
            _ => {
                println!("Unknown REPL command: {}. Type '!help' for available commands.", command);
                false
            }
        }
    }

    fn execute(&mut self, input: &str) {
        let scanner = Lexer::new(input);
        let mut parser = Parser::new_with_interner(input, scanner, self.interner.clone());
        let mut program = parser.parse_program();
        self.interner = parser.into_interner();

        let mut checker = Checker::new(&self.interner);
        let errors = checker.check(&mut program, &mut self.symbols);

        if !errors.is_empty() {
            for err in errors {
                println!("Error: {}", err.kind.to_diagnostic_message());
            }
            return;
        }

        let (main_chunk, constants, functions) = self.compiler.compile(&program, &mut self.interner);
        
        let ctx = crate::vm::core::vm::SharedContext {
            constants,
            functions,
            http_req: None,
        };
        self.vm.clone().run(Arc::new(main_chunk), ctx, &[]);
    }

    fn print_help(&self) {
        print!("{}", r#"
================================================================================
                                XCX HELP SYSTEM
================================================================================

REPL COMMANDS:
  !exec          Execute the current multi-line buffer
  !help          Show this help message
  !clear         Clear the terminal screen
  !exit          Exit the interactive mode

BASIC SYNTAX:
  type: name = value;       Declare a variable (e.g., i: age = 25;)
  const type: NAME = value; Declare a constant
  >! expression;            Print result to terminal (e.g., >! 2 + 2;)
  >? variable;              Wait for user input

DATA TYPES:
  i: Integer (64-bit)       f: Float (64-bit)
  s: String (UTF-8)         b: Boolean (true/false)
  date: Date (YYYY-MM-DD)   json: JSON Object
  array:T { ... }           set:D  { ... }
  map:K<->V { ... }         table: { columns=[...] rows=[...] }

STARTUP FLAGS:
  xcx --no-jit              Disable the JIT compiler for debugging

BUILT-IN SERVICES:
  json.parse(s)             Parse string to JSON
  date.now()                Get current date
  date("2024-01-01")        Create date literal
  store.read(path)          Read file content
  net.get(url)              Perform HTTP GET request

ARITHMETIC & LOGIC:
  +, -, *, /, %, ^, ++      Operators
  ==, !=, >, <, >=, <=      Comparisons
  AND, OR, NOT, HAS         Logical operators

CONTROL FLOW:
  if (cond) then; ... end;
  while (cond) do; ... end;
  for i in start to end do; ... end;

HALT SYSTEM:
  halt.alert >! msg;        Warning (non-fatal)
  halt.error >! msg;        Logic error (stops frame)
  halt.fatal >! msg;        Critical error (terminates process)

CONTACT & SUPPORT:
  Email:    contact@xcxlang.com
  GitHub:   https://github.com/xcxlang-org/xcx
  Website:  https://xcxlang.com

Type any valid XCX statement followed by a semicolon to execute it.
================================================================================
"#);
    }
}
