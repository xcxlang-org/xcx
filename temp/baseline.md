# XCX 4.2 [JIT] — Wyniki

## Suite 1: LOOP / FIB / SIEVE / JSON
| LOOP (100M) | FIB (30) | SIEVE | JSON |
|---|---|---|---|
| 86.36ms | 12.41ms | 2.23ms | 20.70ms |

## Suite 2: Pętle
| TRIPLE FOR | FOR @STEP | WHILE UP | WHILE CDOWN | FOR ARRAY | FOR SET | FOR BREAK | FOR CONT | ARITH | LCG | TOTAL |
|---|---|---|---|---|---|---|---|---|---|---|
| 218.00ms | 28.00ms | 217.00ms | 216.50ms | 5.00ms | 5.00ms | 435.00ms | 636.50ms | 2.00ms | 1058.00ms | 2821.00ms |

## Suite 3: FIB(35) / CROSS_FUNC / INLINE_ARITH
| FIB(35) | CROSS_FUNC 1M | INLINE_ARITH 1M | GEO MEAN |
|---|---|---|---|
| 135.98ms | 12.55ms | 0.43ms | 9.03ms |