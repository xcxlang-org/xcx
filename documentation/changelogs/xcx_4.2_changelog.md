# XCX 4.2 — Changelog

Porównanie względem 4.1, na podstawie analizy diffów źródeł.

> **Uwaga redakcyjna:** wpisy dotyczące porządkowania kodu w plikach HIR (`hir/*`), które same zostały dodane dopiero w 4.2, są opisane w sekcji HIR, a nie w „Naprawiono" — to nie są poprawki regresji z 4.1, tylko dokończenie nowej funkcjonalności. Do „Naprawiono" trafiły wyłącznie zmiany zachowania istniejącego w 4.1.

---

## Nowa warstwa kompilacji: HIR

- **Wprowadzono pośrednią reprezentację HIR** (`src/hir/*`, 11 nowych plików: `hir.rs`, `lower.rs`, `lower_expr.rs`, `lower_stmt.rs`, `pass.rs`, `inline.rs`, `inline_policy.rs`, `compile_hir.rs`, `compile_expr.rs`, `compile_expr_special.rs`, `mod.rs`). AST funkcji top-level jest teraz najpierw lowerowane do HIR, a dopiero potem kompilowane do bytecode (`compiler.rs` wywołuje `hir::lower_program` + `hir::compile_hir_to_chunk`).
- **Dodano inlining funkcji na poziomie HIR** (`run_inliner_pass`). Polityka inliningu (`should_inline`) odrzuca: fibery, funkcje rekurencyjne, funkcje z `return` w pętli, głębokość zagnieżdżenia ≥ 3 oraz funkcje o koszcie ≥ 20. `Compiler` ma nowe pole `disable_inline` (flaga globalna, ustawiana przed kompilacją programu, nie per-`CompileContext`). `FunctionCompiler` ma nowe pola `inline_stack`, `inline_result_locals`, `local_regs`.
- **Zaimplementowano brakującą kompilację elementów HIR** (`src/hir/compile_expr.rs`) — dodano pełną obsługę generowania kodu dla `TableLiteral`, `DatabaseLiteral`, `DateLiteral` oraz `Tuple`, co eliminuje domyślne inicjalizacje do Int (z wartością 0) będące przyczyną błędów dispatchu metod.
- **Refaktoryzacja `loop_stack`** — zamiast nietypowanej 4-krotki wprowadzono strukturę `LoopFrame` (`start_pc`, `breaks`, `continues`, `fiber_reg`), spójnie w kompilatorze AST (`compile_control.rs`) i HIR (`compile_hir.rs`).
- **Usunięto ostrzeżenie o nieosiągalnym wzorcu** w dopasowaniach `HirExprKind` w `compile_expr.rs` po pełnym pokryciu wszystkich wariantów.
- **Sprzątanie w nowo dodanym kodzie HIR** (`hir/inline_policy.rs`, `hir/pass.rs`, `hir/lower_stmt.rs`): usunięto nieużywane funkcje `has_return_nested` i `is_stmt_return_anywhere`, nieużywaną zmienną `span` oraz nadmiarowe dopasowanie `_ => unreachable!(...)` po pełnym pokryciu wariantów `StmtKind`. *(Dotyczy kodu wprowadzonego w tej samej wersji, nie regresji.)*


## JIT — nowe optymalizacje

