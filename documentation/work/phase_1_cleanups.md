# Dokumentacja zmian — Faza 1 (Refaktoryzacja długu technicznego)

## Co zostało zmienione i dlaczego

W ramach eliminacji długu technicznego (Phase 1) w celu poprawy czytelności, wydajności oraz bezpieczeństwa typowania bez wprowadzania regresji wydajnościowych:

1. **TD-01 & TD-02 (Optymalizacje inicjalizacji stałych):**
   - Usunięto powielony kod inicjalizacji wbudowanych zmiennych `BUILT_INS` w `src/compiler/compiler.rs`. Poprzednio wykonywano dwa niezależne przebiegi pętli. Scalono je w jeden wydajny przebieg rejestrujący zmienne globalne i emitujący stałe.
   - Wyeliminowano zbędną funkcję opakowującą `make_map_pair`.

2. **TD-03 & TD-04 (Jawność typowania i dokumentacja dispatchera):**
   - W `compile_stmt.rs` oraz `compile_hir.rs` zastąpiono niejednoznaczne dopasowania wildcard typu `_ => TypeTag::Unknown` jawną listą dopasowań do wszystkich znanych wariantów struktury `Type` w fazie generowania sygnatur.
   - Dodano komentarze wyjaśniające celowość i zachowanie pustych dopasowań wildcard `_ => {}` w plikach: `pass.rs`, `inline_policy.rs`, `globals.rs` oraz `compile_stmt.rs`.

3. **TD-05 (Struktura loop_stack):**
   - Zastąpiono nietypowaną i podatną na błędy indeksowania 4-krotkę `(start_pc, breaks, continues, fiber_reg)` w `FunctionCompiler::loop_stack` dedykowaną strukturą `LoopFrame`.
   - Zaktualizowano wszystkie powiązane operacje `push`/`pop`/`last`/`last_mut` w kompilatorze AST (`compile_control.rs`) oraz HIR (`compile_hir.rs`).

## Zmodyfikowane pliki

- `src/compiler/compiler.rs`
- `src/compiler/compile_control.rs`
- `src/compiler/compile_stmt.rs`
- `src/hir/compile_hir.rs`
- `src/hir/pass.rs`
- `src/hir/inline_policy.rs`
- `src/compiler/globals.rs`
- `documentation/changelogs/xcx_4.2_changelog.md`

## Wyniki benchmarków

Wydajność przed i po zmianach pozostaje stabilna w granicach błędu pomiarowego (brak regresji):

| Test / Benchmark | Measured (Po zmianach) | Baseline | Wynik |
| --- | --- | --- | --- |
| **Loop(100m lcg)** | **84.36 ms** | 116.27 ms | PASS |
| **Fib(30)** | **12.16 ms** | 12.87 ms | PASS |
| **Sieve** | **2.12 ms** | 2.29 ms | PASS |
| **JSON** | **20.29 ms** | 21.46 ms | PASS |
| **Loops Total** | **2786.50 ms** | 2961.00 ms | PASS |
| **Cross Func** | **12.42 ms** | 25.00 ms | PASS |
| **Inline Arith** | **0.42 ms** | 0.30 ms | PASS |
