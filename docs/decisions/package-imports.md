# Package-root imports

## Context

Relative `../` climbs made libraries and examples brittle under directory
fan-out.

## Decision

- Import paths that do **not** start with `.` resolve from the package root.
- Special prefixes:
  - `std/...` → `src/std/...`
  - `lib/...` → `src/lib/...`
- Paths starting with `./` resolve relative to the importing file.
- Paths containing `..` are rejected.
- Package root is the nearest ancestor containing `src/std/`.

Examples:

- `<import>std/list/nth.lkjsxc</import>`
- `<import>lib/edit/loop.lkjsxc</import>`
- `<import>examples/hello/fact.lkjsxc</import>`

## Consequences

Modules are identified by logical path (`std/…`, `lib/…`, `examples/…`).
Top-level `def` remains program-global for this release.
