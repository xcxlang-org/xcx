# XCX REPL and Interactive Execution Shell

The XCX compiler contains an interactive Read-Eval-Print Loop (REPL) engine located in `src/repl/`. The REPL provides an incremental execution shell that parses, type-checks, JIT-compiles, and runs XCX statements interactively while preserving variables, constants, type states, and string symbols across evaluations.

---

## Interactive Loop Architecture (`src/repl/repl.rs`)

The entry point of the interactive workspace is the `Repl` struct:

```rust
pub struct Repl {
    vm: Arc<VM>,
    symbols: SymbolTable<'static>,
    compiler: Compiler,
    interner: crate::intern::Interner,
}
```

### State Persistence
Unlike a standard file-based compilation pass that discards metadata after emitting assembly or bytecode, the `Repl` retains the following state to enable incremental interaction:
1. **VM Instance (`vm: Arc<VM>`):** Retains globally allocated data, active JIT compilation databases, and environment contexts.
2. **Symbol Table (`symbols: SymbolTable<'static>`):** Preserves types, signatures, and addresses of previously declared variables, functions, and fibers.
3. **String Interner (`interner: Interner`):** Maintains unique string identifiers (`StringId`) across statements. Each input evaluation extracts the updated interner state from the parser context.

### Execution Loop (`Repl::run`)
The execution loop manages command input and evaluation cycles:
1. Prompts or reads input via `InputReader`.
2. Inspects input prefixes. If starting with `!`, delegates to command handling.
3. If structural code, invokes the compiler pipeline:
   - **Lexing & Parsing:** Initializes `Lexer` and `Parser` (transferring the current REPL `Interner`).
   - **Semantic Checking:** Checks AST nodes using `Checker::new(&self.interner)` against the persistent `SymbolTable`.
   - **Compilation:** Compiles AST nodes into a `main` execution chunk, updating JIT structures and static constants.
   - **VM Execution:** Dispatches the main bytecode chunk to `self.vm.run` inside a temporary execution context.

---

## Multi-Line Input Editor (`src/repl/input.rs`)

To support comfortable multi-line coding blocks directly within the interactive shell, the REPL features a raw-mode terminal editor backed by `crossterm`.

### Interactive Features
- **Arrow Keys Navigation:** Users can freely navigate the cursor up, down, left, and right within their multi-line input block to modify previous lines before executing.
- **Line Navigation Shortcuts:** `Ctrl+A` jumps to the beginning of the current interactive line, and `Ctrl+E` jumps to the end.
- **Tab Key Indentation:** Pressing `Tab` automatically inserts exactly 4 spaces (instead of a literal tab character) to maintain readable indent alignment.
- **Prompt Transitions:** The standard `xcx> ` prompt initiates the primary buffer, while `...  ` prefixes subsequent lines upon pressing Enter.

### Executing Code
Input gathering allows developers to arbitrarily draft their scripts across lines. To dispatch the written block of code to the compiler, developers use the interactive `!exec` command:
1. Press `<ENTER>` to open a fresh line.
2. Type `!exec` and press `<ENTER>`.
The shell captures the entire visual block, parses it, executes the code, and clears the input buffer for the next cycle. Any direct input command starting with `!` evaluates immediately.

---

## Interactive Commands

Interactive commands bypass semantic validation and are handled directly in the parser loop:

| Command | Action | Implementation |
|---|---|---|
| `!exec` | Executes the currently typed multi-line code buffer block. | Intercepted in `input.rs` read loop to trigger code processing. |
| `!help` | Renders a syntax guide including datatypes, operations, built-ins, and errors. | Prints help menu strings directly to standard out. |
| `!clear` | Clears all scrollback content and returns input cursor to terminal origin. | Renders ANSI code sequence `\x1B[2J\x1B[1;1H`. |
| `!exit` | Breaks execution loops and halts interactive runtime immediately. | Halts command reading iteration, returning exit status. |
| `!globals` | Inspects global variable state. | Lists the names, types, and values of all registered variables in global scope. |
| `!jit` | Displays JIT compiler information and state. | Inspects compilation thresholds, JIT metadata, and active trace definitions. |
| `!reset` | Clears interpreter state. | Resets persistent interpreter bindings, clearing variables and type definitions. |

---

## Module Placeholders

To optimize terminal features in future updates of the XCX environment, the REPL references structural components with standard constructors:
- **`Completer` (`src/repl/completer.rs`):** Placeholder setup for tab-autocomplete hooks.
- **`Highlighter` (`src/repl/highlighter.rs`):** Hook module for syntax highlighting.
- **`History` (`src/repl/history.rs`):** Placeholder setup for persistent interactive command line logs.
