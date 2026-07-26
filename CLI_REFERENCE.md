# XCX Compiler & Runtime — CLI Reference (.md)

Kompleksowa dokumentacja interfejsu wiersza poleceń (**CLI**) oraz powłoki interaktywnej (**REPL**) języka **XCX 4.2**.

---

## 1. Składnia i Podstawowe Użycie (Usage)

Głównym poleceniem wykonywalnym systemu XCX jest `xcx`.

```bash
xcx                     # Uruchamia powłokę interaktywną (REPL)
xcx <plik.xcx>          # Wykonuje podany plik źródłowy XCX
xcx pax                 # Uruchamia wbudowany menedżer pakietów PAX
xcx doc                 # Uruchamia narzędzie interaktywnej dokumentacji offline
```

---

## 2. Flagi i Opcje CLI (Command-Line Flags)

Flagi mogą być przekazywane po nazwie pliku. Dodatkowo XCX wspiera unikalne łączenie opcji za pomocą znaku pipe `|` (np. `--no-jit | --bytecode`).

| Flaga / Opcja | Skrót | Opis |
|---|---|---|
| `--help` | `-h`, `help` | Wyświetla informację o użyciu CLI oraz dostępnych opcjach. |
| `--version` | `-v`, `version` | Wyświetla wersję kompilatora, system operacyjny oraz architekturę (np. `xcx 4.2 (linux/x86_64)`). |
| `--no-jit` | — | Wyłącza kompilator JIT. Kod jest wykonywany wyłącznie przez czysty interpreter bajtokodu. |
| `--no-inline` | — | Wyłącza pass optymalizacyjny inlinowania na poziomie HIR (High-Level IR). |
| `--threshold=N` | `--th=N` | Ustawia próg detekcji hot-spotów (pętli) aktywujących kompilację JIT (domyślnie: `50`). |
| `--check` | — | Tryb sprawdzania ("dry run"). Analizuje składnię i semantykę (typy) pliku bez uruchamiania go w VM. |
| `--bytecode` | — | Generuje i zrzuca na `stdout` pulę stałych oraz bajtokod sekcji głównej i funkcji, po czym kończy działanie. |

### Przykłady wywołania flag CLI:
```bash
# Sprawdzenie poprawności syntaktycznej i typów (bez uruchamiania):
xcx app.xcx --check

# Wyłączenie JIT i zmiana progu pętli:
xcx app.xcx --no-jit --th=10

# Podgląd bajtokodu:
xcx app.xcx --bytecode

# Łączenie flag za pomocą znaku pipe (|):
xcx app.xcx "--no-jit | --bytecode"
```

---

## 3. Powłoka Interaktywna REPL (Interactive Shell)

Po uruchomieniu samego polecenia `xcx` bez argumentu pliku wejściowego, inicjalizowana jest powłoka **REPL** (Read-Eval-Print Loop). Powłoka ta zachowuje stan zmiennych globalnych, tablicę symboli oraz stan JIT pomiędzy wpisywanymi instrukcjami.

### Komendy Specjalne REPL (z wykrzyknikiem `!`)

Wewnątrz sesji REPL polecenia specjalne rozpoczynają się od znaku `!`:

| Polecenie REPL | Opis |
|---|---|
| `!exec` | Wykonuje aktualnie wpisany wielowierszowy bufor kodu. |
| `!help` | Wyświetla wbudowany system pomocy XCX (składnia, typy danych, wbudowane funkcje). |
| `!globals` | Wyświetla tabelę zaktualizowanych zmiennych globalnych, ich typy oraz wartości. |
| `!jit` | Wyświetla diagnostykę kompilatora JIT (próg, liczba wykrytych pętli i skompilowanych ścieżek). |
| `!clear` | Czyści ekran terminala. |
| `!reset` | Resetuje stan maszynki VM, czyści zmienne i resetuje powłokę REPL. |
| `!exit` | Zamyka powłokę REPL i wychodzi z programu (alternatywa: `.terminal !exit;`). |

### Edytor i Nawigacja w REPL
Edytor wielowierszowy REPL oferuje wygodną nawigację terminalową:
- **Strzałki (Góra/Dół/Lewo/Prawo):** Poruszanie się kursorem po bloku kodu przed jego wykonaniem.
- **`Ctrl + A`:** Skok na początek wiersza.
- **`Ctrl + E`:** Skok na koniec wiersza.
- **`Tab`:** Automatyczne wstawienie wcięcia (4 spacje).
- **`Enter`:** Przejście do nowego wiersza (znak zachęty zmienia się z `xcx> ` na `...  `).
- Wykonanie bloku kodu następuje po wpisaniu `!exec` w nowej linii i naciśnięciu `Enter` (lub bezpośrednio po zatwierdzeniu pojedynczej instrukcji kończącej się średnikiem).

---

## 4. Wbudowane Narzędzia (Subcommands)

Kompilator XCX dostarcza 2 wbudowane podnarzędzia uruchamiane bezpośrednio z CLI:

1. **`xcx pax`** – uruchamia menedżer pakietów PAX (`lib/pax/src/pax.xcx`).
2. **`xcx doc`** – uruchamia przeglądarkę/narzędzie offline dokumentacji specyfikacji języka XCX (`lib/doc/doc.xcx`).
