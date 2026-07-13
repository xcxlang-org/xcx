# Dokumentacja techniczna — Faza 3: HIR Inliner Pass

## Co zostało zrobione
W ramach Fazy 3 zaimplementowano optymalizację inlinowania na poziomie High-level Intermediate Representation (HIR).

Dodano i zaimplementowano następujące pliki:
1. `src/hir/inline_policy.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\hir\inline_policy.rs`:
   - Decyduje, czy wywołanie danej funkcji (callee) wewnątrz funkcji wywołującej (caller) powinno zostać rozwinięte inline.
   - Wprowadzono ograniczenia:
     - Wykluczenie funkcji ze znacznikiem `is_fiber = true`.
     - Wykluczenie rekurencji (sprawdzanie bezpośredniej i pośredniej rekurencji w ciele callee poprzez przeszukiwanie AST-like struktur).
     - Maksymalna głębokość inlinowania `depth < 3`.
     - Limit kosztu (logiczna liczba instrukcji w callee) `< 20`.
     - Wykluczenie wczesnych powrotów (`Return`) za pomocą `is_stmt_return_anywhere`, dopuszczając jedynie pojedynczą instrukcję powrotu na samym końcu ciała funkcji.
2. `src/hir/inline.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\hir\inline.rs`:
   - Klonowanie wyrażeń (`clone_expr`) i instrukcji (`clone_stmt`) z offsetowaniem rejestrów zmiennych lokalnych `HirLocal`.
   - Poprawiono obsługę wariantów `FiberDecl` w mapowaniu instrukcji.
   - Implementacja `inline_call_site`, która generuje sekwencję przypisań argumentów na wstrzykniętych rejestrach parametrów, a następnie klonuje instrukcje z przesunięciem slotów rejestrów i podmienia `Return(expr)` na przypisanie do rezultatu bloku inlinowanego (`result_local`).
3. `src/hir/pass.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\hir\pass.rs`:
   - Główny zarządca optymalizacji `run_inliner_pass`. Iteracyjnie wyszukuje wywołania funkcji do inlinowania, aplikuje transformację i dołącza przesunięte zmienne lokalne do listy rejestrów `caller.locals`.

Zmodyfikowano również `src/hir/mod.rs` w celu wyeksportowania modułów oraz samej metody pass `run_inliner_pass`.

Całość została skompilowana i pomyślnie zweryfikowana:
- `cargo build --release` zwraca status powodzenia 0 (brak błędów i ostrzeżeń).
- `cargo test --release` przeszedł pomyślnie z wynikiem 159 poprawnych testów integracyjnych.

## Dlaczego tak zaprojektowano
Wprowadzenie transformacji wewnątrz struktur `InlineBlock` upraszcza kompilację do końcowego kodu bajtowego (JIT/interpreter). Re-indeksacja rejestrów za pomocą offsetowania gwarantuje, że rejestry lokalne i parametry wbudowywanych funkcji nie wejdą w konflikt z istniejącymi rejestrami w funkcji-wywołującej.
Wykluczenie wczesnych powrotów na tym etapie zapewnia 100% poprawności semantycznej bez skomplikowanego przepływu sterowania (skoków warunkowych) wewnątrz bloku inlinowanego.
