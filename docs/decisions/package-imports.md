# Package-Root Imports

## Purpose

Define current path resolution, source suffix, and containment.

## Status

**Current.** Modules, manifests, versions, and locks remain **Deferred**.

## Decision

- Every entry and import ends in `.lkjscript`; other extensions are rejected
  before parsing.
- `std/...`, `lib/...`, and `examples/...` map below package `src` categories.
- `./...` resolves relative to the importing file.
- Parent components, absolute imports, and other leading-dot forms are rejected.
- The package root is the nearest ancestor containing `src/std`; otherwise it
  is the entry file's parent directory.
- Missing local category directories fall back to `$LKJSCRIPT_ROOT/src/...`.
- Local category directories win over installed fallback.
- Canonical paths must remain inside the project or installed root, preventing
  symlink escapes.
- Imported source directories obey the 16-entry language rule.
- Cycles fail and repeated canonical files are deduplicated.

Definitions currently merge into one program-global namespace. This is source
loading, not a complete module or package system.

## Deferred

Namespaces, explicit exports, manifests, versions, locks, registries,
content-addressed archives, dependency trust policy, and serialized compiled
packages require separate measured designs.