- **Bounds-check fast path dla tablic** (`jit/compiler_method.rs`, `jit/emit_call.rs`) — dodano szybką ścieżkę sprawdzającą `index < len` bezpośrednio w JIT-owanym kodzie, z fallbackiem do wywołania runtime'owego poza zakresem. Obejmuje zarówno `bool[]`, jak i `int[]` — dla opcode'ów `GetIndex`/`SetIndex` oraz dla `MethodCall` Get/Set na receiverze typu `Array` ze znanym typem `Int`.
- **Dedykowany fast path dla tablic BoolArray** (`jit/compiler_method.rs`, `jit/emit_call.rs`) — wdrożono bezpośredni inlining odczytów i zapisów do jednobajtowego bufora tablic boolowskich (`Vec<u8>`) spakowanych w `RwLock` przy użyciu stałego layoutu pamięci na Windows x64 (wskaźnik do danych pod offsetem 16, długość pod offsetem 24). Eliminuje to narzuty FFI i `RwLock` dla poprawnie zaadresowanych dostępów.
- **Poprawa inferencji typów dla tablic BoolArray** (`jit/type_inference.rs`) — dodano rozpoznawanie `is_bool_array()` w fazie JIT type inference przy stałych (`LoadConst`), dzięki czemu tablice typu `array:b` są poprawnie typowane jako `TypeTag::BoolArray` zamiast `TypeTag::Unknown`, co wcześniej blokowało włączenie jakichkolwiek fast pathów i zmuszało JIT do spowolnionego dynamicznego dispatchowania.
- **Śledzenie stałych w rejestrach** (`register_const` w `codegen_ctx.rs`, `emit_load_store.rs`, `jit.rs`) — JIT zapamiętuje znane wartości stałe (np. przy `Move`/`LoadConst`), co odblokowuje dalsze optymalizacje arytmetyczne. `clear_block_state` realnie czyści teraz ten stan (wcześniej było to no-op).
- **Dzielenie/modulo przez stałą potęgę 2** (`emit_arith.rs`, korzysta z powyższego śledzenia stałych) — dla znanego dzielnika 2ⁿ (w tym 2³²) operacja jest emitowana bez guardów na dzielenie przez zero/overflow i bez wywołania runtime'owego.
- **Typowany `JumpIfFalse`** (`emit_control.rs`) — gdy typ rejestru jest statycznie znany jako `Bool`, porównanie sprowadza się do prostego `icmp` na bitach zamiast pełnego porównania tag+bits.
- **Uproszczono limit rekursji JIT**: stała zmieniona z `RECURSION_LIMIT`/`UnsignedGreaterThanOrEqual` na `800`/`SignedGreaterThanOrEqual`, logika wejścia/wyjścia z głębokości rekursji uproszczona, a obsługa przekroczenia limitu wydzielona do nowej funkcji pomocniczej `xcx_jit_check_recursion` (zamiast sztywnego kodu błędu inline).
- **Reload globalnych zmiennych po wywołaniu funkcji** (`emit_call.rs`) odbywa się teraz warunkowo — tylko gdy `callee_uses_heap` — zamiast bezwarunkowo.
- Refaktoryzacja emisji pętli (`method_compiler.rs`) — `IncVarLoopNext`, `ArrayLoopNext`, `TableIter` wywołują dedykowane funkcje `*_opcode` zamiast budować argumenty inline za każdym razem.
- `sync_max_locals()` wywoływane po każdej aktualizacji `next_local` przy kompilacji wywołań funkcji i pętli `for` (`compile_expr/call.rs`, `compile_fiber.rs`, `compile_stmt.rs`) — poprawia śledzenie maksymalnej liczby rejestrów lokalnych.

## JSON — cache i szybkie ścieżki dostępu

- **Cache parsowanych stringów** (`json/parse.rs`) — wynik `json.parse()` jest cache'owany w `thread_local` HashMap (do 128 wpisów, czyszczona po przekroczeniu limitu) zamiast parsowany od nowa przy każdym wywołaniu.
- **Fast path dla prostych kluczy** (`json/mod.rs`, `json_ffi.rs`, `vm/core/step/module.rs`) — dostęp po prostym kluczu (bez `.`, `[`, `]`, `/`) trafia bezpośrednio w strukturę obiektu/tablicy z pominięciem generycznego `json_pointer`. Nowe w 4.2: getter (`MethodKind::Get`), `has()`, getter w `MemberAccess`, przejście na `data_ptr()` w `keys()/len()`. Fast path dla setterów istniał już w 4.1 — w 4.2 doprecyzowano warunek `is_simple` (pełne skanowanie bajtów zamiast `!starts_with('/') && !contains(...)`) i zmieniono atomik flagi `dirty` z `Release` na `Relaxed`.
- **Fast path dla indeksu całkowitego w `MethodKind::Get`** (`json/mod.rs`) — gdy argument `get()` jest `Value::Int` i receiver jest tablicą JSON, dostęp odbywa się bezpośrednio przez wskaźnik z pominięciem konwersji int→string→parse.
- **Bezpośrednie przeszukiwanie klucza w `JsonBindLocal`** (`vm/core/step/module.rs`) — dla prostej nazwy klucza (bez `.`, `[`, `]`) na obiekcie JSON `bind()` bezpośrednio skanuje pole `data_ptr()` obiektu zamiast przechodzić przez pełną ścieżkę `normalize_json_path` → `Vec::collect` → `pointer`.
- **Bezpośrednie przeszukiwanie klucza w `get_path_value_xcx`** (`vm/utils/path.rs`) — dla JSON objectów i tablic w środku ścieżki usunięto `format!("/{}", part)` + wywołanie `pointer()`, zastępując je bezpośrednim `data_ptr()` scan dla objectów i `parse::<usize>()` dla tablic.
- **Pre-alokacja kluczy kolumn w `TableObj::to_json`** (`vm/object/table_obj.rs`) — nazwy kolumn są zamieniane na `Arc<String>` raz przed pętlą po wierszach (`Vec<Arc<String>>`) zamiast `Arc::new(col.name.clone())` per wiersz. Dla 1000 wierszy × 3 kolumny redukuje to 3000 alokacji Arc do 3. Dodano też `Vec::with_capacity(columns.len())` dla wektora obiektu per wiersz.
- **Wątkowo-bezpieczne wersjonowanie cache** (`json/obj.rs`, `heap_object.rs`, `json/mod.rs`, `json_ffi.rs`, `vm/core/step/module.rs`) — pojedynczą flagę `dirty: AtomicBool` zastąpiono parą liczników `version`/`cached_version` (`AtomicU64`) z barierami `Acquire/Release`, co eliminuje wyścigi logiczne i ryzyko udostępnienia nieaktualnego/uszkodzonego stringa JSON przy wielowątkowym dostępie (JIT + VM). Sprawdzanie trafienia w cache pozostaje lock-free.
- `as_str_borrow` (`value.rs`) korzysta teraz z `from_utf8_unchecked` zamiast bezpiecznej walidacji UTF-8, zakładając poprawność danych wejściowych.

