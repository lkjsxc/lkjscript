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
- Canonical paths must remain inside the project or installed root. On the
  Current Linux acceptance target, loading opens a stable regular-file
  descriptor, resolves the actual opened object through `/proc/self/fd`, and
  revalidates that descriptor-derived canonical path before byte reading or
  source identity. Descriptor paths that are changed, deleted, unresolvable,
  or outside both roots fail closed. Non-Linux host-path loading is not an
  accepted target and its safe fallback fails closed rather than claim weaker
  TOCTOU containment.
- Source reads are bounded from the opened descriptor by the smaller remaining
  per-file and aggregate byte allowance plus one sentinel byte. Non-regular
  files, metadata/read size changes, and invalid UTF-8 source or host-derived
  logical paths are rejected before parser copying or semantic identity.
- Imported source directories obey the 16-entry language rule.
- Cycles fail and repeated canonical files are deduplicated.

Definitions currently merge into one program-global namespace. Public
in-memory `compile_source` and `validate_source` accept only canonical relative
non-dot UTF-8 logical paths ending in `.lkjscript`, exactly like the Semantic
Source validator; there is no absolute-path or `.lkjml` compatibility path.
This is source loading, not a complete module or package system.

## Deferred

Namespaces, explicit exports, manifests, versions, locks, registries,
content-addressed archives, dependency trust policy, and serialized compiled
packages require separate measured designs.
