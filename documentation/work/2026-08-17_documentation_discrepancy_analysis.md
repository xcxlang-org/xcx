# Analiza niezgodności: `src/` vs `documentation/compiler/` — stan po remediacji 2026-08-17

**Data analizy:** 2026-08-17  
**Autor:** automatyczna inspekcja cross-reference na podstawie plików work/ i src/  
**Zakres:** pełna analiza `src/` vs `documentation/compiler/` po wykonaniu faz 1, 3A, 3B, 3C, 4, 5, 6 (phase 2 pominięta). Każda sekcja wskazuje konkretne pliki dokumentacji i src.

---

## Metodologia

1. Przeczytano wszystkie pliki `documentation/work/2026-08-17_phase*.md` i `*_summary.md`, `*_plan.md`, `*_reverification_pass.md` — lista wszystkich rzeczy usuniętych, zmienionych lub dodanych.
2. Przeczytano cały `src/` fizycznie (listings katalogów + zawartość plików kluczowych).
3. Przeczytano cały `documentation/compiler/` (jit/*, vm/*, compiler/*, repl/, runtime/, sema/, itp.).
4. Porównano: co jest w kodzie → czy dokument to opisuje; co dokument twierdzi → czy to jest w kodzie.

---

## NIEZGODNOŚCI ZNALEZIONE

---

### [DOC-1] `vm_value.md` — brakujące TAG_BOOL_ARR, TAG_FUNC_PTR i BoolArrayObj (KRYTYCZNE)

**Plik dokumentacji:** `documentation/compiler/vm/vm_value.md` — sekcja „Tag Constants"  
**Plik src:** `src/vm/value/tag.rs`, `src/vm/value/value.rs`, `src/vm/object/bool_array_obj.rs`

**Problem:**  
Tabela tagów w `vm_value.md` (linie 45–60) wymienia tylko 14 tagów (TAG_FLOAT=0 … TAG_DB=13) i zatrzymuje się. W kodzie (`tag.rs`, linia 15–16) istnieją dwa dodatkowe tagi:

```rust
pub const TAG_FUNC_PTR: u64 = 16;
pub const TAG_BOOL_ARR: u64 = 17;
```

`TAG_FUNC_PTR` reprezentuje funkcję przenoszoną jako heap-value przez `Arc<FunctionObj>` (inaczej niż `TAG_FUNC`, który przechowuje u32 indeks). `TAG_BOOL_ARR` wskazuje na `Arc<RwLock<BoolArrayObj>>` — wyspecjalizowaną tablicę boolean'ów. Oba są używane w `is_func()`, `is_bool_array()`, `value.rs:variant_rank()`, `value.rs:tag()`.

`Tag` (enum w `tag.rs`) zawiera `BoolArray` i `Unknown` — `vm_value.md` wymienia tylko 14 wariantów bez `BoolArray`.

Metoda `Value::from_bool_array(Arc<RwLock<BoolArrayObj>>)` istnieje w `value.rs` (linia 369), ale nie jest wymieniona w tabeli konstruktorów w `vm_value.md`.

Metoda `Value::from_function_ptr(Arc<FunctionObj>)` istnieje w `value.rs` (linia 377), ale nie jest wymieniona w tabeli konstruktorów.

Metoda `Value::as_bool_array()` istnieje w `value.rs` (linia 393), ale nie ma jej w tabeli akcesorów.

`is_bool_array()` (linia 269 w `value.rs`) nie jest wymienione w tabeli predykatów.

**Plik `src/vm/object/bool_array_obj.rs`** istnieje (562 bajtów) i definiuje `BoolArrayObj` — w `vm_objects.md` nie ma o nim żadnej sekcji.

**Wymagana korekta:** dodać TAG_FUNC_PTR i TAG_BOOL_ARR do tabeli tagów; dodać BoolArrayObj do vm_objects.md; dodać `from_bool_array`, `from_function_ptr`, `as_bool_array`, `is_bool_array` do vm_value.md.

---

### [DOC-2] `vm_value.md` — nadal dokumentuje usunięte funkcje wcx_eq/ne/lt/le/gt/ge i as_array_opt (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/vm/vm_value.md` — sekcja „Comparison" (linia 198) i „Accessors"  
**Plik src:** `src/vm/value/value.rs`  
**Faza usuwająca:** Phase 3C (`2026-08-17_phase3c_scattered_functions_removal.md`)

**Problem:**  
`vm_value.md` linia 198 stwierdza:

> Named comparison helpers: `xcx_eq`, `xcx_ne`, `xcx_lt`, `xcx_le`, `xcx_gt`, `xcx_ge` — thin wrappers used from the runtime and JIT FFI.

Phase 3C (tabela w wierszu `src/vm/value/value.rs`) jawnie usuwa wszystkie sześć: „`xcx_eq`, `xcx_ne`, `xcx_lt`, `xcx_le`, `xcx_gt`, `xcx_ge` (thin PartialEq/PartialOrd wrappers, zero callers)". Weryfikacja grep'em `value.rs` potwierdza — nie istnieją.

`as_array_opt()` wymienione w tabeli akcesorów `vm_value.md` (linia 221) również zostało usunięte w Phase 3C: „`as_array_opt`" wprost na liście. Nie istnieje w `value.rs`.

**Wymagana korekta:** usunąć oba podpunkty z `vm_value.md`.

---

### [DOC-3] `vm_objects.md` — nadal dokumentuje usunięty `StackGuard` (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/vm/vm_objects.md` — sekcja „Stack Structures / StackGuard" (linie 282–291)  
**Plik src:** `src/vm/stack/`  
**Faza usuwająca:** Phase 3A (`2026-08-17_phase3a_dead_file_removal.md`)

**Problem:**  
`vm_objects.md` opisuje `StackGuard` jako aktywną strukturę używaną w call handling. Phase 3A jawnie usuwa `src/vm/stack/stack_guard.rs` (wpis w tabeli: „never constructed; recursion cap enforced elsewhere"). Katalog `src/vm/stack/` zawiera tylko `mod.rs` i `value_stack.rs` — pliku `stack_guard.rs` nie ma.

**Wymagana korekta:** usunąć sekcję StackGuard z `vm_objects.md` lub zastąpić notą historyczną.

---

### [DOC-4] `vm_objects.md` — nadal dokumentuje usunięte `set.rs` / `set_op` (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/vm/vm_objects.md` — sekcja „Set Utilities (`vm/utils/set.rs`)" (linie 325–333)  
**Plik src:** `src/vm/utils/`  
**Faza usuwająca:** Phase 3A (`2026-08-17_phase3a_dead_file_removal.md`)

**Problem:**  
`vm_objects.md` opisuje funkcję `set_op(a, b, op: u8) -> BTreeSet<Value>` z `vm/utils/set.rs`. Phase 3A jawnie usuwa `src/vm/utils/set.rs` (wpis: „`set_op` | set algebra via `SetUnion`/`SetIntersection` opcodes"). Katalog `src/vm/utils/` nie zawiera `set.rs`.

**Wymagana korekta:** usunąć sekcję „Set Utilities" z `vm_objects.md`.

---

### [DOC-5] `vm_objects.md` — nadal odwołuje się do usuniętej `shallow_clone()` / `try_extend_bytes` (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/vm/vm_objects.md`  
**Pliki src:** `src/vm/object/json_val.rs`, `src/vm/object/string_obj.rs`  
**Fazy usuwające:** Phase 3C

**Problem A — json_val.rs / shallow_clone:**  
`vm_objects.md` linia 157 stwierdza:
> `shallow_clone() / deep_clone()` — shallow clone re-uses existing `Arc` references for nested objects; deep clone recursively copies every node. The JIT JSON parse cache uses `is_flat()` to decide which to use.

Phase 3C jawnie usuwa `shallow_clone` z `json_val.rs`. Grep na `src/vm/object/json_val.rs` nie zwraca żadnego wyniku dla `shallow_clone`. Dokumentacja nie powinna już wymieniać `shallow_clone` jako publicznej metody — tylko `deep_clone` i `is_flat`.

**Problem B — string_obj.rs / try_extend_bytes:**  
`vm_opcode.md` linia 194 opisuje StrAppendVar/StrAppendLocal/StrAppendMember/StrAppendElement jako operacje oparte na `StringObj::try_extend_bytes`. Phase 3C usuwa `try_extend_bytes` z `string_obj.rs` (wpis: „`try_extend_bytes` (COW append helper, zero callers)"). 

Grep na `src/vm/object/string_obj.rs` dla `try_extend_bytes` → brak wyników. Jednak `value.rs` implementuje COW append bezpośrednio w `add()` przez inplace mutation `(*obj_ptr).data.extend_from_slice(...)` bez delegacji do `try_extend_bytes`. Dokumentacja StrAppend w `vm_opcode.md` powołuje się na usuniętą metodę jako mechanizm — jest to technicalnie błędne (faktyczny mechanizm jest inline w `value.rs::add`, nie przez `try_extend_bytes`).

**Wymagana korekta:**  
- `vm_objects.md`: usunąć `shallow_clone` ze zdania o json_val; pozostawić `deep_clone` i `is_flat`.  
- `vm_opcode.md` linia 194: zaktualizować opis StrAppend aby nie odwoływał się do `try_extend_bytes`.

---

### [DOC-6] `jit_emitters.md` — Misc Emitters dokumentuje usunięte `emit_env_get` / `emit_env_args` (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/jit/jit_emitters.md` — sekcja „6. Misc Emitters" (linie 84–89)  
**Plik src:** `src/jit/emit_misc.rs`  
**Faza usuwająca:** Phase 3B (`2026-08-17_phase3b_tracejit_fiberjit_removal.md`)

**Problem:**  
`jit_emitters.md` linia 88 stwierdza:
> **OS Environment:** Accesses variables and startup scripts (`emit_env_get`, `emit_env_args`).

Phase 3B wymienia `emit_env_get` i `emit_env_args` jako „orphaned emitters" usunięte razem z trace compiler (ich ostatni caller to był martwy kompilator trace). Grep na całym `src/jit/` dla obu nazw — brak wyników. `emit_misc.rs` (2629 bajtów) nie zawiera żadnej z tych funkcji.

**Wymagana korekta:** usunąć wzmianki o `emit_env_get` i `emit_env_args` z sekcji misc emitters; opisać faktyczną zawartość `emit_misc.rs` (jedynie: `emit_halt_alert`, `emit_halt_error`, `emit_halt_fatal`, `emit_typeof`).

---

### [DOC-7] `jit_emitters.md` — Control Flow dokumentuje usunięte emittery yield/return-fiber (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/jit/jit_emitters.md` — sekcja „3. Control Flow Emitters" (linie 55–62)  
**Plik src:** `src/jit/emit_control.rs`  
**Faza usuwająca:** Phase 3B

**Problem:**  
`jit_emitters.md` linia 61 stwierdza:
> **Yield and Return (`emit_yield`, `emit_return`):** Serializes current compiler registers to `locals_ptr` and returns control to the interpreter parent frame, passing status states.

Phase 3B wprost wymienia jako usunięte z `emit_control.rs`: `emit_yield`, `emit_yield_void`, `emit_method_yield`, `emit_return_fiber` (razem z `emit_loop_exit`, `emit_loop_next_generic`, `emit_loop_next_int`, `emit_inc_local_loop_next`, `emit_inc_var_loop_next`, `emit_array_loop_next`, `emit_table_iter`, `emit_table_size`, `emit_guard_int/float/bool`).

Grep na `src/jit/emit_control.rs` dla `emit_yield`, `emit_method_yield`, `emit_return_fiber` → brak wyników.

Sekcja Control Flow dokumentuje też `emit_guard_int`, `emit_guard_float`, `emit_guard_bool` (linia 58) — wszystkie trzy usunięte przez Phase 3B jako orphaned.

Natomiast docstring w `jit_core.md` linia 3 poprawnie wspomina że fibery teraz są tylko interpretowane, i że fiber-JIT został usunięty — ale `jit_emitters.md` nie dostosowało sekcji kontrolnej do tej zmiany.

**Wymagana korekta:**  
- Usunąć `emit_yield`, `emit_return` z sekcji Control Flow w `jit_emitters.md` (jedynie `emit_return` jest live, bez yield).  
- Usunąć `emit_guard_int/float/bool` ze zdania o type guards; zamiast tego opisać że type guards sprawdzają `ctx.known_types` i emitują `xcx_jit_report_guard_failure` przez live `emit_call_guard_failure`.  
- Precyzyjnie: live emittery control-flow = `emit_jump_if`, `emit_return`, `emit_loop_next_opcode`, `emit_dec_local_loop_prev_opcode`, `emit_inc_var_loop_next_opcode`, `emit_dec_var_loop_prev_opcode`, `emit_array_loop_next_opcode`, `emit_set_loop_next`, `emit_table_iter_opcode`.

---

### [DOC-8] `vm_objects.md` — brak sekcji `BoolArrayObj` (BRAKUJE)

**Plik dokumentacji:** `documentation/compiler/vm/vm_objects.md`  
**Plik src:** `src/vm/object/bool_array_obj.rs` (562 bajtów)  
**Powiązanie:** DOC-1

**Problem:**  
`vm_objects.md` opisuje module layout `src/vm/object/` (linie 10–22), ale nie wymienia `bool_array_obj.rs` w liście plików ani nie zawiera sekcji opisującej `BoolArrayObj`. Tymczasem jest to aktywny obiekt związany z `TAG_BOOL_ARR = 17`, opisany w `jit_codegen.md` sekcja „BoolArray Fast Path" (linie 78–85) jako wymagający konkretnego memory layout na Windows x64 (data pointer offset 16, length offset 24).

**Wymagana korekta:** dodać `bool_array_obj.rs` do module layout w `vm_objects.md` i dodać sekcję `BoolArrayObj` opisującą strukturę i jej memory layout.

---

## NIEZGODNOŚCI WĄTPLIWE (prawdopodobnie celowe lub historyczne)

---

### [DOC-W1] `vm_opcode.md` — sekcja `GetMember` zduplikowana w tabeli Misc

**Plik dokumentacji:** `documentation/compiler/vm/vm_opcode.md` — linie 405–409 (Misc)  
**Obserwacja:** `GetMember` pojawia się dwukrotnie: raz w „Collections — Operations" (linia 300) i raz na końcu „Misc" (linia 409). Jest to duplikat w dokumentacji; opcode w kodzie jest jeden. Nie jest to błąd krytyczny, ale powoduje dezorientację.

---

### [DOC-W2] `vm_opcode.md` — sekcja StrAppend odwołuje się do `try_extend_bytes` (powiązane z DOC-5B)

Patrz DOC-5B wyżej — ten sam problem.

---

### [DOC-W3] `vm_objects.md` — `ValueStack` opisuje `MAX_STACK = 256K` ale note mówi że Executor używa własnego `Vec<Value>` 64K

**Obserwacja:** dokumentacja jest wewnętrznie spójna (nota wyjaśnia różnicę), ale `vm_executor.md` (linia 127) opisuje dwa rozmiary stack'a (64K lub 512K w zależności od `disable_jit`), a `vm_objects.md` mówi tylko o 64K jako inicjalnym rozmiarze. Nie jest to sprzeczność — `vm_objects.md` opisuje `ValueStack` struct (alternatywę 256K), a `vm_executor.md` opisuje faktyczny rozmiar Executor'a.

---

## RZECZY ZWERYFIKOWANE JAKO POPRAWNE (po remediacji)

| Aspekt | Status |
|---|---|
| `jit_core.md` — opis metody JIT (nie trace), fiber-JIT marked as removed | ✅ POPRAWNE |
| `vm_executor.md` — brak wzmianki o `traces`, `hotspot`, `recorder`, `trace_cache` | ✅ POPRAWNE |
| `vm_executor.md` — Executor struct nie zawiera tych pól | ✅ POPRAWNE (weryfikacja grep) |
| REPL `show_jit_stats()` — pokazuje tylko 2 wiersze (JIT Enabled, Warmup Limit) | ✅ POPRAWNE („Loop Traces"/„JIT-Compiled" usunięte w 3B) |
| `src/vm/trace/` — katalog nie istnieje | ✅ POPRAWNE (usunięty w 3B) |
| `src/jit/compiler_fiber.rs` — nie istnieje | ✅ POPRAWNE (usunięty w 3B) |
| `src/jit/loop_context.rs` — nie istnieje | ✅ POPRAWNE (usunięty w 3A) |
| `src/vm/core/arena.rs` — nie istnieje | ✅ POPRAWNE (usunięty w 3A) |
| `src/compiler/register_manager.rs`, `patch.rs`, `liveness.rs` — nie istnieją | ✅ POPRAWNE (usunięte w 3A) |
| `src/frontend/ast/visitor.rs` — nie istnieje | ✅ POPRAWNE (usunięty w 3A) |
| `src/frontend/parser/parse_query.rs` — nie istnieje | ✅ POPRAWNE (usunięty w 3A) |
| `src/vm/object/closure_obj.rs`, `upvalue_cell.rs` — nie istnieją | ✅ POPRAWNE (usunięte w 3A) |
| `TAG_ARENA` — brak w `tag.rs`, `value.rs`, `nan_boxing.rs` | ✅ POPRAWNE (usunięty w Phase 4) |
| `TAG_CLOSURE` — brak w kodzie | ✅ POPRAWNE (usunięty w 3A) |
| `MakeClosure opcode` — brak w `opcode.rs` | ✅ POPRAWNE (usunięty w 3A) |
| `Chunk::has_loops` / `calculate_has_loops` — brak w `chunk.rs` | ✅ POPRAWNE (usunięte w Phase 6 late find) |
| `jit_codegen.md` — `CodegenCtx` struct, sekcje BoolArray fast path, constant tracking, itp. | ✅ POPRAWNE |
| `vm_value.md` — brak wzmianki o TAG_ARENA | ✅ POPRAWNE |
| Repl.rs — brak wierszy „Loop Traces", „JIT-Compiled"; jest „Warmup Limit" | ✅ POPRAWNE |
| `emit_arith.rs` — brak `emit_div_int`, `emit_mod_int`, `emit_mod_poly`, `emit_div_poly` | ✅ POPRAWNE (usunięte w 3B/3C) |
| `jit.rs` — tylko struct JIT + new(), bez `compile()` trace compilera | ✅ POPRAWNE |
| `xcx_eq/ne/lt/le/gt/ge` — usunięte z `value.rs` | ✅ POPRAWNE (usunięte w 3C) |
| `as_array_opt` — usunięte z `value.rs` | ✅ POPRAWNE (usunięte w 3C) |
| `shallow_clone` — usunięte z `json_val.rs` | ✅ POPRAWNE (usunięte w 3C) |
| `try_extend_bytes` — usunięte z `string_obj.rs` | ✅ POPRAWNE (usunięte w 3C) |
| `emit_env_get`, `emit_env_args` — nie istnieją w jit/ | ✅ POPRAWNE (usunięte w 3B) |
| `emit_yield`, `emit_method_yield`, `emit_return_fiber` — nie istnieją w emit_control.rs | ✅ POPRAWNE (usunięte w 3B) |
| `emit_guard_int/float/bool` — nie istnieją w emit_control.rs | ✅ POPRAWNE (usunięte w 3B) |

---

## CZĘŚĆ 2 — Szeroka analiza stanu ogólnego (wszystkie documentation/compiler/)

Poniższe niezgodności zostały znalezione przez pełne przeczytanie **każdego** pliku w `documentation/compiler/` i porównanie z realnym `src/` — niezależnie od plików work/.

---

### [GEN-1] `ast.md` — module layout nadal zawiera usunięty `visitor.rs` oraz dokumentuje AstVisitor (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/frontend/ast.md` — linie 9–21 (module layout) i linie 241–252 (Visitor sekcja)  
**Plik src:** `src/frontend/ast/` — fizycznie brak `visitor.rs`  
**Faza usuwająca:** Phase 3A

**Problem A — module layout:**  
`ast.md` wymienia w bloku kodu modułu (linia 20): `└── visitor.rs — AstVisitor trait`. Plik fizycznie nie istnieje.

**Problem B — sekcja Visitor:**  
`ast.md` linie 241–252 dokumentują `AstVisitor` trait z metodami `visit_program`, `visit_stmt`, `visit_expr` i stwierdzają: „Default implementations walk the entire tree with no-op leaf visits. Implementors override only the node types they care about. Used internally by semantic analysis and optimization passes." Phase 3A usuwa `visitor.rs` z powodu: „zero implementors, zero users". 

Cała sekcja Visitor powinna zostać usunięta z `ast.md`.

**Wymagana korekta:** usunąć `visitor.rs` z module layout; usunąć sekcję `### AstVisitor` z `ast.md`.

---

### [GEN-2] `parser.md` — module layout jest nieaktualny (stara struktura plików) (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/frontend/parser.md` — linie 9–23 (module layout)  
**Plik src:** `src/frontend/parser/`

**Problem:**  
`parser.md` opisuje następującą strukturę:
```
├── parse_expr.rs
├── parse_stmt.rs
├── parse_type.rs
├── parse_misc.rs     ← NIE ISTNIEJE
├── recovery.rs
├── token_stream.rs
└── expander.rs
```

Rzeczywista zawartość `src/frontend/parser/` (15 plików):
- `parse_control.rs` — nie wymienione
- `parse_decl.rs` — nie wymienione  
- `parse_fiber.rs` — nie wymienione
- `parse_fn.rs` — nie wymienione
- `parse_table.rs` — nie wymienione
- `parse_misc.rs` — **nie istnieje** (wymienione w dokumentacji)

Wygląda na to że `parse_stmt.rs` (duży plik, 21KB) albo rozrósł się z `parse_misc.rs`, albo `parse_misc.rs` zostało rozdzielone na `parse_control.rs`, `parse_decl.rs`, `parse_fiber.rs`, `parse_fn.rs`, `parse_table.rs`. Dokumentacja nie odzwierciedla tego rozdziału.

**Wymagana korekta:** zaktualizować module layout w `parser.md` dodając 5 plików i usuwając `parse_misc.rs`.

---

### [GEN-3] `parser.md` — dokumentuje usunięte funkcje `peek_precedence` i `Precedence::for_token` (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/frontend/parser.md` — linia 113 i linie 130–132  
**Plik src:** `src/frontend/parser/pratt.rs`, `src/frontend/parser/precedence.rs`  
**Faza usuwająca:** Phase 3C

**Problem:**  
Linia 113: `Precedence::for_token(kind)` maps a `TokenKind` to its precedence — ta funkcja jest usunięta w Phase 3C (wpis: „`Precedence::for_token` (whole impl block; duplicate of live `Parser::current_precedence`)").

Linia 130: `current_precedence() / peek_precedence()` — `peek_precedence` jest usunięte w Phase 3C (wpis: „`peek_precedence`"). Aktualnie w kodzie istnieje tylko `current_precedence()`.

Linia 132: „The distinction matters during infix parsing where the parser must compare the current lookahead's precedence..." — ta fraza opisuje `peek_precedence` jako żywe narzędzie, co jest niepoprawne po jego usunięciu.

Grep'y na `src/frontend/parser/pratt.rs` i `precedence.rs` potwierdzają brak obu funkcji.

**Wymagana korekta:**
- Usunąć zdanie o `Precedence::for_token` z bloku Precedence Ranks.
- Z sekcji Pratt Expression Parser usunąć `peek_precedence`; zostawić tylko `current_precedence`.

---

### [GEN-4] `compiler_core.md` — module layout nadal zawiera usunięty `patch.rs` (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/compiler/compiler_core.md` — linia 16 (module layout) i linie 146–151 (sekcja Backpatching)  
**Plik src:** `src/compiler/`  
**Faza usuwająca:** Phase 3A

**Problem A — module layout:**  
`compiler_core.md` linia 16: `├── patch.rs — backpatching for jump instructions`. Plik fizycznie nie istnieje w `src/compiler/` (potwierdzone przez listing katalogów: lista 16 plików i katalogów, brak `patch.rs`).

**Problem B — sekcja Backpatching:**  
Linie 146–151 opisują `patch.rs` jako aktywny moduł z funkcjami `emit_jump → usize` i `patch_jump(ip)`. Phase 3A usuwa `src/compiler/patch.rs` jako martwy (wpis: „`patch_jump`, `patch_jump_to` | zero callers").

Analogiczne backpatchowanie (jeśli nadal istnieje) jest realizowane inline w `compile_control.rs` lub `compiler.rs` — sekcja powinna opisywać faktyczny mechanizm.

**Wymagana korekta:** usunąć `patch.rs` z module layout; usunąć lub przepisać sekcję „Backpatching".

---

### [GEN-5] `compiler_core.md` — odwołuje się do `RegisterManager` pass jako aktywnego (BŁĄD)

**Plik dokumentacji:** `documentation/compiler/compiler/compiler_core.md` — linia 110  
**Plik src:** `src/compiler/`  
**Faza usuwająca:** Phase 3A (`src/compiler/register_manager.rs` usunięty)

**Problem:**  
Linia 110 stwierdza:
> XCX's compiler uses an unoptimized flat register allocator during this phase; dense optimization and parameter pinning are deferred to the `RegisterManager` pass.

`register_manager.rs` (`RegisterManager::compress_registers`) został usunięty w Phase 3A jako: „zero callers; superseded register allocation". Ale `compiler_core.md` nadal powołuje się na niego jako na istniejący pass. Nie ma żadnego RegisterManager w kodzie.

Podobnie `compiler/README.md` (linia 8) wymienia `compiler_registers.md` i opisuje go jako: „tracking machine register controllers alongside security bounds validation for constants" — `compiler_registers.md` sam opisuje tylko `scope_tracker.rs` (live) i `FunctionCompiler` sequential allocation. Jednak komentarz w `README.md` brzmi jakby opisywał coś bardziej zaawansowanego niż to, co jest.

**Wymagana korekta:** usunąć zdanie o `RegisterManager` z linia 110 `compiler_core.md`; zastąpić opisem że brak post-pass optymalizacji rejestrów — alokacja jest sekwencyjna i finalna po emisji.

---

### [GEN-6] `runtime_collections.md` — błędny typ kontenera dla `MapObj` i `SetObj` (KRYTYCZNY BŁĄD FAKTYCZNY)

**Plik dokumentacji:** `documentation/compiler/runtime/runtime_collections.md` — linie 17 i 21  
**Pliki src:** `src/vm/object/map_obj.rs`, `src/vm/object/set_obj.rs`

**Problem A — MapObj:**  
`runtime_collections.md` linia 17 stwierdza:
> Represented by `MapObj` wrapping `RwLock<HashMap<Value, Value>>`.

Faktyczna implementacja (`map_obj.rs`):
```rust
pub struct MapObj {
    pub elements: Vec<(Value, Value)>,
}
```
MapObj używa `Vec<(Value, Value)>` — **nie HashMap**. Jest to ordered key-value dictionary (z zachowaniem kolejności wstawiania). Wyszukiwanie jest liniowe. `vm_objects.md` dokumentuje to poprawnie. `runtime_collections.md` jest **sprzeczne** z kodem i z `vm_objects.md`.

**Problem B — SetObj:**  
`runtime_collections.md` linia 21 stwierdza:
> Represented by `SetObj` wrapping `RwLock<HashSet<Value>>`.

Faktyczna implementacja (`set_obj.rs`):
```rust
pub struct SetObj {
    pub elements: BTreeSet<Value>,
    ...
}
```
SetObj używa `BTreeSet<Value>` — **nie HashSet**. `vm_objects.md` dokumentuje to poprawnie. `runtime_collections.md` jest sprzeczne z kodem i z `vm_objects.md`.

Błędy te nie są błahostką — HashMap vs Vec zmienia złożoność operacji (HashMap O(1) vs Vec O(n) lookup), a BTreeSet vs HashSet zmienia porządek iteracji (BTreeSet daje deterministyczną kolejność, HashSet nie).

**Wymagana korekta:**  
- Linia 17: zmienić `RwLock<HashMap<Value, Value>>` na `Vec<(Value, Value)>`; dodać wyjaśnienie że lookup jest liniowy i kolejność jest zachowana.
- Linia 21: zmienić `RwLock<HashSet<Value>>` na `BTreeSet<Value>`; dodać wyjaśnienie deterministycznej kolejności.

---

### [GEN-7] `runtime_core.md` — opis panic propagation jest nieścisły (WĄTPLIWY)

**Plik dokumentacji:** `documentation/compiler/runtime/runtime_core.md` — linie 85–89  
**Plik src:** `src/runtime/`

**Problem:**  
`runtime_core.md` stwierdza: „They trigger a panic `panic!(\"halt.error:...\")`. The compiler JIT or VM executor context catches the panic message..." 

W rzeczywistości FFI funkcje wywołane z JIT nie propagują panik przez granicę ABI w sposób opisany przez ten dokument — paniki w Rust przez `extern "C"` to undefined behavior. Faktyczny mechanizm to: FFI helper zapisuje błąd do `exec_ptr` (przez `vm.error_count`) i zwraca kod błędu jako `i32`. Sekcja jest myląca — opisuje mechanizm który nie jest poprawny dla boundary JIT-FFI.

(Uwaga: interpreter-side Rust kod Rust `panic!` może być złapany przez `std::panic::catch_unwind`, ale to nie jest opisane w tym dokumencie.)

**Wymagana korekta:** przepisać sekcję „Panic and Exit Propagation" aby opisywała faktyczny mechanizm przez `error_count` i kod powrotu.

---

### [GEN-8] `hir/` dokumenty — brak dokumentacji `pass.rs`, `lower_expr.rs`, `lower_stmt.rs` (BRAKUJE)

**Plik dokumentacji:** `documentation/compiler/hir/` — wszystkie 4 pliki  
**Plik src:** `src/hir/` — 11 plików

**Problem:**  
`src/hir/` zawiera 11 plików. Dokumentacja HIR ma 4 pliki (hir_core, hir_codegen, hir_inline, hir_lower). Fizycznie nieudokumentowane pliki z `src/hir/`:

| Plik | Rozm. | Brak w dokumentacji |
|---|---|---|
| `pass.rs` | 16,931 B | Całkowity brak |
| `lower_expr.rs` | 10,581 B | Całkowity brak (hir_lower.md opisuje ogólnie, nie wymienia pliku) |
| `lower_stmt.rs` | 13,568 B | Całkowity brak (jw.) |
| `inline_policy.rs` | 13,436 B | Wzmiankowany w hir_core.md (linia 79) ale nie udokumentowany |
| `compile_expr_special.rs` | 15,645 B | Całkowity brak |

`pass.rs` (17KB) jest prawdopodobnie głównym plikiem orchestrującym przejście HIR (potwierdzenie: `compiler_core.md` linia 116 wspomina `hir::lower_program` i `hir::run_inliner_pass` — te funkcje zapewne żyją w `pass.rs`). Brak dokumentacji.

`hir_lower.md` opisuje „lowering" ale nie wymienia `lower_expr.rs` i `lower_stmt.rs` — zakładając że są to właśnie te pliki.

**Wymagana korekta:** zaktualizować `hir/README.md` i `hir_lower.md` aby wymieniały `lower_expr.rs`, `lower_stmt.rs`, `pass.rs`, `inline_policy.rs`, `compile_expr_special.rs`; dodać sekcje lub nowe pliki dokumentacji dla `pass.rs`.

---

### [GEN-9] `vm_executor.md` — `SharedContext::http_req` opisany jako `Option<HttpRequest>` ale kod używa `Arc<Mutex<Option<tiny_http::Request>>>` (NIEZGODNOŚĆ TYPÓW)

**Plik dokumentacji:** `documentation/compiler/vm/vm_executor.md` — linie 62–75, sekcja SharedContext  
**Plik src:** `src/vm/core/vm.rs` linia 21

**Problem:**  
`vm_executor.md` linia 67 stwierdza:
```rust
pub struct SharedContext {
    pub http_req: Option<HttpRequest>,
}
```
Faktyczny kod (`vm.rs` linia 21):
```rust
pub http_req: Option<Arc<std::sync::Mutex<Option<tiny_http::Request>>>>,
```
Typ jest znacząco bardziej złożony: nie `Option<HttpRequest>` (który sugeruje własny typ), ale `Option<Arc<Mutex<Option<tiny_http::Request>>>>` (Arc Mutex wrap tiny_http). Dokumentacja ukrywa faktyczny typ — to powoduje niejasności przy debugowaniu przekazywania `http_req` przez warstwy.

**Wymagana korekta:** zaktualizować struct listing `SharedContext` w `vm_executor.md` aby pokazywał faktyczny typ `Option<Arc<Mutex<Option<tiny_http::Request>>>>`.

---

## Podsumowanie — kompletna lista plików do aktualizacji

| Priorytet | ID | Plik do zmiany | Co zmienić |
|---|---|---|---|
| WYSOKI | DOC-1 | `vm/vm_value.md` | Dodać TAG_BOOL_ARR (=17), TAG_FUNC_PTR (=16), BoolArray enum variant, konstruktory i predykaty |
| WYSOKI | DOC-2 | `vm/vm_value.md` | Usunąć `xcx_eq/ne/lt/le/gt/ge` i `as_array_opt` |
| WYSOKI | DOC-3 | `vm/vm_objects.md` | Usunąć sekcję StackGuard |
| WYSOKI | DOC-4 | `vm/vm_objects.md` | Usunąć sekcję Set Utilities (set.rs/set_op) |
| WYSOKI | DOC-5 | `vm/vm_objects.md` + `vm/vm_opcode.md` | Usunąć `shallow_clone`; zaktualizować opis StrAppend |
| WYSOKI | DOC-6 | `jit/jit_emitters.md` | Usunąć `emit_env_get`/`emit_env_args` z Misc |
| WYSOKI | DOC-7 | `jit/jit_emitters.md` | Usunąć `emit_yield`, fiber return, `emit_guard_int/float/bool` z Control |
| WYSOKI | DOC-8 | `vm/vm_objects.md` | Dodać `bool_array_obj.rs` do module layout i sekcję BoolArrayObj |
| WYSOKI | GEN-1 | `frontend/ast.md` | Usunąć `visitor.rs` z module layout; usunąć sekcję AstVisitor |
| WYSOKI | GEN-2 | `frontend/parser.md` | Zaktualizować module layout (dodać 5 nowych plików, usunąć parse_misc.rs) |
| WYSOKI | GEN-3 | `frontend/parser.md` | Usunąć `Precedence::for_token` i `peek_precedence` |
| WYSOKI | GEN-4 | `compiler/compiler_core.md` | Usunąć `patch.rs` z module layout; usunąć sekcję Backpatching |
| WYSOKI | GEN-5 | `compiler/compiler_core.md` | Usunąć zdanie o `RegisterManager` pass |
| KRYTYCZNY | GEN-6 | `runtime/runtime_collections.md` | Poprawić typ MapObj (Vec, nie HashMap) i SetObj (BTreeSet, nie HashSet) |
| ŚREDNI | DOC-W1 | `vm/vm_opcode.md` | Usunąć zduplikowany GetMember w Misc |
| ŚREDNI | GEN-7 | `runtime/runtime_core.md` | Przepisać sekcję Panic/Exit propagation |
| ŚREDNI | GEN-9 | `vm/vm_executor.md` | Poprawić typ `http_req` w SharedContext |
| NISKI | GEN-8 | `hir/hir_lower.md` + `hir/README.md` | Dodać pass.rs, lower_expr.rs, lower_stmt.rs, inline_policy.rs, compile_expr_special.rs |