## Konkatenacja stringów: `StrAppendVar` / `StrAppendLocal` / `StrAppendMember` / `StrAppendElement`

**Problem:** wzorzec `var = var + expr` (globalny i lokalny), `obj.field = obj.field + expr` (pole JSON) oraz `arr.update(i, arr.get(i) + expr)` (element tablicy stringów) generował w każdej iteracji pętli trzy alokacje (odczyt/rozkopiowanie obu operandów + alokacja wyniku). Dla 100k iteracji dawało to kilkaset tysięcy alokacji/dealokacji i czas rzędu 75–3700 ms.

**Zmiany:**
- Dodano `StringObj::try_extend_bytes(arc, suffix)` — bezpieczne rozszerzenie bufora bez kopiowania, gdy `Arc` ma unikalną własność.
- Nowe opcode'y `OpCode::StrAppendVar { var_idx, src }`, `OpCode::StrAppendLocal { local_idx, src }`, `OpCode::StrAppendMember { container, name_idx, src }` oraz `OpCode::StrAppendElement { container, index, src }` w interpreterze (`vm/core/step/mod.rs`). Warunek unikalnej własności (`Arc::strong_count <= 1`) pozwala na mutację in-place; w przeciwnym razie następuje bezpieczny fallback do pełnego klonowania (COW).
- Kompilator AST i HIR (`compile_stmt.rs`, `hir/compile_hir.rs`) rozpoznaje optymalny wzorzec i emituje odpowiedni opcode. Dodano również spłaszczanie łańcuchów konkatenacji w czasie kompilacji (Option A) — zagnieżdżone wyrażenia dodawania z lewostronną rekurencją (np. `res = res + "a" + "b"`) są automatycznie wykrywane i spłaszczane do sekwencji instrukcji `StrAppendLocal` / `StrAppendVar` (zarówno na poziomie AST, jak i HIR), o ile zmienna docelowa nie jest używana po prawej stronie.
- JIT: FFI helpery z tą samą logiką COW/in-place, zintegrowane w Cranelift z poprawnym śledzeniem liveness i reloadem po powrocie z helpera.

**Wyniki (100k iteracji):**

| Tryb / Scenariusz | Przed | Po |
|---|---|---|
| Global `--no-jit` | ~1300 ms | 3.5 ms |
| Global JIT | ~1300 ms | 2.2 ms |
| Local `--no-jit` | ~1300 ms | 3.6 ms |
| Local JIT | ~1300 ms | 2.5 ms |
| JSON Member `--no-jit` | ~1681 ms | 6.0 ms |
| JSON Member JIT | ~3737 ms | 3.2 ms |
| Array Element `--no-jit` | 86.1 ms | 4.18 ms |
| Array Element JIT | 75.0 ms | 2.96 ms |
| General Str Append `--no-jit` | 322 ms | 7.1 ms |
| General Str Append JIT | 10790 ms | 5.5 ms |

`cargo test --release`: 160/160 testów przechodzi, w tym nowy test COW dla pól struktur oraz `test_general_string_append_cow`.

