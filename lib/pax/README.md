# PAX Package Manager Manual

PAX is the official package manager for XCX, integrated directly into the `xcx` binary. It manages dependencies, project scaffolding, registry publishing, and build automation.

---

## Project Configuration: `project.pax`

Every PAX project must have a `project.pax` file in its root directory. It uses a custom declarative format.

```pax
---
PAX Project Configuration
*---
/
    name        :: "my_project",
    version     :: "1.0.0",
    author      :: "yourname",
    description :: "A short description of the project",
    main        :: "src/main.xcx",
    tags  :: ["tag1", "tag2"],
    files :: ["src/main.xcx", "src/lib.xcx"],
    deps  :: [
        "somelib@1.0.0",
        "author/repo@latest",
        "https://example.com/lib.xcx"
    ]
/
```

### Fields

| Field         | Required | Description                                                                 |
|---------------|----------|-----------------------------------------------------------------------------|
| `name`        | Yes      | Logical name of the project/package.                                        |
| `version`     | Yes      | Semantic version string (e.g. `"1.0.0"`).                                  |
| `author`      | No       | Author name, shown on the registry.                                         |
| `description` | No       | Short description of the project.                                           |
| `main`        | No       | Custom entry point. Defaults to `src/main.xcx` if omitted.                 |
| `tags`        | No       | List of tags for registry discoverability.                                  |
| `files`       | No       | Explicit list of files to include when publishing. If omitted, PAX excludes known non-code files automatically. |
| `deps`        | No       | List of dependencies. Supports versioned names, `author/repo` shortcuts, and direct URLs. |

### Dependency Formats

```pax
deps :: [
    "mylib@1.0.0",           --- versioned registry package
    "mylib@latest",          --- latest registry package
    "author/mylib@2.1.0",    --- with author prefix
    "https://example.com/lib.xcx"  --- direct URL
]
```

---

## Lock File: `pax.lock`

PAX automatically maintains a `pax.lock` file after each install. It records the exact resolved version and local path of every installed dependency.

```json
{
  "mylib": { "version": "mylib@1.0.0", "file": "lib/mylib/" },
  "otherlib": { "version": "otherlib@latest", "file": "lib/otherlib/" }
}
```

- Do **not** edit `pax.lock` manually.
- Commit it to version control for reproducible builds.

---

## Directory Structure

A standard PAX project follows this layout:

```
my_project/
├── project.pax       # Project configuration
├── pax.lock          # Auto-generated lockfile
├── src/
│   └── main.xcx      # Main entry point
├── lib/              # Installed dependencies (managed by PAX)
└── tests/            # Project-specific tests
```

---

## Command Reference

PAX is invoked via `xcx pax <command>`.

| Command                    | Description                                            |
|----------------------------|--------------------------------------------------------|
| `xcx pax new <name>`       | Scaffold a new project structure.                      |
| `xcx pax install`          | Fetch all dependencies into `lib/`.                    |
| `xcx pax add <dep>`        | Add a dependency and install it immediately.           |
| `xcx pax remove <name>`    | Remove a dependency from `project.pax`.                |
| `xcx pax clone <package>`  | Download a published package as a local project.       |
| `xcx pax run [path]`       | Execute the project entry point.                       |
| `xcx pax search <query>`   | Search the registry for available packages.            |
| `xcx pax upgrade <target>` | Upgrade the local XCX installation (xcx, tools, or all).|
| `xcx pax login <token>`    | Store a registry authentication token.                 |
| `xcx pax logout`           | Remove the stored authentication token.                |
| `xcx pax whoami`           | Verify your current registry account.                  |
| `xcx pax publish`          | Publish the project to the PAX registry.               |

---

## Command Details

### `xcx pax new <name>`

Scaffolds a new project with the standard directory layout.

```sh
xcx pax new my_project
```

Creates:
```
my_project/
├── project.pax
├── src/
│   └── main.xcx
└── README.md
```

---

### `xcx pax install`

Reads `project.pax`, resolves all `deps`, and downloads them into `lib/`. Skips packages already present (by name).

```sh
xcx pax install
```

