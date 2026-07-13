# Refactoring: Quick Wins (QW-1 – QW-5)

Implementacja pięciu drobnych usprawnień w architekturze kompilatora i VM, mających na celu redukcję długu technicznego bez wprowadzania regressions.

## Opis zmian

### QW-1 & QW-2: Unifikacja analizy skoków wstecznych oraz zapytania o rejestry na opcode'ach
- **Co zmieniono:** Wyekstrahowano wspólną funkcję `collect_backedges` oraz helper methods (`dst_reg`, `src_regs`, `is_unconditional_jump`, `is_return`, `is_halt`) bezpośrednio do `impl OpCode` w pliku `opcode.rs`. Ponadto plik `type_inference.rs` został oczyszczony z duplikatów `impl OpCode`.
- **Dlaczego:** Kod w `liveness.rs` i `opcode.rs`/`type_inference.rs` powielał logikę rozróżniania typów instrukcji i pobierania ich rejestrów. Przeniesienie do definicji `OpCode` zapobiega rozjeżdżaniu się logiki przy dodawaniu nowych opcode'ów.
- **Pliki:**
  - `src/vm/opcode/opcode.rs`
  - `src/compiler/liveness.rs`
  - `src/jit/type_inference.rs`

### QW-3: Usunięcie redundancji w implementacji `Neg`
- **Co zmieniono:** Usunięto nieużywany trait `impl std::ops::Neg for Value`.
- **Dlaczego:** Typ `Value` posiadał dwie tożsame implementacje negacji: trait `Neg` oraz metodę `neg()`. Metoda `neg()` została zachowana jako jedyny punkt wejścia.
- **Pliki:**
  - `src/vm/value/value.rs`

### QW-4: Integracja handle_call i handle_call_no_jit
- **Co zmieniono:** Metody `handle_call` i `handle_call_no_jit` zostały scalone za pomocą wspólnej prywatnej metody pomocniczej `handle_call_inner` w `executor.rs`.
- **Dlaczego:** Obie metody współdzieliły niemal identyczną logikę obsługi ramek, limitu rekursji i JIT, a różniły się jedynie obecnością `ActiveVmGuard` oraz sposobem pobierania JIT pointera. Zmniejszono narzut utrzymaniowy za pomocą wspólnej funkcji bazowej.
- **Pliki:**
  - `src/vm/core/executor.rs`

### QW-5: Dodanie debug_assert! na stabilność globals_raw
- **Co zmieniono:** Wprowadzono `debug_assert_eq!` sprawdzający rozmiar globalnego wektora zmiennych na poziomie inicjalizacji `Executor::new`.
- **Dlaczego:** `globals_raw` polega na braku realokacji wektora dla zachowania bezpieczeństwa operacji na wskaźnikach. Zapewnia to natychmiastowe wykrycie błędu w trybie deweloperskim w razie modyfikacji rozmiaru `globals`.
- **Pliki:**
  - `src/vm/core/executor.rs`

---

## Wyniki testów i benchmarków

Wszystkie unit testy (159/159) przechodzą poprawnie.

Wyniki wydajnościowe przed i po refaktoryzacji:

| Kategoria | Test / Benchmark | Wydajność (Przed / Baseline) | Wydajność (Po refaktoryzacji) | Status |
|---|---|---|---|---|
| Main | Loop(100m lcg) | 85.76 ms | 86.80 ms | OK |
| Main | Fib(30) | 12.38 ms | 12.53 ms | OK |
| Main | Sieve | 2.23 ms | 2.46 ms | OK |
| Main | JSON | 20.86 ms | 21.03 ms | OK |
| Loops | bench_01_for_nested.xcx | 215.00 ms | 219.00 ms | OK |
| Loops | bench_02_for_step.xcx | 27.00 ms | 28.00 ms | OK |
| Loops | bench_03_while.xcx | 215.50 ms | 226.00 ms | OK |
| Loops | bench_04_while_countdown.xcx | 215.00 ms | 221.50 ms | OK |
| Loops | bench_05_for_array.xcx | 5.00 ms | 5.00 ms | OK |
| Loops | bench_06_for_set.xcx | 5.00 ms | 5.00 ms | OK |
| Loops | bench_07_for_break.xcx | 434.50 ms | 441.50 ms | OK |
| Loops | bench_08_for_continue.xcx | 636.00 ms | 651.00 ms | OK |
| Loops | bench_09_for_arith.xcx | 2.00 ms | 2.00 ms | OK |
| Loops | bench_10_lcg.xcx | 1051.50 ms | 1070.00 ms | OK |
| Func | fib_self | 136.65 ms | 139.50 ms | OK |
| Func | cross_func | 12.52 ms | 12.72 ms | OK |
| Func | inline_arith | 0.43 ms | 0.43 ms | OK |
