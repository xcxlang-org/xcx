# Faza 14 — Eliminacja bimodalności w pętli LCG (XCX 4.2)

## Opis zmian
Aby wyeliminować bimodalną charakterystykę wydajności benchmarku Loop (LCG 100M iteracji), zmodyfikowaliśmy obsługę operacji Modulo dla dzielnika `4294967296` (2^32) w pliku generatora Cranelift.

- **Pliki zmodyfikowane:** `src/jit/emit_arith.rs`
- **Szczegóły zmiany:** Dla `divisor == 4294967296` usunięto sprawdzanie znaku liczby (`icmp_imm` + `brif` + block dynamic dispatch) i powiązany z nim wolny blok (`slow_blk`) wykonujący instrukcję `srem_imm`. JIT zawsze generuje bezpośrednią sekwencję instrukcji `ireduce (i32) + uextend (i64)`, co sprowadza się do bitowego maskowania dolnych 32 bitów na rejestrach procesora.

## Wyniki wydajności

### Przed optymalizacją (XCX 4.2 regression / bimodal)
- **Loop(100m lcg):** bimodalnie ~116 ms (ścieżka fast z ireduce) lub ~130ms-150ms (ścieżka slow z srem_imm używająca instrukcji idiv), średnio ~128 ms.

### Po optymalizacji
- **Loop(100m lcg):** 85.04 ms (stabilne, bez oscylacji)
- **Fib(30):** 12.24 ms
- **Sieve:** 2.18 ms
- **JSON:** 20.38 ms
- **Checksum:** 3552931968 (poprawny, niezmieniony)

Wszystkie testy regresyjne kompilatora w trybie `--release` przechodzą pomyślnie.