- Only installs files declared in the dependency's own `files` field.
- If the dependency has no `files` field, PAX automatically excludes `README.md`, `LICENSE`, `tests/`, `docs/`, `.git*`, and `project.pax`.
- Updates `pax.lock` after each successful fetch.

---

### `xcx pax add <dep>`

Adds a dependency to `project.pax` and immediately runs install.

```sh
xcx pax add mylib@1.0.0
xcx pax add author/mylib@latest
```

---

### `xcx pax remove <name>`

Removes a dependency entry from `project.pax`.

```sh
xcx pax remove mylib
```

> **Note:** `lib/` folders and `pax.lock` entries are not automatically cleaned up. Re-run `xcx pax install` or remove them manually.

---

### `xcx pax clone <package>`

Downloads a published package from the PAX registry as a full local project, ready to run or modify.

```sh
xcx pax clone snake_game
xcx pax clone author/snake_game
```

- Creates a directory named after the package in the current working directory.
- Copies the full project structure including `project.pax`, `src/`, and all declared files.
- Does **not** install dependencies automatically.

**Typical workflow:**
```sh
xcx pax clone snake_game
cd snake_game
xcx pax install
xcx pax run
```

---

### `xcx pax run [path]`

Executes the project. Entry point resolution order:

1. Path provided as argument (e.g. `xcx pax run other/main.xcx`)
2. `main` field in `project.pax`
3. Default: `src/main.xcx`

```sh
xcx pax run
xcx pax run src/alt.xcx
```

---

### `xcx pax search <query>`

Searches the PAX registry for packages matching the query.

```sh
xcx pax search json
```

Output format:
```
- json_utils (v1.2.0) by alice
- fast_json (v0.9.1) by bob
```

---

### `xcx pax upgrade <target>`

Upgrades the local XCX installation itself, separate from project dependency management. Pulls release artifacts from the official GitHub releases.

```sh
xcx pax upgrade xcx      --- upgrade only the xcx binary (compiler/VM/JIT, includes REPL)
xcx pax upgrade tools    --- upgrade only pax, stdlib, and docs (installed under lib/ next to the binary)
xcx pax upgrade all      --- upgrade both xcx and tools
```

`<target>` is required. Running `xcx pax upgrade` with no target prints an error:

```
Error: specify what to upgrade: xcx, tools, all
```

#### `--check`

Add `--check` to any of the three targets to only compare the local version against the latest GitHub release tag and print the result, without downloading or installing anything.

```sh
xcx pax upgrade xcx --check
xcx pax upgrade tools --check
xcx pax upgrade all --check
```

#### Install layout

`tools` (pax, stdlib, docs) installs into `lib/` next to the `xcx` binary itself — this is a global, installation-level directory, distinct from a project's own `lib/` (which holds dependencies declared in that project's `project.pax`). Upgrading `tools` does not touch project-level dependencies.

```
<xcx install directory>/
├── xcx          --- binary (source)
└── lib/         --- pax, stdlib, docs (tools)
```

> **Note:** Downgrading to a specific older version is not currently supported.

---

### `xcx pax login <token>`

Stores a registry authentication token locally in `.pax_token`.

```sh
xcx pax login your_token_here
```

---

### `xcx pax logout`

Removes the stored authentication token.

```sh
xcx pax logout
```

---

### `xcx pax whoami`

Verifies your current registry session and prints your account info.

```sh
xcx pax whoami
# Account: alice [developer]
```

---

### `xcx pax publish`

Publishes the current project to the PAX registry. Requires login.

```sh
xcx pax publish
```

- Reads `project.pax` for metadata (`name`, `version`, `description`).
- Scans and uploads all project files (excluding `.zip`, `.pax_token`, `.git`).
- Sends a compressed archive + file manifest to the registry.

> **Note:** Bump `version` in `project.pax` before each publish. The registry may reject duplicate versions.

---

## Registry Configuration

By default PAX connects to `pax.xcxlang.com`. You can override this via a `.pax_config` file in the project root:

```json
{
  "registry": "https://my-custom-registry.example.com"
}
```

- HTTP is used automatically for `localhost` and `127.0.0.1`.
- HTTPS is used for all other hosts.