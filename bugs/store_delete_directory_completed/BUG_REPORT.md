# Bug Report: store.delete Fails on Directories (Specification Mismatch)

## Summary
The specification in `documentation/language/library_modules.md` states:
> `store.delete(p)` -> `Removes file or directory (recursive).`

However, calling `store.delete(p)` on a directory path (empty or non-empty) fails, returns `false`, and leaves the directory intact on the file system.

---

## Environment
- **XCX Version:** 4.2.0
- **File:** `D:\xcx\bugs\store_delete_directory\reproduce.xcx`

---

## Python 1:1 Verification (`c:\tmp\test_store_delete.py`)

```python
import os
import shutil

dir_path = "c:/tmp/temp_delete_test"
os.makedirs(dir_path, exist_ok=True)

shutil.rmtree(dir_path)
print(f"Directory exists: {os.path.exists(dir_path)}")
```
- **Python Output:** `Directory exists: False`

---

## Reproduction Script (`reproduce.xcx`)

```xcx
s: dir_path = "programs/temp_delete_dir_bug";

store.mkdir(dir_path);
b: delete_result = store.delete(dir_path);
b: exists_after  = store.exists(dir_path);

>! "store.delete(dir_path) returned: " + s(delete_result);
>! "Directory exists after delete: "  + s(exists_after);
```

---

## Observed Behavior

```text
Directory created. Exists: true
store.delete(dir_path) returned: false
Directory exists after delete: true
```
