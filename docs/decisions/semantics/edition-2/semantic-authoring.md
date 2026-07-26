# Edition 2: Semantic Authoring

[Authority](../edition-2.md)

## Purpose

Define the bounded Semantic Source operations that may author Edition 2 without
textual or destructive-edit ambiguity.

## Status

**Accepted Target, not Current** for a protocol-level `migrate_edition`
operation. The Current compiler-owned migration host boundary provides exact
`check_edition2_migration`, `diff_edition2_migration`, and
`publish_edition2_migration` with complete closure validation, impact,
reservation, atomic publication, rollback, and recovery. It is not inferred
from a Semantic Source request and does not expose a partial publish endpoint.

## Exact Operation Vocabulary

Schema 2 adds only these Edition 2 operations in this slice:

- `insert_enum`;
- `add_variant`, `remove_variant`, and `rename_variant`;
- `add_variant_field` and `replace_variant_field_type`;
- `insert_match`, `generate_match_skeleton`, and `add_match_arm`;
- `replace_pattern`;
- `fill_hole`; and
- `migrate_edition`, with exact `check`, `diff`, or `publish` mode.

No endpoint exists as a PLACEHOLDER or accepts an incomplete subset under one
of these names. Exact strict request variants state semantic IDs, pinned
revision, preconditions, and complete payloads; unknown operations or fields
fail. Additional edit names require a later complete contract rather than an
alias for one of these operations.

## Edit Impact

Enum and variant edits compute and return bounded impacts over every
constructor, nested pattern, match usefulness result, deterministic missing-case
witness, type substitution, layout plan, and downstream diagnostic. Field edits
also report declaration-order and evaluation-order changes. Renames preserve
stable semantic identity only when the identity contract says the renamed
entity is the same declaration; the response states old and new identities.

Removing a variant or replacing a field type is destructive and requires exact
preconditions for every constructor, pattern, exhaustiveness proof, and public
API impact. Unsupported external domains reject the operation. Newly incomplete
uses are returned as diagnostics or explicit requested holes; the service never
silently inserts a wildcard, default arm, conversion, trap, hole, or lossy
replacement.

`generate_match_skeleton` creates one source-ordered arm for every stable
variant ID, exact named-field patterns, and typed body holes. Boolean creates
`true` and `false`; infinite constructor spaces require an explicit finite set
plus a requested wildcard hole. It never adds a wildcard to a closed enum.

Migration obeys semantic check/diff/publish and atomic closure publication. All
operation, candidate, matrix, witness, impact, staged-byte, and response work is
charged before publication; truncation cannot turn incomplete impact into
authority.
