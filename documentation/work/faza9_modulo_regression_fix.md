# XCX 4.2 — Dokumentacja Analizy Modulo i Regression Revert (Faza 9)

Analiza wpływu usunięcia rozgałęzień (branching) dla dzielników modulo będących potęgami dwójki większymi lub równymi 65536 w kompilatorze JIT.

---

## 1. Wprowadzona zmiana i cel
W celu uproszczenia generowanego kodu w metodzie `emit_poly_div_mod_fast_path` (`src/jit/emit_arith.rs`), podjęto próbę zastąpienia trójblokowej sekwencji rozgałęziającej (sprawdzającej negatywność wejścia w celu optymalizacji maskowaniem `band_imm`) uproszczonym i bezwarunkowym wywołaniem instrukcji Cranelift `srem_imm` dla stałych potęg dwójki $\ge 65536$:

```rust
// Zamiast sekwencji: icmp_imm -> brif -> fast_block (band_imm) / slow_block (srem_imm)
ctx.b.ins().srem_imm(l_bits, divisor)
```

---

## 2. Wyniki i regresja wydajności
Bezpośrednie użycie `srem_imm` wywołało krytyczną regresję czasową w benchmarkach opartych na Linear Congruential Generator (LCG):

| Benchmark / Metryka | Z uproszczeniem (`srem_imm`) | Z rozgałęzieniem (Baseline) | Różnica (Zysk z revertu) |
|---|---|---|---|
| **Loop (100M lcg)** | **218.84 ms** | **130.70 ms** | **-88.14 ms (~40% szybciej)** |
| **bench_10_lcg.xcx** | **1936.50 ms** | **1066.50 ms**| **-870.00 ms (~45% szybciej)** |
| **loops TOTAL** | **3732.00 ms** | **2859.50 ms** | **-872.50 ms (PASS)** |

---

## 3. Przyczyna techniczna (Wnioski)
1. **Koszt reszty z dzielenia ze znakiem (`srem`):** Podpisane modulo dla potęg dwójki z negatywnymi wejściami wymaga wykonania skomplikowanych operacji (lub instrukcji `idiv` na x86_64), aby zachować matematyczną poprawność znaku.
2. **Skuteczność predykcji rozgałęzień:** Dane w pętli LCG są w zdecydowanej większości nieujemne (dodatnie). W efekcie, warunek `is_neg` jest stale fałszywy, a procesor z blisko 100% skutecznością przewiduje skok do ścieżki szybkiej (`fast_blk`).
3. **Optymalny kod ścieżki szybkiej:** Ścieżka szybka wykonuje wyłącznie 1-cyklową operację bitową `band_imm(l_bits, divisor - 1)`. Kod ten całkowicie omija ciężkie dzielenie sprzętowe i nie generuje kosztów z powodu świetnej predykcji rozgałęzień przez procesor.

---

## 4. Podjęta decyzja
Przywrócono oryginalny kod bazujący na rozgałęzieniu (branching fast-path), ze względu na to, iż w środowisku produkcyjnym i obliczeniowym zapewnia on o ok. 40-45% krótszy czas wykonania operacji modulo.

Modyfikacji uległ plik:
- [src/jit/emit_arith.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/emit_arith.rs)
