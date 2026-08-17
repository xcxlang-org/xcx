# Bug Report: JSON Parser Nesting/Recursion Limit

## Description
The JSON parser fails with a `halt.fatal: Invalid JSON (R305)` error when attempting to parse JSON payloads that are nested deeper than 128 levels. This is caused by the default recursion limit in `serde_json::from_str`.

## Reproduction
Run this compiler command on the reproduction script:
```cmd
cargo run -- bugs/json_recursion_limit/reproduce.xcx
```

Expected output:
Successful execution and parsing of a 1000-deep nested JSON tree.

Actual output:
`thread 'xcx-executor' panicked at ... halt.fatal: Invalid JSON (R305)`
