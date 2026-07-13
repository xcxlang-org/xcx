# Faza 16 — Poprawka sumy kontrolnej scoreSum (JSON Benchmark)

## Co zostało zmienione (What was changed)
- Zmieniono typowanie operacji zmiennoprzecinkowych w kompilatorze JIT (`src/jit/compiler_method.rs` oraz `src/jit/compiler_fiber.rs`). Szybka ścieżka instrukcji zmiennoprzecinkowych (np. `emit_add_float`) jest od teraz generowana wyłącznie wtedy, gdy oba operandy są statycznie oznaczone jako `Float` (warunek `&&` zamiast `||`).
- Poprawiono metody operacji arytmetycznych w strukturze `Value` (`src/vm/value/value.rs` — metody `add`, `sub`, `mul`, `div`, `rem`, `pow`). Przy operacjach o typach mieszanych (np. `Float + Int`), gdzie jeden z operandów jest floatem, wartości są rzutowane przy użyciu `.cast_float()` zamiast surowego odczytu bitów rzutowania `.as_f64()`. 
- Oczyszczono tymczasowy plik diagnostyczny `test_output/test_mod.xcx`.

## Dlaczego (Why)
- Poprzednio JIT generował szybką ścieżkę zmiennoprzecinkową nawet gdy jeden z rejestrów miał typ `Unknown` (np. wartości zbindowane z JSON-a powracające z FFI). Powodowało to rzutowanie bitowe zmiennej całkowitej bezpośrednio na liczbę f64 (np. bity liczby int `50` interpretowane jako f64), co niszczyło jej wartość i zwracało wynik bliski `0.0`.
- Ponadto, gdy JIT wycofywał się (fallback) do wykonania w interpreterze maszynerii VM ze względu na typy mieszane, interpreter wykonywał `Value::add(self, rhs)`. Ta metoda generowała ten sam błąd: rzutowała bitowo za pomocą `.as_f64()` obiekt `Int`, zamiast poprawnie go konwertować na typ zmiennoprzecinkowy (co skutkowało sumowaniem wartości bliskich `0`).

## Zmodyfikowane pliki (Which files were modified)
- [compiler_method.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/compiler_method.rs)
- [compiler_fiber.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/compiler_fiber.rs)
- [value.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/vm/value/value.rs)
- [test_mod.xcx](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/test_output/test_mod.xcx)

## Wyniki benchmarku przed i po (Benchmark results before and after)

### Przed zmianami (Before)
- **JSON Benchmark (`main.xcx`)**:
  - `active=250000`
  - `scoreSum=12249999.999999998` (mismatch względem Node.js `12497500.00`)

### Po zmianach (After)
- **JSON Benchmark (`main.xcx`)**:
  - `active=250000`
  - `scoreSum=12497499.9999999` (dokładne pokrycie różnicy rzędu `247 500` odpowiadającej sumie wartości po poprawnej konwersji typów, co po zaokrągleniu daje żądane `12497500.00`).
- **Wynik testów (`cargo test --release`)**: Wszystkie 159 testów przechodzi pomyślnie.
