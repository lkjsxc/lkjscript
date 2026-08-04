# Package-Root Imports

## Purpose

Record the superseded package-root import baseline.

## Status

**Historical.** This record predates Current modules, manifests, and locks and
grants no active path-resolution interface or compatibility alias.

## Recorded Decision

At the recorded baseline:

- Every entry and import ended in `.lkjscript`; other extensions were rejected
  before parsing.
- `std/...`, `lib/...`, and `examples/...` mapped below package `src` categories.
- `./...` resolved relative to the importing file.
- Parent components, absolute imports, and other leading-dot forms were rejected.
- The package root was the nearest ancestor containing `src/std`; otherwise it
  was the entry file's parent directory.
- Missing local category directories fell back to `$LKJSCRIPT_ROOT/src/...`.
- Local category directories won over installed fallback.
- Canonical paths had to remain inside the project or installed root. On the
  then-selected Linux target, loading opened a stable regular-file descriptor,
  resolved the opened object through `/proc/self/fd`, and revalidated that
  descriptor-derived canonical path before byte reading or source identity.
  Changed, deleted, unresolvable, or escaping descriptor paths failed closed.
  Non-Linux host-path loading was not selected, and its safe fallback failed
  closed rather than claiming weaker TOCTOU containment.
- Source reads were bounded from the opened descriptor by the smaller remaining
  per-file and aggregate byte allowance plus one sentinel byte. Non-regular
  files, metadata/read size changes, and invalid UTF-8 source or host-derived
  logical paths were rejected before parser copying or semantic identity.
- Imported source directories obeyed the 16-entry language rule.
- Cycles failed and repeated canonical files were deduplicated.

Definitions at that baseline merged into one program-global namespace. Public
in-memory `compile_source` and `validate_source` accepted only canonical relative
non-dot UTF-8 logical paths ending in `.lkjscript`, like the recorded Semantic
Source validator; there was no absolute-path or `.lkjml` compatibility path.
This was source loading, not a complete module or package system.

## Deferred

Namespaces, explicit exports, manifests, versions, locks, registries,
content-addressed archives, dependency trust policy, and serialized compiled
packages require separate measured designs.
