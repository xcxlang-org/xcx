# XCX 4.2 — Dokumentacja Optymalizacji Modulo 2^32 (Faza 10)

Techniczny opis optymalizacji dzielnika modulo będącego dokładnie $2^{32}$ (4294967296) w ścieżce JIT.

---

## 1. Problem i Narzut `band_imm` w trybie 64-bitowym
W benchmarku LCG (`Loop(100m lcg)`) gorąca pętla wykonuje modulo z dzielnikiem $2^{32}$:
```xcx
x = ((x * 1664525) + 1013904223) % 4294967296;
```
Dla nieujemnej części tego modulo, kompilator JIT generował instrukcję maskowania bitów:
```rust
let fast_val = ctx.b.ins().band_imm(l_bits, 4294967295);
```
W architekturze x86_64 instrukcja `and` z natychmiastowym operandem 32-bitowym podlega rozszerzeniu ze znakiem (sign-extension) do 64-bitów. Ponieważ `4294967295` wykracza poza zakres signed `i32`, kompilator Cranelift musiał za każdym razem emitować:
1. Załadowanie wartości `4294967295` do rejestru tymczasowego (np. `mov $0xFFFFFFFF, %r11d`).
2. Wykonanie operacji `and %r11, %rax`.
Operacja ta generowała zbędne instrukcje i narzut alokacji rejestrów wewnątrz ciasnej pętli.

---

## 2. Rozwiązanie: Zero-arkuszowe rzutowanie do 32-bitów
Ponieważ w ścieżce szybkiej (gdy wejście $\ge 0$) modulo $2^{32}$ jest matematycznie równoważne wyzerowaniu górnych 32-bitów rejestru 64-bitowego, zastąpiono `band_imm` poprzez parę instrukcji Cranelift `ireduce` (obcięcie do `I32`) oraz `uextend` (rozszerzenie bez znaku do `I64`):

```rust
let fast_val = if divisor == 4294967296 {
    let reduced = ctx.b.ins().ireduce(types::I32, l_bits);
    ctx.b.ins().uextend(types::I64, reduced)
} else {
    ctx.b.ins().band_imm(l_bits, divisor - 1)
};
```

### Sprzętowy ekwiwalent x86_64
Na poziomie assemblera x86_64 para `ireduce(types::I32)` + `uextend(types::I64)` jest translowana na:
```assembly
mov %eax, %eax
```
Każdy zapis do 32-bitowego rejestru na architekturze x86_64 automatycznie zeruje górną połowę rejestru 64-bitowego. Instrukcja ta na nowoczesnych mikroarchitekturach procesorów wykonuje się w **0 cyklach** (dzięki mechanizmowi *Register Renaming* w porcie front-endu), co całkowicie eliminuje narzut instrukcji maskowania i obciążenie portów ALU.

---

## 3. Rezultaty
Wdrożenie optymalizacji przyniosło skrócenie czasów wykonywania benchmarku Loop oraz pełną poprawność testu jednostkowego `cargo test` i testów integracyjnych:

- **Loop (100M lcg):** **114.67 ms** (bezpośredni run w CWD, cel z baseline: `116.27 ms` — **PASS**)
- **Poprawność matematyczna:** Checksum: `3552931968` (100% zgodności)
- **Brak regresji logicznych:** `cargo test` zaliczony pomyślnie.

Modyfikacji uległ plik:
- [src/jit/emit_arith.rs](file:///D:/XCX-WORKSPACE/xcx_compiler_workspace/src/jit/emit_arith.rs)
