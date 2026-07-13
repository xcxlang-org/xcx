# XCX JIT — Stan obecny i braki względem profesjonalnych implementacji

Dokument opisuje co SajaJIT (Cranelift-based tracing JIT w XCX) już posiada oraz czego brakuje w porównaniu do dojrzałych JIT-ów takich jak V8 (Node.js), HotSpot (JVM) czy LuaJIT.

---

## Co JIT ma

- **Cranelift backend** — produkcyjny generator kodu maszynowego (x86-64, AArch64) używany przez wasmtime i rustc. Pełna alokacja rejestrów, peephole optymalizacje.
- **Tracing JIT z type guards** — spekulatywne założenie typów (`GuardInt`, `GuardFloat`, `GuardBool`) i emisja szybkich ścieżek dla operacji int/float bez tagowania wartości.
- **Statyczna analiza typów przed emisją** — `analyze_trace_global_ints`, `analyze_trace_non_ptr_regs` eliminują ref-counting dla rejestrów i globalnych zmiennych które nigdy nie trzymają wskaźników. Skutkuje to brakiem dec_ref/inc_ref na hot path pętli liczbowych.
- **Preload locals/globals** — wartości używane w pętli są ładowane do rejestrów SSA przed wejściem w pętlę, nie per-iterację.
- **Selektywny reload globals po wywołaniu** — przeładowanie `globals_raw` jest emitowane tylko jeśli callee mógł użyć sterty (`callee_uses_heap`).
- **HIR-level function inlining** — inlining funkcji przed emisją bytecodu eliminuje część narzutu wywołania dla małych funkcji.

---

## Czego brakuje

### 1. OSR — On-Stack Replacement *(wysoki priorytet)*

**Problem:** JIT uruchamia się dopiero po N wywołaniach funkcji. Jeśli program ma jedną długą pętlę w `main` (jak benchmark `main.xcx`), JIT nigdy jej nie skompiluje — pętla wykonuje się wyłącznie w interpreterze.

**Co to jest:** Mechanizm wejścia w JIT w trakcie wykonywania pętli, bez konieczności powrotu do callera. Wymaga przeniesienia stanu interpretera (rejestry lokalne, IP) do ramki JIT.

**Efekt braku:** Benchmark `Loop(100M lcg)` w `--no-jit` i w normalnym trybie (zanim pętla zostanie nagrana jako trace): pełna prędkość interpretera ~7s zamiast JIT ~114ms.

---

### 2. Tiered compilation

**Problem:** Jest tylko jeden poziom kompilacji. V8 ma 4 (Ignition → Sparkplug → Maglev → Turbofan).

**Co to jest:** Szybki, nieoptymalizujący JIT (np. template JIT / baseline) uruchamiany wcześnie, który zastępuje interpreter z minimalnym narzutem kompilacji; następnie pełny optymalizujący JIT zastępuje go dla bardzo gorących ścieżek.

**Efekt braku:** Zimny start: pierwsze N wywołań funkcji idzie przez interpreter zamiast choćby lekko skompilowanego kodu.

---

### 3. Inline Caches dla dostępu do właściwości

**Problem:** Każde odwołanie do pola JSON/Map (`obj.field`) w JIT idzie przez `xcx_jit_get_member` — FFI call z hash-lookup za każdym razem.

**Co to jest:** Pamięć podręczna kształtu obiektu (shape/hidden class). Jeśli obiekt ma taki sam układ jak poprzednio, lookup jest bezpośrednim offsetem bez hash-lookup.

**Efekt braku:** Dostęp do pól obiektów JSON/Map w JIT jest wolniejszy niż mógłby być.

---

### 4. Escape Analysis / Stack Allocation

**Problem:** Każdy krótkotrwały obiekt (tablica, mapa, string tymczasowy) alokowany jest na stercie z ref-countingiem.

**Co to jest:** Analiza czy obiekt "ucieka" poza bieżący scope. Jeśli nie — można go alokować na stosie lub wyeliminować w całości.

**Efekt braku:** Programy tworzące tymczasowe kolekcje w pętlach generują nadmiarowe alokacje sterty i presję GC.

---

### 5. Automatyczny inlining w JIT

**Problem:** HIR inlining jest statyczny (przed bytecode). JIT sam nie decyduje o inliningu callees podczas kompilacji gorącej ścieżki.

**Co to jest:** JIT widzi że funkcja jest wołana często w gorącej pętli i inlinuje jej ciało bezpośrednio w emitowanym kodzie natywnym, eliminując narzut wywołania i otwierając dalsze optymalizacje (stała propagacja przez granice funkcji).

---

### 6. Deoptymizacja z rollback stack frames

**Problem:** Gdy guard nie przejdzie (np. zmienna okazuje się float zamiast int), JIT powraca do interpretera przez IP — ale nie ma pełnego mechanizmu rekonstrukcji stanu interpretera z frame'ów natywnych.

**Co to jest:** Możliwość "odwinięcia" skompilowanej ramki z powrotem do stanu interpretera w dowolnym punkcie wykonania (jak HotSpot's uncommon trap). Wymaga map deoptymizacji dla każdej instrukcji.

---

### 7. SIMD / Auto-wektoryzacja

Brak emisji instrukcji wektorowych. Dla pętli na tablicach liczb (np. DSP, ML) jest to istotna strata.

---

## Priorytety dla przyszłych wersji

| Funkcja | Trudność | Wpływ na wydajność | Priorytet |
|---|---|---|---|
| OSR | Wysoka | Krytyczny (top-level pętle) | **1** |
| Inline caches (IC) | Średnia | Wysoki (JSON/Map access) | **2** |
| Tiered JIT (baseline) | Wysoka | Średni (cold start) | **3** |
| Automatyczny JIT inlining | Średnia | Średni | **4** |
| Escape analysis | Bardzo wysoka | Średni | **5** |
| SIMD | Wysoka | Niski (niszowy) | **6** |

---

*Plik techniczny — nie marketing. Stan na XCX 4.2.*
