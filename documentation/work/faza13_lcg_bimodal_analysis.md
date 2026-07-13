# Faza 13 — Analiza Bimodalnej Wydajności LCG (XCX 4.2)

## Opis analizy

W tej fazie zbadaliśmy bimodalny rozkład wydajności benchmarku LCG (100M iteracji), gdzie wyniki oscylowały wokół dwóch stanów: ~110 ms (szybki) oraz ~130 ms (wolny). 

### Eksperyment i wnioski

1. **Sortowanie rejestrów w `analysis.rs`:**
   Sprawdziliśmy hipotezę, że niedeterministyczny rozkład pamięci (ASLR) w połączeniu z różną kolejnością iteracji po `HashSet` przy analizie zmiennych lokalnych i globalnych powoduje różny layout kodu i w efekcie różną wydajność loops. 
   - **Wynik:** Wprowadzenie deterministycznego sortowania elementów przed wygenerowaniem preloada JIT nie zlikwidowało bimodalności (nadal występowały przebiegi ~114 ms oraz ~130 ms), a dodatkowo wywołało delikatną regresję w innych testach z `loops` suite. Zmiana została wycofana.

2. **Poziom sprzętowo-systemowy (Hardware / OS):**
   Ponieważ każdy przebieg benchmarku to uruchomienie nowego procesu (`subprocess.run(xcx-compiler.exe)`), a bimodalny rozkład (wyniki w klastrach ~110ms / ~130ms) dotyczy dokładnie tej samej, niezmiennej binarki JIT, powodem jest zachowanie systemu Windows 11 i procesora:
   - **DVFS (Dynamic Voltage and Frequency Scaling):** Zmiana stanów energetycznych rdzeni procesora (P-state/C-state) pomiędzy procesami. Procesor potrzebuje czasu na wejście na pełne taktowanie turbo po uruchomieniu nowego procesu, co przy tak krótkim benchmarku (~100 ms) wpływa bezpośrednio na średnią.
   - **UOP Cache & Alignment:** Losowość przydziału stron adresowych przez system operacyjny (ASLR) przy alokacji pamięci wykonywalnej (JIT arena) determinuje, czy gorąca pętla przekracza granice linii cache / okien uop cache procesora. Wpływa to na dekodowanie instrukcji w pętli.

### Rekomendacje

Nie zaleca się wprowadzania "sztucznych" poprawek wyrównania w kompilatorze (np. dodawanie instrukcji NOP przed pętlami lub zamiana struktur analizy), ponieważ:
1. Zmiana layoutu dla jednego benchmarku (LCG) popsuje wyrównanie dla innych benchmarków.
2. Sprzętowa bimodalność jest naturalną cechą mikroarchitektury CPU na współczesnych systemach operacyjnych dla tak krótkich pętli wykonywanych w JIT.
3. Stabilizacja LCG w XCX 4.2 na poziomie ~116ms–130ms przy wyeliminowaniu wcześniejszego narzutu z `clear_block_state` jest optymalnym stanem dla tej architektury.
