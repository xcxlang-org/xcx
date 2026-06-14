# doc

XCX compiler error code reference tool.

```sh
xcx doc S103
xcx doc R305
xcx doc list
```

Works fully offline — no network required.

## Install

```sh
xcx pax clone doc
cd doc
xcx pax run
```

## Files

- `doc.xcx` — entry point
- `src/errors.xcx` — full error registry
- `src/lookup.xcx` — code lookup logic
- `src/format.xcx` — terminal output formatting
