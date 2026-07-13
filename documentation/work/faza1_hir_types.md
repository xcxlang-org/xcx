# Dokumentacja techniczna — Faza 1: Typy HIR (High-level Intermediate Representation)

## Co zostało zrobione
W ramach Fazy 1 zdefiniowano reprezentację struktur danych nowej warstwy High-level Intermediate Representation (HIR) dla kompilatora XCX 4.2.

Dodano następujące pliki:
1. `src/hir/hir.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\hir\hir.rs`:
   - `HirLocal`: Alias `u32` reprezentujący unikalny slot zmiennej lokalnej. Rozwiązanie to uniezależnia identyfikację zmiennych od nazw (`StringId`), eliminując kolizje nazw przy inliningu (odpowiedni offset jest dodawany do wszystkich slotów callee).
   - `HirBinOp` oraz `HirUnOp`: Własne enumy operatorów binarnych i unarnych uniezależniające HIR od tokenów lexera.
   - `HirArgument`: Opakowanie dla argumentów pozycyjnych i nazwanych.
   - `HirParam` oraz `HirLocalDef`: Definicje parametrów wejściowych oraz deklarowanych zmiennych lokalnych.
   - `HirExpr` / `HirExprKind`: Zoptymalizowane i uproszczone warianty wyrażeń (np. `Local(HirLocal)` z zakodowanym slotem rejestru zamiast tekstowego `Identifier(StringId)`).
   - `HirStmt` / `HirStmtKind`: Uproszczone warianty instrukcji. Dodano specjalny wariant `InlineBlock` na potrzeby wstawiania ciał funkcji inlined.
   - `HirFunc`: Główna struktura przechowująca sygnaturę, lokalne definicje typów, ciało funkcji oraz flagę `is_fiber`.
2. `src/hir/mod.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\hir\mod.rs`:
   - Moduł eksportujący i udostępniający powyższe typy na poziomie crate.

Zmodyfikowano:
- `src/lib.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\lib.rs` — eksponuje moduł `hir`.

## Dlaczego tak zaprojektowano
Zdecydowano o zastosowaniu `HirLocal` jako `u32` (indeks tablicy) zamiast mapowania po `StringId`. Ułatwi to remapping registerów (lokali) podczas wstawiania kodu inlined za pomocą prostego algorytmu offsetowego: `new_local_idx = callee_local_idx + caller_locals_count`.
Modyfikacja `src/lib.rs` wpięła moduł do drzewa ułatwiając kompilację. Kod kompiluje się czysto w wersji release.
