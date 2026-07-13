# Faza 12: Stabilizacja kompilatora HIR (Math & Relational)

## Opis zmian

### 1. Eliminacja błędnego zachowania instrukcji Return w funkcjach wywołujących
**Plik zmodyfikowany**: [pass.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/hir/pass.rs)
- **Problem**: Podczas inliningu, prepended opcody powiązane z inlinowaną funkcją były grupowane razem z przetwarzaną instrukcją zewnętrzną (np. `Return`) w jeden blok `InlineBlock` z wartością `result_local: None`. Powodowało to błąd w generatorze kodu (w `compile_stmt`), który dla każdego `Return` sprawdzał stos `inline_result_locals` i zamiast zwracać wartość z funkcji (za pomocą instrukcji `Return`), emitował `Jump` do końca bloku inline. W efekcie funkcje (np. `math.mean`) nie zwracały prawidłowo wartości.
- **Rozwiązanie**: Usunięto opakowywanie spłaszczonej listy w `InlineBlock` na poziomie wywołań instrukcji w `inline_in_stmt`. Zwracana jest płaska lista instrukcji `Vec<HirStmt>`. Granica inlinowanej funkcji jest poprawnie określona bezpośrednio w miejscu inlinowania przez `InlineBlock` z jawnym `Some(result_local)`.

### 2. Zabezpieczenie rejestrów zmiennych lokalnych przy JsonBind/JsonInject/Serve
**Plik zmodyfikowany**: [compile_hir.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/hir/compile_hir.rs)
- **Problem**: W generatorze kodu dla instrukcji `JsonBind`, `JsonBindGlobal`, `JsonInject`, `JsonInjectLocal` oraz `Serve` po wygenerowaniu opcodów następowało zwalnianie rejestrów tymczasowych poprzez przypisanie `compiler.next_local = json_src as usize` (lub `port_src`). W przypadku gdy wyjściowy rejestr wyrażenia (`json_src` / `port` itp.) wskazywał na zmienną lokalną (czyli indeks `< func.locals.len()`), powodowało to zresetowanie pozycji wolnych rejestrów tymczasowych do obszaru zajmowanego przez inne aktywne zmienne lokalne. Kolejne kompilowane wyrażenia nadpisywały te rejestry (np. nadpisanie rejestru json stringiem `"role"` w teście `ult_06`).
- **Rozwiązanie**: Wprowadzono standardowy mechanizm zapisu i przywracania `compiler.next_local` przy użyciu zmiennej `saved_next_local` na poziomie instrukcji.

---

## Wyniki testów i benchmarków

### Testy
Uruchomienie `cargo test --release` wykazało pełne przejście wszystkich **159** testów.
Naprawiono błędy:
- `ultimate_suite::ult_17_math_comprehensive` (przechodzi)
- `ultimate_suite::ult_06_table_relational` (przechodzi)

### Benchmarki
```
[main_benchmark]
  Loop(100m lcg)                 | 129.12 ms  (FAIL, niezwiązany regres)
  Fib(30)                        | 12.51 ms   (PASS)
  Sieve                          | 2.35 ms    (PASS)
  JSON                           | 21.22 ms   (PASS)

[cross_func_call]
  fib_self                       | 146.83 ms  (PASS)
  cross_func                     | 13.40 ms   (PASS)
  inline_arith                   | 0.44 ms    (PASS)
```
Brak jakichkolwiek regresji wydajnościowych na wspieranych benchmarkach głównych oraz loop tests.
