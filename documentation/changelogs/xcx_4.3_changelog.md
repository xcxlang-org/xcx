# XCX 4.3 - Changelog (4.2 -> 4.3)
**Type: RELEASE**

---

## VM: Stabilność JIT i fallback do interpretera

- **[CHG]** Pole `disable_jit` w strukturze `VM` zmienione z `bool` na `std::sync::atomic::AtomicBool`. Umożliwia bezpieczną modyfikację w trakcie działania VM z kontekstu JIT (bez konieczności blokowania mutexa).
- **[NEW]** Dynamiczne wyłączenie JIT przy pierwszym błędzie kompilacji. Jeżeli `JIT::compile_method` lub `JIT::compile` zwróci błąd (np. `finalize_definitions` zgłosi `EACCES` na systemach z polityką W^X), VM trwale wyłącza JIT dla całej bieżącej sesji (`disable_jit.store(true)`) i kontynuuje w trybie interpretera. Poprzednie zachowanie: VM logował błąd i kontynuował w nieokreślonym stanie.
- **[FIX]** Usunięto inkrementację `error_count` przy błędzie kompilacji JIT w `executor.rs` i `jit_helpers.rs`. Wcześniej błąd kompilacji (który skutkuje poprawnym fallbackiem) był liczony jako błąd wykonania, co mogło nieprawidłowo wpływać na kody wyjścia procesu.

Dotyczy: FreeBSD 14+ (kernel W^X), oraz każdy system, na którym JIT nie może sfinalizować metadanych pamięci wykonywalnej.

---

## VM: SSRF — ujednolicenie zachowania interpretera i JIT

- **[FIX]** Funkcja `call()` w `src/runtime/builtin/net/client.rs` (ścieżka interpretera dla `net.get`) zastępuje ręczne sprawdzenie `url.contains("169.254.")` wywołaniem `is_safe_url(&url)`. Poprzednia implementacja blokowała tylko adresy link-local (`169.254.*`) zamiast pełnego zestawu chronionych zakresów adresowych.
- **[FIX]** Gdy `is_safe_url()` zwróci błąd z prefiksem `HALT.FATAL` lub `HALT.ERROR`, interpreter teraz rzuca `panic!()` z odpowiednią wiadomością. Poprzednie zachowanie: powrót `OpResult::Continue` z mapą błędu — VM kontynuował działanie zamiast przerywać proces.
- **[CHG]** Zachowanie SSRF jest teraz spójne między ścieżką JIT (`xcx_jit_net_call`) i interpretera (`call`): obie kończą proces przy wykryciu niedozwolonego adresu.

---

## JIT: Śledzenie inicjalizacji rejestrów i obsługa wielokrotnych ścieżek wyjścia (Multi-Return)

- **[FIX] Śledzenie `defined_locals` w `CodegenCtx`**: Dodano tablicę `defined_locals: [bool; 256]` w `CodegenCtx`. Rejestry lokalne są oznaczane jako zainicjowane (`true`) wyłącznie przy wczytywaniu w `preload_locals` (jeśli należą do wyznaczonego zbioru `needs_init`), oraz przy bezpośrednim zapisie w `def_local`, `def_local_nanboxed` i `reload_local`.
- **[FIX] Filtrowanie `cleanup_all` i `should_skip_dec_ref`**:
  - `cleanup_all` pomija rejestry, dla których `defined_locals[r]` wynosi `false`. Zapobiega to przypadkowemu wczytywaniu i definiowaniu zmiennych Cranelift w bloku wejściowym (Block 0) dla niezainicjowanych rejestrów podczas przetwarzania wczesnych ścieżek powrotu (`return`).
  - `should_skip_dec_ref` zwraca `true` dla rejestrów z `defined_locals[r] == false`, uniemożliwiając generowanie instrukcji `dec_ref` dla starych/losowych wartości ze stosu przy pierwszym przypisaniu do rejestru.
- **[FIX] Synchronizacja rejestrów po FFI w `reload_local`**: Funkcja `reload_local` po wywołaniu pomocnika `StrAppendLocal` ustawia teraz flagi `mark_used`, `mark_dirty`, `defined_locals[r] = true` oraz `known_types[r] = TypeTag::String`. Poprzednio brak oznaczenia rejestru jako `dirty` powodował, że kolejne operacje `spill_all` nie zapisywały zaktualizowanego wskaźnika i tagu napisu do pamięci VM (`locals_ptr`).
- **[FIX] Adnotacje typów stałych w `emit_load_const`**: Wskaźniki stałych obiektów alokowanych na stercie (w tym ciągów znaków) są teraz rejestrowane w `ctx.known_types[dst] = TypeTag::String` przy wywołaniu `emit_load_const`. Poprzednio `known_types` pozostawało `TypeTag::Unknown`.

---

## I/O & Terminal: Dedykowana obsługa uruchamiania plików `.xcx` (`.terminal !run`)