## VM / Executor — wydajność

- **Dynamiczny rozmiar stosu** (`executor.rs`) — 64K wartości (1 MB) w trybie `--no-jit` dla lepszej lokalności cache, 512K wartości (8 MB) przy aktywnym JIT dla bezpieczeństwa głębokiej rekurencji.
- **Wskaźnik `globals_raw`** — surowy wskaźnik na zmienne globalne ustawiany raz przy inicjalizacji `Executor`, zamiast odczytu z `vm.globals.read()` przy każdym dostępie (`executor.rs`, `jit_helpers.rs`).
- Inicjalizacja lokalnych zmiennych funkcji uproszczona do jednej pętli zamiast kopiowania sliceów.
- Scalono `handle_call` i `handle_call_no_jit` w jedną wspólną `handle_call_inner`, eliminując powielony kod przygotowania/sprzątania ramek wywołania.

## Zapytania na tabelach (`table.*`)

- **Optymalizacja `table.join` (Hash-Join)** (`src/vm/utils/table.rs`) — wprowadzono algorytm Hash-Join o złożoności O(N+M) dla key-based joins (wyszukiwania kluczy z wykorzystaniem nowej struktury pomocniczej `HashableValue`). Czas wykonania benchmarku `join` (500x500 wierszy) spadł z 215 ms do 10 ms (ok. 21-krotne przyspieszenie).
- **Optymalizacja `table.where` (Cache na wiersze)** (`src/vm/core/executor.rs` & `src/runtime/builtin/table/select.rs`) — dodano `row_cache` do struktury `Executor`, która przechowuje już zaalokowane obiekty `RowObj` powiązane z wierszami tabeli. Zapobiega to wykonywaniu 500 000 alokacji `Arc` przy 1000 iteracji i 500 wierszach. Pętla mutacji tabel (`insert`, `delete`, `update`, `clear`) unieważnia ten cache automatycznie, by zapobiec odczytowi nieaktualnych danych.
- `table.count()` / `.len()` / `.size()` (`table/select.rs`) — gdy tabela ma aktywne `sql_binding` i `sql_where`, liczba wierszy jest liczona zapytaniem `SELECT COUNT(*) ... WHERE ...` bezpośrednio w bazie, zamiast liczenia załadowanych wierszy w pamięci.

## Sieć — HTTP

- **Pooling połączeń TCP/TLS** (`src/runtime/builtin/net/client.rs`) — dotychczasowe wywołania `ureq::get`, `ureq::post` itd. korzystały z tymczasowych agentów jednorazowych (nowe uzgodnienie TCP/TLS przy każdym żądaniu HTTPS). Zastąpiono globalnym singletonem `HTTP_AGENT: OnceLock<ureq::Agent>`, inicjalizowanym przy pierwszym żądaniu i utrzymującym pulę połączeń. Dotyczy wszystkich ścieżek: `xcx_jit_net_call`, `call` (interpreter), `xcx_jit_net_request`, `request` (interpreter builder). Walidacja SSRF i timeouty per-request bez zmian. Pomiar: 100 sekwencyjnych żądań HTTPS do tego samego hosta — 195 ms/żądanie → 63 ms/żądanie.

## CLI

- Uporządkowano wyjście `--help` na sekcje: `Usage`, `Options`, `Execution`, `Dev tools`.
- Dodano skrócone flagi `-h` (pomoc) i `-v` (wersja).
- Parsowanie flag pomocy/wersji przeniesiono na sam początek działania programu — można je podać w dowolnym miejscu argumentów (np. `xcx --bytecode -h`, `xcx --version`).
- Dodano obsługę łączenia opcji kompilatora operatorem `|` (np. `--no-jit | --bytecode`), z automatycznym dzieleniem i przetwarzaniem w parserze CLI.
- Dodano link do `xcxlang.com` i adres `contact@xcxlang.com` na dole ekranu pomocy.

## Refaktoryzacja i dług techniczny

