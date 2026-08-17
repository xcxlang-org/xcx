# Bug Report: `crypto.token(len)` Output Length Mismatch

## Summary
The system test benchmark script `D:\xcx\programs\sys_modules_test.xcx` fails during execution because `crypto.token(16)` returns a 16-character hex string, whereas `sys_modules_test.xcx` expects a 32-character string (assuming 16 was the byte count).

---

## Environment
- **XCX Version:** 4.2.0
- **File:** `D:\xcx\programs\sys_modules_test.xcx`

---

## Reproduction Script (`reproduce.xcx`)

```xcx
s: tok16 = crypto.token(16);
i: len = tok16.length;
>! "crypto.token(16) length = " + s(len);
```

---

## Steps to Reproduce

1. Run the reproducer script:
   ```powershell
   xcx "D:\xcx\bugs\crypto_token_length\reproduce.xcx"
   ```
2. Run `sys_modules_test.xcx`:
   ```powershell
   xcx "D:\xcx\programs\sys_modules_test.xcx"
   ```

---

## Observed Behavior

Running `sys_modules_test.xcx` outputs:
```text
Crypto Token (16 bytes hex): 215e97ae70be35fb (len=16)

--- VERIFICATION ---
[FAIL] System module verification failed! b64=true tok=false store=true perf=true
```

`crypto.token(16)` produces a string with `length = 16` characters.

---

## Description of Specification Mismatch

`documentation/language/library_modules.md` states:
> `crypto.token(length)` | Generates random hex token of given length

This creates an inconsistency between `sys_modules_test.xcx` (which expects 32 hex chars for parameter 16) and the actual XCX VM implementation (which returns 16 hex chars).
