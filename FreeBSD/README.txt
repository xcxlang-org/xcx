XCX 4.4 - Runtime & Compiler (FreeBSD)

XCX is a backend programming language with a built-in runtime.
This package contains the compiler and basic tooling for FreeBSD.

--------------------------------------------------

INSTALLATION

Note: The installer wizard uses 'bsddialog' (in the FreeBSD base system
since 14.0) or 'whiptail' (pkg install newt) as a fallback.

1. Open a terminal.
2. Navigate to the extracted folder.
3. Run the installer:

   sh install.sh

4. Verify installation:

   xcx --version

--------------------------------------------------

UNINSTALLATION

To uninstall XCX:
1. Run the generated uninstaller script:

   ~/.local/share/xcx/uninstall.sh

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

XCX 4.4 is the current release.

- Suitable for: small tools, experiments, learning
- Not suitable for: production systems

APIs and behavior may change between versions.

--------------------------------------------------

KNOWN NOTES

- Supported platform: FreeBSD 14+ (x86_64 / AArch64)
- W^X enforcement: on systems where the JIT cannot finalize executable
  memory, the VM prints a notice and continues with the interpreter —
  program results are identical, only execution speed differs.
- JIT optimizations apply mainly to loops
- Some features (fibers, database) have known limitations

--------------------------------------------------

DOCUMENTATION

Online:
https://xcxlang.com/docs

--------------------------------------------------

XCX is a personal project focused on runtime design and language architecture.
