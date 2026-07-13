# Package Build Scripts and SQLite DB Cleanup Fixes

## 1. Description and Rationale
1. **SQLite DB Cleanup Fixes in `stability_suite`**:
   - The SQLite database files (`.db`, `.db-journal`, `.db-wal`, `.db-shm`) generated during integration/stability tests were left in the workspace on Windows/Unix if they were located in subdirectories or in `temp_dir`.
   - Introduced recursion via `collect_db_files` and cleanup helpers `cleanup_db_files` in `tests/xcx_runner.rs`'s `stability_suite` module.
   - Cleared the cleanup platform guards so Windows is also cleaned up.
   - Ensured cleanup runs both at start and end of `run_xcx_stability_suite` across both `project_root` and `temp_dir`.
2. **Package Build Scripts Path Updates**:
   - The recent packaging restructuring renamed `xcx-installer-pkg` to `Windows`.
   - The MacOS and Linux pkg build scripts (`MacOS/build_macOS_pkg.sh` and `Linux/build_linux_pkg.sh`) were still attempting to copy `mathlib`, `pax` and other resources from `$PROJECT_ROOT/xcx-installer-pkg/`.
   - Corrected all references in both script files to copy resources from the updated `$PROJECT_ROOT/Windows/` path.

## 2. Modified Files
- `tests/xcx_runner.rs` (`stability_suite` module cleanup logic)
- `MacOS/build_macOS_pkg.sh` (copy commands paths)
- `Linux/build_linux_pkg.sh` (copy commands paths)

## 3. Verification Results
- All unit and integration tests successfully pass locally via `cargo test --release`.
- Distribution build scripts correctly find and copy libraries and other resources from the restructured layout.