- **[FIX] Wywoływanie interpretera dla plików `.xcx`**: Dodano funkcję pomocniczą `execute_run(cmd: &str)` w `src/runtime/builtin/io/terminal.rs`. Gdy pierwszy parametr polecenia wskazuje na plik ze rozszerzeniem `.xcx`, wywołanie `.terminal !run` uruchamia bezpośrednio proces bieżącego wykonywalnego kompilatora (`std::env::current_exe()`) przekazując plik i argumenty, zamiast polegać na asocjacjach plików w systemie operacyjnym (co na systemach Windows wywoływało okno "Otwórz za pomocą").
- **[FIX] Obsługa standardowego wyjścia i statusu wyjścia**:
  - `OpCode::TerminalRun` oraz `xcx_jit_terminal_run` (w ścieżka JIT) wypisują przechwycony bufor `stdout` procesu potomnego bezpośrednio na konsolę terminala (`write_buffered` + `flush_buffered`).
  - Funkcje zwracają ciąg wyjściowy `Value::from_string(stdout)` przy sukcesie (lub `Value::from_bool(true)` gdy wyjście jest puste) oraz `Value::from_bool(false)` przy błędzie/niepowodzeniu procesu, umożliwiając prawidłową ewaluację wyrażeń warunkowych (np. `if (NOT .terminal !run target)` w menedżerze pakietów `PAX`).

---

## Crypto: Fix random token length representation

- **[FIX]** Modified `crypto.token(len)` to generate `len * 2` hexadecimal characters rather than `len` characters. The parameter `len` represents the length of the token in bytes, meaning the output hexadecimal string representation is now correctly `2 * len` characters long.

---

## JIT: Poprawa precyzji wnioskowania typów parametrów (Type Inference)

- **[FIX]** Wprowadzono dwukierunkową propagację ograniczeń typów w `infer_param_types`. Poprzednio mechanizm wnioskowania typów przypisywał statyczny typ `TypeTag::Int` do parametrów na podstawie samego wystąpienia operacji arytmetycznych (`Add`, `Sub`, `Mul` itp.) lub porównań (`Less`, `Greater` itp.). Powodowało to błędy (np. w `nested_func_expr_arg`), gdzie wartości zmiennoprzecinkowe przekazywane do zagnieżdżonych funkcji były nieprawidłowo traktowane jako liczby całkowite, co prowadziło do uszkodzenia wartości rejestrów / zwracania `0` w JIT.
- **[REF]** Nowy mechanizm propaguje typy wstecz i w przód, na podstawie typów stałych (`LoadConst`), rzutowań (`Cast*`) i instrukcji specyficznych dla typu (np. operacji pętlowych i inkrementacji). Zabezpiecza to poprawność typowania zmiennych zmiennoprzecinkowych, jednocześnie w pełni zachowując optymalizacje JIT dla operacji na liczbach całkowitych (np. w benchmarku `fib(30)`).

---

## Store: Rekurencyjne usuwanie katalogów (Directory Deletion)

- **[FIX]** Zmodyfikowano funkcję `store.delete()` w ścieżce interpretera (`read_write.rs`) oraz pomocnika JIT `xcx_jit_store_delete` (`jit_helpers.rs`). Poprzednio obie funkcje bezwarunkowo wywoływały `std::fs::remove_file`, co skutkowało błędem i zwróceniem `false` przy próbie usunięcia katalogu. Obecnie sprawdzane jest czy ścieżka to katalog – w takim przypadku usuwana jest rekurencyjnie za pomocą `std::fs::remove_dir_all`, w przeciwnym razie przez `std::fs::remove_file`.

---

## Terminal: Zapobieganie nieprawidłowemu wzrostowi licznika błędów w funkcjach i zasięgu globalnym (Fix `terminal_error_count`)

- **[FIX]** Zmodyfikowano kompilatory AST (`call.rs`) oraz HIR (`compile_expr_special.rs`), dodając obsługę przestrzeni nazw `terminal`. Metody takie jak `.write()`, `.clear()`, `.raw()`, `.normal()`, `.cursor()`, `.move()`, `.exit()`, `.run()` są teraz prawidłowo wykrywane i bezpośrednio kompilowane do ich dedykowanych kodów operacji VM (opcodes), zamiast wywoływania ogólnej metody pomocniczej JIT/VM `xcx_jit_method_call_custom` z odbiornikiem `TAG_STR` ("terminal"), co uprzednio skutkowało błędnym podbiciem wewnętrznego licznika błędów (`error_count`) przy każdym wywołaniu.

---

## Input: Filtrowanie zdarzeń zwolnienia klawiszy w trybie surowym (Fix `double_input_event`)

- **[FIX]** Zmodyfikowano obsługę wejścia w funkcjach `input()`, `read_key()` oraz `wait_key()` w pliku `input.rs` tak, aby odfiltrowywały zdarzenia `KeyEventKind::Release` (czyli puszczenie klawisza) podczas pobierania zdarzeń klawiatury za pomocą biblioteki `crossterm`. Poprzednio brak filtrowania powodował, że jedno fizyczne naciśnięcie klawisza na systemie Windows generowało dwa zdarzenia o tym samym kodzie klawisza (wcisku i puszczenia), co podwójnie wyzwalało rejestrację zdarzenia w maszynie wirtualnej.
