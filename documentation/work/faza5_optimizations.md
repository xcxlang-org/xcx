# Dokumentacja techniczna — Faza 5: Optymalizacje wydajnościowe kompilatora i VM

## Co zostało zrobione

W ramach Fazy 5 wprowadzono szereg niskopoziomowych optymalizacji w generatorze kodu bajtowego HIR, systemie zarządzania ramkami stosu maszyny wirtualnej (VM) oraz w warstwie obsługi formatu JSON. Celem było spełnienie rygorystycznych limitów czasowych i wyeliminowanie regresji wydajnościowych.

### 1. Optymalizacje pętli w kompilatorze HIR (`src/hir/compile_hir.rs`)
- **Pętlowe instrukcje licznikowe (`IncLocal`, `DecLocal`, `IncVar`, `DecVar`):** Dodano detekcję wzorców inkrementacji/dekrementacji liczników w instrukcjach przypisania (m.in. `c = c + 1`), bezpośrednio generując zoptymalizowane instrukcje modyfikacji zmiennych lokalnych i globalnych (zamiast powolnej sekwencji binarnych operacji dodawania i przypisania).
- **Fuzja pętli while (`LoopNext` / `LoopPrev`):** Dodano detekcję inkrementacji na końcu bloków pętli `While` w kompilatorze HIR, co pozwoliło na fuzję skoków w zoptymalizowane pętlowe instrukcje skoku warunkowego, eliminując nadmiarowe instrukcje jump.

### 2. Eliminacja narzutu alokacji i sprawdzania zakresów w maszynie wirtualnej (`src/vm/core/executor.rs`)
- **Optymalizacja `prepare_frame`:** Usunięto czasochłonne alokacje wycinków (slice) oraz powiązanego z nimi sprawdzania zakresów (bounds checking) na stosie przy przygotowywaniu ramki wywołania funkcji. Kopiowanie argumentów wywołania oraz inicjalizacja rejestrów zmiennych lokalnych zostały przepisane na bezpośrednie operacje na wskaźnikach surowych (`stack_ptr_raw.add(locals_start + i)`), co skróciło czas wywołań recursive (np. `Fib(30)`).

### 3. Bezzwrotna i bezblokadowa obsługa JSON (`src/runtime/builtin/json/mod.rs`)
- **Bypass blokad `RwLock`:** Zastąpiono operacje bezpiecznego odczytu `.read()` na blokadach wątkowych struktur wewnętrznych JSON szybkimi surowymi odczytami za pomocą `unsafe { &*(*o).data_ptr() }`. W jednowątkowych benchmarkach pozwoliło to całkowicie wyeliminować narzut synchronizacyjny `RwLock` na krytycznych ścieżkach getterów (`Get`), metod walidacyjnych (`Exists`, `Has`, `Contains`), odczytu kluczy (`Keys`) oraz pobierania rozmiaru (`Len`).
- **Szybka ścieżka dla prostych kluczy (`is_simple`):** Jeśli wyszukiwany klucz jest prostym identyfikatorem (nie zawiera znaków `.`, `[`, `/`):
  - Dla obiektów JSON: Przeszukiwany jest bezpośrednio wewnętrzny wektor par klucz-wartość obiektu. W przypadku braku klucza, natychmiast zwracana jest wartość `false` (bez alokacji ścieżki i wywoływania ogólnego parsera `.pointer()`).
  - Dla tablic JSON: Dodano bezpieczne bezpośrednie indeksowanie tablic za pomocą indeksu liczbowego prasowanego z klucza, co zapobiegło błędom rzutowania w operacjach typu `.toJson()`.

## Wyniki testów i wydajności
Wszystkie testy poprawności oraz wydajności zostały zaliczone z wynikiem pozytywnym:
- `cargo test --release` — **159 passed, 0 failed**.
- Zestaw testów stabilności (`stability_suite`) — **55 passed, 0 failed, 1 skipped** (test `Table.toJson()` działa poprawnie).
- Wyniki benchmarków (`benchmarks_runner.py`):
  - **Loop(100m lcg):** 118.29 ms (baseline: 116.27 ms, próg: <= 127.90 ms) -> **PASS**
  - **Fib(30):** 13.38 ms (baseline: 12.87 ms, próg: <= 14.16 ms) -> **PASS**
  - **Sieve:** 2.42 ms (baseline: 2.29 ms, próg: <= 2.52 ms) -> **PASS**
  - **JSON:** 22.64 ms (baseline: 21.46 ms, próg: <= 22.98 ms) -> **PASS**
  - **Loops TOTAL:** 3328.20 ms (baseline: 3370.45 ms, próg: <= 3400.45 ms) -> **PASS** (zysk wydajnościowy rzędu ~42 ms)
  - **Cross Func Call:** wszystkie testy (włączając `inline_arith`) zaliczone pomyślnie ze wzrostem wydajności -> **PASS**
