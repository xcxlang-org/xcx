# Faza 15: Wdrożenie flag CLI kompilatora (--no-inline, --check, --bytecode)

## Opis zmian

### 1. Wyłączenie inliningu na poziomie HIR (`--no-inline`)
**Plik zmodyfikowany**: [compiler.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/compiler/compiler.rs)
- Dodano pole `pub disable_inline: bool` jako opcję kompilatora.
- Zmodyfikowano metodę `compile()`, by wywołanie `run_inliner_pass` wykonywało się wyłącznie wówczas, gdy `!self.disable_inline`.

### 2. Nowe flagi i logika przepływu kompilacji
**Plik zmodyfikowany**: [main.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/main.rs)
- Dodano parsowanie argumentów CLI:
  - `--no-inline`: zapobiega inlinowaniu funkcji HIR.
  - `--check`: weryfikuje syntaktykę i typy statyczne, po czym natychmiast wychodzi z kodem `0` (sukces) lub `1` (porażka), wypisując przejrzystą diagnostykę.
  - `--bytecode`: drukuje bajtkod i stałe, a następnie wychodzi z kodem `0`.
- Zmodyfikowano sygnaturę i implementację funkcji `run_file` do obsługi nowej logiki.
- Zaktualizowano CLI help screen (`--help`), który opisuje zastosowanie nowych flag.

---

## Wyniki testów i weryfikacji

### 1. Test walidacji semantyki (`--check`)
Pomyślny przypadek testowy na pliku `cross_func_call.xcx`:
```cmd
target\release\xcx-compiler.exe --check Benchmarks\call_dispatch\xcx\cross_func_call.xcx
```
Output:
`[XCX] Syntax and semantic analysis passed successfully.`

Niepomyślny przypadek (błąd składniowy/parsera) na pliku `test_syntax_error.xcx`:
```cmd
target\release\xcx-compiler.exe --check test_syntax_error.xcx
```
Output:
```
ERROR: Expected ';' at the end of the statement.
   1 | invalid code syntax;
               ~~~~
...
[XCX] Syntax and semantic analysis failed due to syntax errors.
```

### 2. Test dumpowania bytecode (`--bytecode`)
- Uruchomienie `--bytecode` na pliku wykazuje wydrukowanie tabeli stałych oraz instrukcji bajtkodu dla wszystkich funkcji (w tym chunk `main`), kończąc działanie bez wywołania wątku VM.

### 3. Test nie-inlinowania (`--no-inline`)
Oryginalnie funkcja `mul()` inlinuje wywołanie `add()`, redukując bytecode do zwykłego `Add`. Z flagą `--no-inline`:
```
=== BYTECODE DUMP FOR FUNCTION 1 (mul) ===
max_locals: 4
0000: Move { dst: 2, src: 0 }
0001: Move { dst: 3, src: 1 }
0002: Call { dst: 2, func_idx: 0, base: 2, arg_count: 2 }  <-- Call wygenerowano poprawnie!
0003: LoadConst { dst: 3, idx: 5 }
0004: Mul { dst: 2, src1: 2, src2: 3 }
```
Wywołanie `Call` zostało poprawnie zachowane przy włączonej opcji `--no-inline`.
