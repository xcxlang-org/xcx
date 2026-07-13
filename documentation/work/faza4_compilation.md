# Dokumentacja techniczna — Faza 4: HIR Compilation, Integration & Stability Fixes

## Co zostało zrobione
W ramach Fazy 4 zaimplementowano kompilację struktur High-level Intermediate Representation (HIR) do końcowego kodu bajtowego oraz zintegrowano nowy rurociąg kompilatora. W trakcie weryfikacji naprawiono także krytyczne błędy stabilności powiązane z inlinowaniem oraz zapytaniami bazodanowymi.

### 1. Kompilacja i Integracja HIR (`src/hir/compile_hir.rs` & `src/compiler/compiler.rs`):
- **Kompilator HIR (`src/hir/compile_hir.rs`):** Przekłada zoptymalizowaną strukturę `HirFunc` (wraz z zagnieżdżonymi strukturami wyrażeń i instrukcji) na końcowy `Chunk` instrukcji kodu bajtowego (Bytecode). Obsługuje m.in. bloki inlinowane `InlineBlock`, dbając o prawidłowe mapowanie rejestrów i zmiennych lokalnych.
- **Integracja rurociągu (`src/compiler/compiler.rs`):** Zastąpiono tradycyjne bezpośrednie generowanie kodu bajtowego z AST nowym rurociągiem: `AST` -> `HIR Lowering` -> `Inliner Pass` -> `HIR Compilation`.

### 2. Poprawki Stabilności (HALT-003, COL-001, DB-005, COL-003b):
- **JIT Halt Propagation (`src/jit/emit_call.rs`):** Przywrócono strażnika `is_inner_func` w generatorze wywołań JIT, co gwarantuje, że zatrzymania (halts) wewnątrz podfunkcji JIT są poprawnie propagowane, bez błędnego przerywania głównego wątku interpretera.
- **Rozdzielanie parametrów lambd bazodanowych (`src/hir/compile_expr.rs`):**
  - Usunięto wadliwy warunek `slot >= 10000` przy kompilacji `HirExprKind::Local`. Umożliwiło to prawidłowe mapowanie przechwytywanych parametrów w domknięciach (np. zmiennej `t` o indeksie 0 wewnątrz fiberów).
  - Wprowadzono warunkowe kompilowanie zmiennych globalnych pod kątem lambd bazodanowych (`compiler.is_table_lambda`). Jeśli zmienna nie jest zarejestrowana jako lokalna/globalna, a kompilowana jest lambda tabeli, generowana jest instrukcja `MethodCallCustom` na rejestrze 0 (Row object) zamiast `GetVar`. Dzięki temu mechanizm translacji filtrów SQL (`translate_filter_to_sql`) poprawnie mapuje referencje na kolumny sqlite (np. `token` czy `department`).
- **Leniwe Zapytania SQL (`src/runtime/builtin/table/select.rs`):**
  - Przywrócono leniwe wyliczanie `sql_where` w metodzie `.where()`, co zapobiega przedwczesnemu pobieraniu i filtrowaniu danych w pamięci.
  - Zaimplementowano bezpośrednie zapytanie `SELECT COUNT(*)` w metodach `.count()`, `.len()`, `.size()` w przypadku przypisanych filtrów SQL (`sql_where`), co eliminuje niezgodności w raportowaniu liczby wierszy.

## Wyniki testów i wydajności
Wszystkie testy integracyjne w pakiecie kompilatora XCX przeszły pomyślnie:
- `cargo test --release` — **159 passed, 0 failed**.
- Zestaw stabilności (`stability_suite`) — **55 passed, 0 failed, 1 skipped** (COL-003b oraz błędy dzielenia przez zero zostały w pełni naprawione).
- Pomiary wydajnościowe z runnera benchmarków wykazują zerową regresję (drobne odchyłki w testach Sieve czy JSON mieszczą się w granicach błędu pomiarowego i szumu CPU, a inlinowanie przynosi korzyści w testach wywołań).
