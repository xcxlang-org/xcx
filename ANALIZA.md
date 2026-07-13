# Analiza narzutu wywołań rekurencyjnych w SajaJIT (XCX)

Porównanie z prostym AOT-owym kompilatorem ("Kala") na przykładzie fib(30)

## 1. Wynik wyjściowy

| Implementacja | Czas fib(30) |
|---|---|
| XCX (SajaJIT, tracing JIT na Cranelift) | 12 148 µs |
| Kala (prosty AOT codegen, jednoprzebiegowy) | 7 578 µs |

XCX jest ~1,6× wolniejszy na tym mikrobenchmarku (rekurencja, ~2.7 mln wywołań `fib`). Pytanie: czy to systemowy narzut architektury SajaJIT na ścieżce wywołań rekurencyjnych.

## 2. Co przeanalizowano

- `jit/abi.rs` — konwencja wywołań funkcji JIT-owanych
- `jit/emit_call.rs` — kodogeneracja dla `Call` (w tym rekurencja)
- `jit/codegen_ctx.rs` — model rejestrów, spillowanie, zmienne lokalne
- `jit/jit.rs` — kompilacja śladu, punkty spillowania
- `jit/builder.rs`, `jit/symbols/mod.rs` — konfiguracja Cranelift, tablica FFI

Dla porównania: `kala/src/codegen.rs` — prosty kompilator AST → x86, bez VM, bez tracingu, bez tagowania wartości.

## 3. Model danych: 16 bajtów zamiast 8

XCX trzyma każdą wartość jako parę `(bits: i64, tag: i64)` — 16 bajtów na slot, nawet dla zwykłego `int`:

```rust
// jit/codegen_ctx.rs
pub struct SlotVars {
    pub bits_var: Variable,
    pub tag_var:  Variable,
}
// VALUE_SIZE = 16 bajtów/slot
```

Kala operuje na gołych `i64` bez tagu. Dla czysto arytmetycznej funkcji jak `fib` to podwaja ruch danych na każdy load/store lokalnej, każdy argument i każdy zwracany wynik.

To architektoniczny koszt tagowania potrzebnego przy dynamicznych typach w runtime — nie błąd. Nie da się tego usunąć bez utraty dynamicznego typowania, ale da się obejść dla wąskich przypadków (patrz pkt 6.1).

## 4. ABI: 7 dodatkowych wskaźników na każdy call

```rust
// jit/abi.rs
pub type JITFunction = unsafe extern "C" fn(
    *mut VMValue,        // out_ptr      — dokąd zapisać wynik
    *mut VMValue,        // locals_ptr   — baza tablicy lokalnych
    *mut VMValue,        // globals_ptr  — baza globali
    *const VMValue,      // consts_ptr   — baza puli stałych
    *mut VM,             // vm_ptr
    *mut Executor,       // exec_ptr
    *const bool,         // shutdown_ptr
) -> i32;
```

Dla `fib(n)`, który potrzebuje tylko jednego `int`, i tak przy każdym z 2,7 mln wywołań idzie 7 dodatkowych wskaźników plus argument jako (bits, tag). W Kali to jeden rejestr z liczbą.

Wynik nie wraca przez rejestr — zapisywany jest pod `out_ptr` i odczytywany stamtąd przez wywołującego:

```rust
// jit/emit_call.rs — wywołanie rekurencyjne
let out_slot = ctx.b.create_sized_stack_slot(...16 bajtów...);
let out_ptr = ctx.b.ins().stack_addr(I64, out_slot, 0);
call_args.push(out_ptr);
...
let inst = ctx.b.ins().call(self_ref, &call_args);
...
let res_bits = ctx.b.ins().load(I64, ..., out_ptr, 0);
let res_tag  = ctx.b.ins().load(I64, ..., out_ptr, 8);
```

Dodatkowy round-trip przez pamięć na każde wywołanie, zamiast zwrotu w rejestrze.

## 5. Recursion guard na każdym poziomie

```rust
// jit/emit_call.rs
let cur_depth = ctx.b.ins().load(I64, ..., ctx.executor_ptr, ctx.call_depth_offset);
let limit = ctx.b.ins().iconst(I64, 800);
let is_overflow = ctx.b.ins().icmp(SignedGreaterThanOrEqual, cur_depth, limit);
ctx.b.ins().brif(is_overflow, overflow_blk, ..., run_blk, ...);
// run_blk:
let new_depth = ctx.b.ins().iadd_imm(cur_depth, 1);
ctx.b.ins().store(..., new_depth, ctx.executor_ptr, ctx.call_depth_offset);
// po powrocie:
ctx.b.ins().store(..., cur_depth, ctx.executor_ptr, ctx.call_depth_offset);
```

Słuszny bezpiecznik (bez niego głęboka rekurencja segfaultowałaby proces), ale to load + porównanie + branch + 2 store'y na każde wywołanie. Kala się tym nie przejmuje — polega na limicie stosu systemowego.

