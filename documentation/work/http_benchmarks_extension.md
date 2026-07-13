# Rozszerzenie benchmarków HTTP o POST oraz Multi-Endpoint

Dodano benchmarki POST oraz benchmarki korzystające z wielu różnych endpointów (bez pełnego reuse tego samego adresu URL) dla 8 języków w folderze `Benchmarks/http_suite/`.

## Zmiany w plikach benchmarków

Stworzono pliki `post.*` oraz `multi.*` w następujących ścieżkach:
- `Benchmarks/http_suite/xcx/` (`post.xcx`, `multi.xcx`)
- `Benchmarks/http_suite/node/` (`post.js`, `multi.js`)
- `Benchmarks/http_suite/php/` (`post.php`, `multi.php`)
- `Benchmarks/http_suite/cpython/` (`post.py`, `multi.py`)
- `Benchmarks/http_suite/ruby/` (`post.rb`, `multi.rb`)
- `Benchmarks/http_suite/perl/` (`post.pl`, `multi.pl`)
- `Benchmarks/http_suite/lua/` (`post.lua`, `multi.lua`)
- `Benchmarks/http_suite/luajit/` (`post.lua`, `multi.lua`)

### Poprawki poprawności w XCX
1. **`post.xcx`**: Zgodnie z dokumentacją `json_http.md`, metoda `net.post(url, body)` przyjmuje typ `json` jako body, a nie string. Zastąpiono string literałem JSON: `json: post_body <<< {"title": "bench", "body": "test", "userId": 1} >>>;` przekazywanym do wywołania.
2. **`multi.xcx`**: Zmienna `s: url` została zadeklarowana na poziomie zewnętrznym pętli, aby uniknąć błędu kompilacji `RedefinedVariable` wywoływanego przez ponowną deklarację w pętli warmup i pętli pomiarowej.

---

## Wyniki pomiarów

### HTTP POST Benchmark (5 warmup + 50 pomiarów na ten sam endpoint)
- **XCX**: Avg ms/req: **143.62 ms** (Czas łączny: 7.19s, 417.2 Req/min)
- **Node.js**: Avg ms/req: **147.62 ms** (Czas łączny: 7.39s, 406.0 Req/min)

### HTTP Multi-Endpoint Benchmark (5 warmup + 50 pomiarów na `/posts/1` - `/posts/50`)
- **XCX**: Avg ms/req: **67.10 ms** (Czas łączny: 3.36s, 892.1 Req/min)
- **Node.js**: Avg ms/req: **249.20 ms** (Czas łączny: 12.47s, 240.6 Req/min)

---

## Analiza różnic wydajności (Multi-Endpoint)

Zauważalne narzuty w Node.js (249 ms na żądanie vs 67 ms w XCX) wynikają ze sposobu zarządzania połączeniami HTTP w bibliotece `undici` (silniku `fetch` w Node.js 18+):
1. **Zarządzanie pulą połączeń**: `undici` domyślnie tworzy osobne połączenia do obsługi równoległych/potokowych zapytań lub agresywniej zamyka połączenia TCP/TLS Keep-Alive przy zmianie ścieżek URL, gdy nie wykrywa ponownego użycia dokładnie tego samego zasobu. W efekcie Node.js co kilka żądań wykonywał pełne uzgodnienie TCP/TLS (skoki do ~315 ms).
2. **Sekwencyjny Agent w Rust (`ureq`)**: Nasz singleton `HTTP_AGENT` w Rust wykonuje żądania ściśle sekwencyjnie. `ureq::Agent` perfekcyjnie zachowuje aktywny socket TCP/TLS dla tego samego hosta niezależnie od odpytywanej ścieżki i nie zamyka go potokowo, omijając narzut handshake TLS i omijając limity IP/throttling hosta jsonplaceholder.
