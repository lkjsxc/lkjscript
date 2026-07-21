# Limits

## Purpose

Document fixed language-spec source and tree budgets for this version.

## Spec constants

Hardcoded in the runtime (`MAX_*` in core `limits` module):

- `MAX_NEST_DEPTH`: 8
- `MAX_CHILDREN`: 8
- `MAX_TOKENS_PER_FILE`: 256 (primary file-size budget)
- `MAX_DIR_CHILDREN`: 8
- `MAX_TOPLEVEL_FORMS`: 8

These are not JSON-configurable. Changing them means a new language version.

Directory fan-out is enforced by `lkjscript2026-xtask check-tree`.
