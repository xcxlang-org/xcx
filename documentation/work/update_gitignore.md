# Updating Git Ignore Rules & Platform READMEs — 2026-07-15

## Goal

Add the `/temp/` local directory to git ignore exclusions and generate platform-specific installer `README.txt` manuals for macOS and Linux. Revise platforms documentation to specify new validation notes.

## Changes

### 1. Ignore Configurations
- Excluded the local `/temp/` folder (used for temporary files and sieve dumps) in `.gitignore`.

### 2. Installer READMEs
- Created [README.txt](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/MacOS/README.txt) inside `/MacOS` with macOS installation and troubleshooting instructions.
  - Noted that releases are verified via GitHub Actions and requested user feedback.
- Created [README.txt](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/Linux/README.txt) inside `/Linux` with Linux installation and command reference details.
  - Documented that the target is tested against Ubuntu and Arch Linux.
- Refined both files to remove the "Basic libraries" row since the libraries are built-in language features. Kept a row specifying that "Offline docs" (local language reference and compiler spec folders) are included in the packaging directory `/lib/doc`.

### 3. README.md Platform Expansion
- Explicitly verified and labeled macOS support alongside Windows and Linux inside the root [README.md](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/README.md).
- Updated the download guide to suggest Linux/macOS tarballs in the "Getting Started" section.

### 4. Builders Re-routing
- Modified [build_macOS_pkg.sh](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/MacOS/build_macOS_pkg.sh) to copy macOS `README.txt` to the packaged environment.
- Modified [build_linux_pkg.sh](file:///d:/XCX-WORKSPACE/xcx_compiler_workspace/Linux/build_linux_pkg.sh) to copy Linux `README.txt` to the packaged environment.

## Files Modified

- `.gitignore`
- `README.md`
- `MacOS/README.txt`
- `Linux/README.txt`
- `MacOS/build_macOS_pkg.sh`
- `Linux/build_linux_pkg.sh`

## Benchmark Results

N/A
