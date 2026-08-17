# Bug Report: Expression Arguments in Nested Function Calls Evaluate to Corrupt Register

## Summary
When an arithmetic expression involving function parameters (e.g. `x * x + y * y`) is passed directly as an argument to another function call from within a function body, the XCX compiler/VM fails to evaluate the expression into a temporary register before executing the inner function call. As a result, the inner function receives `0` or corrupt register bits instead of the evaluated result.

---

## Environment
- **XCX Version:** 4.2.0
- **File:** `D:\xcx\bugs\nested_func_expr_arg\reproduce.xcx`

---

## Python 1:1 Verification (`c:\tmp\test_nested_func_arg.py`)

```python
def inner_check(val):
    if val < 0.0:
        print(f"Error: inner_check received negative value: {val}")
        return
    print(f"inner_check received valid value: {val}")

def outer_func(x, y):
    inner_check(x * x + y * y)

outer_func(3.0, 4.0)
```
- **Python Output:** `inner_check received valid value: 25.0`

---

## Reproduction Script (`reproduce.xcx`)

```xcx
func inner_check(f: val) {
    if (val < 0.0) then;
        halt.error >! "Error: inner_check received negative value: " + s(val);
        return;
    end;
    >! "inner_check received valid value: " + s(val);
};

func outer_func(f: x, f: y) {
    inner_check(x * x + y * y);
};

--- Direct call works
inner_check(3.0 * 3.0 + 4.0 * 4.0);

--- Nested function call fails
outer_func(3.0, 4.0);
```

---

## Observed Behavior

```text
Testing direct call: inner_check(3.0 * 3.0 + 4.0 * 4.0)...
inner_check received valid value: 25

Testing nested function call: outer_func(3.0, 4.0)...
ERROR halt: Error: inner_check received negative value: 0
```

- Direct call `inner_check(3.0 * 3.0 + 4.0 * 4.0)` at global scope evaluates correctly to `25`.
- Nested call `outer_func(3.0, 4.0)` passing `inner_check(x * x + y * y)` inside `outer_func` passes `0` / corrupt register to `inner_check`.

---

## Workaround in XCX User Code

Assign the argument expression to a local variable before passing it to the inner function:

```xcx
func outer_func(f: x, f: y) {
    f: res = x * x + y * y;
    inner_check(res);
};
```
