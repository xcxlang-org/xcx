# Versioning Scheme

This document describes the versioning schemes used by this project. Two schemes have been used across the project's history — the rules for each are described below. Concrete version numbers are given only as illustrative examples, not as part of the definitions themselves.

## Classic Scheme

A standard three-segment scheme: `MAJOR.MINOR.PATCH`

- **MAJOR** — A major leap for the project. This does not necessarily mean a rewrite of the core architecture (though that has happened) — more often it reflects a large number of significant changes landing together across many areas: new large subsystems, resolution of architectural inconsistencies, and broad bugfixing, together representing a change large enough to no longer fit within a single MINOR.
- **MINOR** — New features and larger improvements built on the same underlying foundation.
- **PATCH** — A hotfix or a small, quick correction that introduces no new functionality.

A full rewrite of the underlying foundation (VM, JIT, etc.) has occurred within some MAJOR releases, but is not part of the definition of MAJOR — some MAJOR releases consisted of many significant changes across many areas without any such rewrite.

**Reading:** segments are read in order, as plain numbers.

*Example: version `4.2.1` corresponds to MAJOR=4, MINOR=2, PATCH=1, read as "four-two-one".*

## Letter Scheme

Schema: `MAJOR.MINOR + LETTER [.HOTFIX]`

- **MAJOR** — A higher and narrower threshold than in the classic scheme: specifically, a fundamental architectural change (e.g. a VM rewrite, a new JIT backend), not simply "many changes at once" as under the classic scheme. What would have qualified as MAJOR under the classic scheme (a large leap without a foundational rewrite) now typically falls to the MINOR level instead.
- **MINOR** — The equivalent of the classic scheme's MAJOR: a large leap across many areas, without requiring a rewrite of the foundation, now expressed as MINOR under the new, narrower MAJOR.
- **LETTER** (`a`, `b`, `c`, ...) — The equivalent of the classic scheme's MINOR: new features and improvements within the same MINOR.
- **HOTFIX** (optional fourth segment) — Identical in meaning to PATCH under the classic scheme: a pure bugfix, with no new functionality.

**The letter is mandatory, not optional** — there is no such thing as a bare MINOR version without a letter. The first letter of any MINOR (and of any new MAJOR) always starts at `a`. This does *not* imply that a prior letter existed — `a` is simply first in sequence, the same way MINOR=0 or MINOR=1 would be the first minor under the classic scheme.

Incrementing MINOR (e.g. `5.1` → `5.2`) resets the letter back to `a`.

**Reading:** segments are read in order; the letter is read as "[number]-[letter]".

*Optional escape hatch (a practical consideration, not a strict rule): if a very long time passes without a MAJOR increment despite continued significant development, MAJOR may be bumped simply to keep the numbering feeling current, even without strictly meeting the criterion above. This is a case-by-case judgment call, not an automatic rule.*

### Examples

| Version | Reading | Meaning |
|---|---|---|
| `5.0a` | "five zero-a" | MAJOR 5, MINOR 0 (equivalent of a standalone classic-scheme major), first letter |
| `5.1a` | "five one-a" | MINOR bump (5.1) — equivalent of a classic-scheme MAJOR bump — first letter of this MINOR |
| `5.1b` | "five one-b" | Next letter within MINOR 5.1 — equivalent of a classic-scheme MINOR bump |
| `5.2a` | "five two-a" | Next MINOR bump (5.2) — letter resets to `a` again |
| `5.1a.1` | "five one-a one" | Hotfix #1 to version 5.1a |

| Classic scheme level | Letter scheme equivalent |
|---|---|
| *(new, narrower threshold — foundational rewrite only)* | MAJOR |
| MAJOR | MINOR |
| MINOR | LETTER |
| PATCH | HOTFIX |

## Naming Conventions

Starting from version `5.0a`, a standardized file and directory naming convention will officially take effect across the repository (concerning platform directories, test layouts, and external assets) to ensure consistency.