## 6. Spill rejestrów i globali

Sprostowanie wcześniejszej hipotezy: `spill_all()` **nie** zrzuca bezwarunkowo wszystkich 256 rejestrów — iteruje tylko po bitmasce `dirty_registers`:

```rust
// jit/codegen_ctx.rs
pub fn spill_all(&mut self) {
    for i_idx in 0..4usize {
        let mut bits = self.dirty_registers[i_idx];
        while bits != 0 { /* store tylko dirty rejestrów */ }
    }
    self.spill_globals(); // <- to ZAWSZE iteruje wszystkie globalne
}
```

Dwa realne koszty przy rekurencji:
- `spill_globals()` iteruje bezwarunkowo po wszystkich śledzonych globalnych, niezależnie czy są "dirty", przy każdym wywołaniu.
- Dla `fib` argument i wynik pośredni są zwykle "dirty" tuż przed wywołaniem (dopiero co policzone), więc realny narzut zbliża się do pełnego spilla mimo selektywnego mechanizmu.

W `jit.rs` `spill_all()` jest wywoływane przy każdym punkcie wyjścia ze śladu (shutdown check, koniec pętli, fallback) — poprawne dla spójności stanu VM, ale mnoży się z częstotliwością rekurencji.

## 7. reload_globals() po powrocie

```rust
if ctx.uses_heap {
    ctx.reload_globals();
}
```

Warunkowe na `uses_heap` — jeśli funkcja nie dotyka sterty (jak `fib`), tego kosztu nie ma. To jeden z niewielu miejsc, gdzie już jest dobry fast path.

## 8. Zestawienie kosztu jednego wywołania

| Element kosztu | XCX (SajaJIT) | Kala (AOT) |
|---|---|---|
| Rozmiar wartości | 16 B (bits+tag) | 8 B (i64) |
| Argumenty w ABI | 1 wartość + 7 wskaźników | 1 rejestr |
| Zwrot wyniku | przez pamięć (out_ptr) | przez rejestr |
| Recursion guard | load+cmp+branch+2×store | brak (limit systemowy) |
| Spill przed wywołaniem | dirty regs + wszystkie globalne | brak |
| Reload po powrocie | warunkowy (uses_heap) | brak |

## 9. Rekomendacje (wg priorytetu)

**9.1 Fast path dla samo-rekurencyjnych funkcji czysto arytmetycznych** (Int/Float/Bool, `uses_heap = false`) — największy potencjalny zysk. Jeśli analiza typów wykaże, że funkcja operuje wyłącznie na Int/Float/Bool, nie używa sterty i wywołuje samą siebie, można wygenerować tańszą ścieżkę:
- argumenty/wynik jako gołe i64/f64 bez tagu (tag odtwarzany statycznie, bo typ jest znany w compile-time)
- wynik zwracany przez rejestr Cranelift zamiast przez out_ptr
- pominięcie zbędnych wskaźników ABI (np. globals_ptr jeśli funkcja nie czyta globali)

Nie narusza istniejącej, ogólnej ścieżki dla pozostałych przypadków (JSON, HTTP, kolekcje itd.).

**9.2 Tańszy recursion guard** — trzymanie `call_depth` w rejestrze Cranelift zamiast w pamięci Executora dla funkcji jednowątkowych bez fiberów, albo inkrementacja co N wywołań przy udowodnionej głębokości statycznej.

**9.3 Selektywny spill_globals()** — dodanie bitmaski dirty dla globali (analogicznie do lokalnych), żeby pominąć zapis gdy globalne się nie zmieniły.

**9.4 Zmniejszenie ABI per-funkcja** — statyczna analiza mogłaby generować zredukowaną sygnaturę wywołania (bez globals_ptr/consts_ptr/vm_ptr, gdy niepotrzebne) zamiast jednej sztywnej sygnatury o 7 parametrach dla wszystkich przypadków.

## 10. Podsumowanie

Różnica 12 148 µs vs 7 578 µs nie wynika z jednej wady, tylko z sumy drobnych kosztów rozłożonych na 2,7 mln wywołań: podwójny rozmiar wartości, rozbudowane ABI, zwrot wyniku przez pamięć, recursion guard liczony przy każdym wywołaniu, spill rejestrów i globali przed każdym callem. Każdy element jest uzasadniony ogólnością SajaJIT (typy dynamiczne, bezpieczeństwo, JSON/HTTP/kolekcje/fibery), czego Kala w ogóle nie obsługuje. Największy zysk bez naruszania tej ogólności dałby fast path dla funkcji samo-rekurencyjnych, czysto arytmetycznych i bezstanowych (9.1) — atakuje wszystkie wymienione koszty naraz, bez zmiany zachowania dla bardziej złożonych przypadków.