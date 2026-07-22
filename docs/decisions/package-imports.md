# Package-root imports

## Context

Relative `../` climbs made libraries and examples brittle under directory
fan-out.

## Decision

- Import paths that do **not** start with `.` resolve from the package root.
- Special prefixes:
  - `std/...` → `src/std/...`
  - `lib/...` → `src/lib/...`
  - `examples/...` → `src/examples/...`
- Paths starting with `./` resolve relative to the importing file.
- Paths containing `..` are rejected.
- Package root is the nearest ancestor containing `src/std/`; otherwise it is
  the current working directory.
- If a project has no local `src/std`, `src/lib`, or `src/examples`, those
  prefixes fall back to the corresponding directory under
  `$LKJSCRIPT_ROOT/src`.
- Project-local library directories win over the installed fallback.

Example:

```text
import/
std/list/nth.lkjml
/import
```

The same form accepts `lib/lkjedit/loop.lkjml` and
`examples/hello/fact.lkjml`.

## Consequences

Modules are identified by logical path (`std/…`, `lib/…`, `examples/…`).
Top-level `def` remains program-global for this release. Docker sets
`LKJSCRIPT_ROOT` to its bundled libraries, so bind-mounted projects do not
need to copy the standard library.
