# Dokumentacja techniczna — Faza 2: Lowering AST do HIR

## Co zostało zrobione
W ramach Fazy 2 zaimplementowano proces translacji AST do struktur High-level Intermediate Representation (HIR).

Dodano i zaimplementowano następujące pliki:
1. `src/hir/lower_expr.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\hir\lower_expr.rs`:
   - Translacja `ExprKind` -> `HirExprKind`.
   - Zmienne lokalne (w tym parametry) są prawidłowo identyfikowane w scopes i mapowane na sloty `HirLocal`.
   - Poprawiono mapowania tokenów operatorów w `lower_bin_op` (odpowiednio: `Caret` -> `Pow`, `EqualEqual` -> `Equal`, `Has` -> `Has`, `SymDifference` -> `SetSymDifference`, `PlusPlus` -> `IntConcat`).
   - Usunięto błędy borrow checkera (E0507) poprzez klonowanie `TokenKind`.
2. `src/hir/lower_stmt.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\hir\lower_stmt.rs`:
   - Translacja `StmtKind` -> `HirStmtKind`.
   - Dodano lexical scoping za pomocą struktury pomocniczej `HirFuncBuilder` (w tym `enter_scope`, `exit_scope`, `define_local` i `lookup_local`).
   - Poprawiono dopasowanie typów dla gałęzi `else_ifs` w instrukcji warunkowej (prawidłowe pakowanie warunku w `Box`).
3. `src/hir/lower.rs` pod adresem `d:\XCX-WORKSPACE\xcx_compiler_workspace\src\hir\lower.rs`:
   - Metoda `lower_func` tworząca instancję buildera, mapująca parametry wejściowe na pierwsze rejestry lokalne i spłaszczająca translację instrukcji.
   - Metoda `lower_program` tworząca kompletną mapę zidentyfikowanych w AST definicji funkcji (`HashMap<u32, HirFunc>`).
4. Zaktualizowano `src/hir/mod.rs` w celu eksponowania powyższych punktów wejściowych translacji.

Wszystkie pliki zostały przepisane bez komentarzy i są w pełni budowalne (`cargo build --release` zwraca status powodzenia 0).

## Dlaczego tak zaprojektowano
Struktura mappingu uwzględnia różnicę między zmiennymi lokalnymi a globalnymi. `HirLocal` jako alias `u32` zapewnia płaski, bezpieczny wektor rejestrów na poziomie pojedynczego `HirFunc`, co ułatwi późniejsze wstawianie ciał funkcji oraz re-indeksację.
