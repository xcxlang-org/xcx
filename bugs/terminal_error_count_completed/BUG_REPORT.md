# Bug Report: Terminal Operations inside Functions increment VM Error Count

## Summary
Calling `terminal.write(...)` (and related terminal operations) repeatedly inside a function (`func`) causes the VM's internal error counter (`error_count`) to increment on every call, even though the script executes visually without halting. At the end of execution, the VM observes `error_count > 0` and exits with an error status (`[XCX] Process failed with N errors.`).

---

## Environment
- **XCX Version:** 4.2.0
- **File:** `reproduce.xcx`

---

## Reproduction Script (`reproduce.xcx`)

```xcx
func test_repro() {
    for i in 0 to 50 do;
        terminal.write("a");
    end;
};

test_repro();
>! "";
```

---

## Steps to Reproduce

1. Open a terminal in the project directory.
2. Run the reproduction script:
   ```powershell
   xcx "D:\xcx\bugs\terminal_error_count\reproduce.xcx"
   ```

---

## Observed Behavior

The script outputs the characters, but upon completion, the runtime exits with exit code `1` and prints:
```text
[XCX] Process failed with 102 errors.
```

- Each iteration of `terminal.write` inside the function increments the internal `error_count` by 2.
- Running the exact same `terminal.write` calls at global scope (outside a `func`) does not produce this error count inflation.

---

## Expected Behavior

The script should complete successfully with exit code `0` and without reporting any errors when valid terminal write commands are executed inside functions.

---

## Diagnostic Notes for Investigation

- **Symptom:** The error counter increments silently without halting execution or raising a user-visible runtime panic during the loop.
- **Scope dependency:** The issue occurs specifically when terminal operations are wrapped inside a function body (`func ...`).
- **Impact:** Any long-running script, CLI tool, or ASCII animation that updates the terminal via functions eventually fails with a large error count on process exit.
