# XCX Built-in Services: Networking, Cryptography, and Storage

The XCX runtime houses core service integrations enabling HTTP server execution, hashing/decryption routines, and file operations.

---

## Networking (`src/runtime/builtin/net/`)

The JIT and VM delegate networking logic to the `builtin/net` module, which leverages `tiny_http` and standard socket libraries.

### HTTP Server (`server.rs`)
The function `serve_impl` instantiates and drives the HTTP server context:
1. **Binding:** Binds an HTTP server to the designated `host:port` address.
2. **Pre-loop Preparation:** Clones the routes configuration (represented as a dynamic `Map` binding string paths to function/fiber identifiers).
3. **Request Thread Loop:** 
   - Listens on incoming connections. For each request, it spawns a standard thread (`std::thread::spawn`).
   - The thread receives the request, parses custom headers, and reads the payload body.
   - If the request body contains valid JSON, it unmarshals details into a structured `JsonObj` representation; otherwise, the body remains a raw `StringObj`.
   - The method constructs a context object carrying HTTP request definitions:
     ```rust
     let mut obj = Vec::new();
     obj.push(("method", JsonVal::String(req.method())));
     obj.push(("path", JsonVal::String(req.url())));
     obj.push(("headers", JsonVal::Object(headers)));
     obj.push(("body", body_val));
     ```
4. **Execution:** Delegates routing matching. Matches the pattern `METHOD URL` against keys in the routes dictionary. If a match is found, it runs the designated chunk inside VM executors (`vm.run`).
5. **Fallbacks:** If no route pattern matches, the handler serves a standard `404 Not Found` response.

### HTTP Client (`client.rs`)
Enables making external HTTP requests. Supports method verbs, headers payload conversion, thread isolation, and response retrieval.

---

## Date Services (`src/runtime/builtin/date/`)

Handles temporal calculations and dynamic formatting using the `chrono` library via `handle_date_method`:
- **Property Extractors:** Provides direct access to component integers for `Year`, `Month`, `Day`, `Hour`, `Minute`, `Second`, and `Ms` (millisecond of the second).
- **Format Token Translation:** Processes customized format templates into standard Chrono formats at runtime:
  - Translations: `YYYY` -> `%Y`, `MM` -> `%m`, `DD` -> `%d`, `HH` -> `%H`, `mm` -> `%M`, `ss` -> `%S`, `ms`/`SSS` -> `%3f`.
  - Safely handles literal escape sequences by translating standalone percentage tokens (`%`) to double percent marks (`%%`) to prevent format errors.
- **ToStr Conversion:** Formats UNIX millisecond timestamps directly to standard databases-compatible date-time representation (`%Y-%m-%d %H:%M:%S`).

---

## Cryptography (`src/runtime/builtin/crypto/`)

Provides password hashing utilities (`hash.rs`) and token generation routines (`token.rs`).

### Hashing Implementations
Supported cryptographic algorithms are executed dynamically via `hash_impl`:
- **Bcrypt:** Invokes the `bcrypt` library with a default cost configurations.
- **Argon2id:** Uses `argon2` memory-hard hashing. Generates a random 16-byte salt via `rand::fill`, returning the encoded representation.
- **Base64 Encoding/Decoding:** Offers standard base64 conversions.

### Verification Routines
The `verify_impl` helper validates raw password inputs against pre-calculated hash records:
- **bcrypt verification:** Calls `bcrypt::verify`.
- **argon2 verification:** Decodes the hash representation to an instance structure via `argon2::PasswordHash` and verifies the payload matches.

---

## Storage Services (`src/runtime/builtin/store/`)

Provides interface wrappers to filesystem storage logic (`read_write.rs`, `fs_ops.rs`).

### Path Security Constraints
Before executing any filesystem access (read, write, append, directory listing), paths are validated by `validate_path_safety`:
- Traversal checks reject paths carrying directory escape segments (`..`).
- System root blocks reject absolute prefix values `/` or drives mapping (`C:`).
Failure to pass security checks instantly raises a `halt.fatal` exception.

### Storage Operations
- **`read` / `write` / `append`:** Standard operations returning boolean status results. Auto-creates parents directories if missing.
- **`exists` / `is_dir` / `size` / `mkdir` / `list`:** Standard metadata and directory operators.
- **`glob`:** Performs pattern matching via the Rust `glob` crate, returning an array of string paths.
- **`zip` / `unzip`:** Compresses folders or extracts archives through internal helpers.
