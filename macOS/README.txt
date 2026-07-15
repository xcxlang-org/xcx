XCX 4.2 — Runtime & Compiler (macOS)

XCX is a backend programming language with a built-in runtime.
This package contains the compiler and basic tooling for macOS.

--------------------------------------------------

INSTALLATION

1. Open Terminal.
2. Navigate to the extracted folder.
3. Run the installer:

   ./install.sh

4. Verify installation:

   xcx --version

--------------------------------------------------

USAGE

Run a file:

   xcx file.xcx

Start interactive mode (REPL):

   xcx

Example:

   >! "Hello, world!";

--------------------------------------------------

WHAT IS INCLUDED

- xcx               → compiler + runtime
- PAX               → package manager (preview)
- Offline docs      → language reference & compiler specifications

--------------------------------------------------

PROJECT STATUS

XCX 4.2 is under active development.

- Suitable for: small tools, experiments, learning
- Not suitable for: production systems

APIs and behavior may change between versions.

--------------------------------------------------

KNOWN NOTES

- Supported platform: macOS (11.0+)
- Note: macOS releases are built and verified automatically via GitHub Actions. Due to the lack of dedicated Apple Silicon hardware for local testing, user feedback and bug reports are highly appreciated.
- JIT optimizations apply mainly to loops
- Some features (fibers, database) have known limitations

--------------------------------------------------

DOCUMENTATION

Online:
https://xcxlang.com/docs

--------------------------------------------------

SUPPORT

Issues and bugs:
https://github.com/xcx-lang/xcx-compiler/issues

--------------------------------------------------

XCX is a personal project focused on runtime design and language architecture.
