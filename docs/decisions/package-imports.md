# Package-Root Imports

## Purpose

Define current path resolution and its accepted containment/extension repairs.

## Status

Package-root categories and importer-relative `./` are **Current**. Canonical
`.lkjscript` enforcement and containment hardening are **Accepted Target**.

## Current Decision

- `std/...`, `lib/...`, and `examples/...` map below package `src` categories.
- `./...` resolves relative to the importing file.
- Strings containing `..` are rejected.
- The package root is the nearest ancestor containing `src/std`; otherwise the
  current working directory is used.
- Missing local category directories fall back to `$LKJSCRIPT_ROOT/src/...`.
- Local category directories win over installed fallback.
- Cycles fail and repeated canonical files are deduplicated.

Current definitions merge into one program-global namespace; this is source
loading, not a complete module/package system.

## Accepted Target

- Every source and import ends in `.lkjscript`; `.lkjml` is rejected.
- Absolute import paths are rejected.
- Canonicalized paths remain inside the selected project or installed category
  root; symlinks cannot escape it.
- Imported source directories obey the 16-entry language rule.

## Deferred

Namespaces, exports, manifests, versions, locks, registries, content-addressed
archives, and serialized compiled packages require separate decisions.