- Usunięto powieloną implementację `std::ops::Neg` dla `Value`, pozostawiając wyłącznie metodę `.neg()`.
- Dodano `debug_assert!` sprawdzający rozmiar globalnego wektora zmiennych w `Executor::new`, zapobiegając nieoczekiwanym realokacjom naruszającym bezpieczeństwo wskaźnika `globals_raw`.
- Wyekstrahowano logikę skoków wstecznych i zapytań o rejestry źródłowe/docelowe do metod `jump_target()`, `dst_reg()`, `src_regs()` na `OpCode` — usunięto zduplikowane mapowania w `liveness.rs` i zduplikowany `impl OpCode` w `type_inference.rs`.
- Usunięto `make_map_pair` i scalono powielone przebiegi inicjalizacji `BUILT_INS` w `compiler.rs` do jednej pętli.
- Zastąpiono ogólne dopasowania `_ => TypeTag::Unknown` szczegółowym mapowaniem wszystkich wariantów `Type` (kompilator AST i HIR).
- Udokumentowano celowe pominięcie walidacji `is_const` w `compile_decl.rs` (weryfikacja leży po stronie analizy semantycznej) oraz zachowanie pustych dopasowań `_ => {}` w dispatcherach AST (`pass.rs`, `inline_policy.rs`, `globals.rs`, `compile_stmt.rs`).
- Dodano logowanie błędów kompilacji JIT pod `debug_assertions` oraz zliczanie błędów JIT w globalnym `error_count` VM (`executor.rs`).
- Przeniesiono wszystkie stałe `TAG_*` określające typy wartości wykonawczych z `nan_boxing.rs` bezpośrednio do `tag.rs` (przy zachowaniu kompatybilności wstecznej eksportów z `nan_boxing` dla kompilatora JIT i reszty systemu). Usunięto przestarzałe/mylące komentarze dotyczące NaN-boxingu z `nan_boxing.rs`.

## Naprawiono

- **Zawieszanie się/błędny natywny kod przy pętlach `@step` bez `LoopNext`** (`vm/trace/recording_helper.rs`) — gołe wsteczne skoki bez `LoopNext` wcześniej po prostu zatrzymywały nagrywanie traceu; teraz adres startowy jest jawnie dodawany do blacklisty hotspotów, więc JIT nie próbuje emitować natywnej pętli bez warunku wyjścia.
- **`JsonPush` na obiekcie JSON (zamiast tablicy) powodował panic** — teraz zamiast crasha zwiększany jest licznik błędów (`increment_error_count`) i zwracane jest `0`.
- **`table.where(...)` nie propagował dodatkowych argumentów** jako *captures* do funkcji filtrującej — teraz `args[1..]` jest poprawnie przekazywane.
- **Błędne rzutowanie bitowe w operacjach zmiennoprzecinkowych typów mieszanych** (`Float + Int` i pochodne) — rozwiązano rozbieżność sumy kontrolnej `scoreSum` w benchmarku JSON, spowodowaną błędnym bitcastem `TAG_INT` na bity `F64`:
  - JIT (`compiler_method.rs`, `compiler_fiber.rs`): szybka ścieżka instrukcji float emitowana teraz tylko gdy **oba** operandy są statycznie typu float (`&&` zamiast `||`).
  - VM (`value.rs`): `Value::{add, sub, mul, div, rem, pow}` używają teraz `.cast_float()` zamiast surowego odczytu bitów `.as_f64()` dla typów mieszanych.
- **Brak instrukcji wyboru bloku w GetIndex dla BoolArray** — naprawiono błąd SSA w kompilatorze JIT polegający na braku wywołania `switch_to_block(fast_blk)` w pasie `GetIndex` w `compiler_method.rs`, co powodowało niekompletne generowanie kodu Cranelift.
- **Błąd dispatchu i alokacji w `table.where(...)`** — naprawiono alokację rejestru domknięcia (closure) i captures (zarówno w kompilatorze HIR, jak i AST), gwarantując że są one poprawnie umieszczane w sąsiadujących rejestrach od `base + 1`. Eliminuje to błędy dispatcherów wykonawczych w JIT oraz interpreterze.
- **Usunięto ostrzeżenie kompilatora w `path.rs`** o nieużywanej zmiennej `tag`.
- Literówka w nazwie zmiennej `exec_ptr`/`exec_ptr ` w `compiler_fiber.rs` (kosmetyczne wyrównanie, brak wpływu funkcjonalnego).

## Uwaga o domyślnym zachowaniu (bez zmiany funkcjonalnej)

- `disable_jit` pozostaje domyślnie `false` — JIT jest włączony domyślnie zarówno w CLI, jak i w trybie embedded/biblioteki, uruchamiając się automatycznie po przekroczeniu progu hotspotów. Host embedujący może jawnie wyłączyć JIT (`--no-jit` / odpowiednik w API), ale nie musi tego robić dla pełnej wydajności. *(Brak zmiany względem 4.1 — wymieniono dla jasności, bo temat pojawiał się w kontekście innych zmian JIT.)*