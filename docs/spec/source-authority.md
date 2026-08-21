# Source authority contract 1

This specification is normative for maintained program authority. `authority.rs`, `workspace.rs`,
the public CLI, and their corruption/publication tests are the executable oracle. This contract
does not own semantic typing, compiled artifacts, deployment grants, application data, or Git
history.

## Authored representation

One project root contains one strict `lkjscript.package.json` and the explicitly listed `.lkj`
modules beneath `src/`. Those bytes are the sole editable truth. A module is UTF-8 with LF line
endings, no BOM, carriage return, or NUL. Semicolon comments and whitespace are authored bytes and
therefore participate in authored history. The parser derives a canonical comment-free form stream
for semantic equality; it is never editable authority.

Default per-module parsing bounds are 1,048,576 bytes, 100,000 forms, nesting depth 256, 256 bytes
per atom, and 1,048,576 bytes per string. A package descriptor is at most 1,048,576 bytes. One
accepted revision is limited to 64 MiB of source and 512 MiB of exact dependency artifacts. Bounds
are checked before retained allocation or traversal.

Source spans use zero-based byte start/end and one-based line/column diagnostics. The public
location contains the descriptor/module path, not an IR or bytecode index.

## Identity and equality

Package identity is an explicit nonzero 32-character lowercase hexadecimal value and cannot change
inside one project history. A semantic declaration owner is the tuple `(package_id, module_name,
declaration_name)`. Paths are locators and do not enter this tuple. Renaming or moving a module or
declaration creates a different owner; contract 1 has no continuity declaration and never infers
identity from syntax position.

The authored revision digest is BLAKE3 derive-key domain `lkjscript.authored-revision.v1` over each
lexicographically ordered path and exact byte string with lengths. Semantic revision equality uses
the validated canonical module forms and semantic package metadata. Formatting/comment-only edits
may therefore change authored equality without changing semantic equality.

Internal objects use BLAKE3 derive-key domain `lkjscript.source-object.v1`, plus object kind and
length. Digests are 64 lowercase hexadecimal characters. A digest proves equality in this domain;
it does not prove provenance or authorization.

## Publication

Authority lives under `.lkjscript/source-v1`. An exclusive project lock serializes publication.
`validate` binds an exact current revision and record digest, parses and semantically validates all
changed working authority, and writes nothing. `apply` repeats validation, acquires the lock, and
rejects if either exact base changed.

For an authored change, publication writes and synchronizes immutable source/dependency objects,
then a revision manifest, revision record, and revision index, and finally atomically replaces and
synchronizes `HEAD.json`. The HEAD replacement is the sole current-publication point. Success
advances exactly one revision. Rejection, stale base, validation, and authored no-change do not
advance HEAD. A formatting-only change is an authored publication and reports
`semantic_changed: false`.

An interruption before HEAD may leave unreachable immutable objects. They confer no current
authority and may be deleted only by a future exact reachability collector. An interruption after
HEAD has published one reconstructible revision. Conflicting bytes at an existing object digest are
corruption, never replacement.

## History, restore, and backup

Each record binds package, monotonically increasing revision, parent record digest, manifest,
authored digest, and semantic digest. Current/historical reads reconstruct from records, manifests,
source objects, and dependency objects. Shallow doctor checks current authority; deep doctor walks
the complete parent chain and every reachable object. A revision index is reconstruction aid, not
an alternate history owner.

`project restore --revision N` replaces working source bytes only and publishes nothing. A later
exact-base apply creates an ordinary new revision; accepted history is never rewritten.

Backup snapshots the exact accepted closure under the publication lock, writes a checksum manifest
to a new temporary directory, then renames it to a required-absent destination. Restore validates
all paths, sizes, checksums, records, and objects into a required-absent project before publication.
Backup and restore do not migrate predecessor formats.

## Hostile input and predecessor policy

Source, package JSON, internal JSON, paths, digests, dependency artifacts, and backups are hostile.
Unknown fields, trailing input, noncanonical names/paths/digests, symlinks at authority boundaries,
excess, missing objects, identity mismatch, corrupt checksums, future revisions, and broken parent
chains reject with stable diagnostic class/code.

The predecessor `.lkjscript/project` marker is rejected as `source_predecessor_rejected`. There is
no graph-project migration, dual reader, or automatic format upgrade in the current executable.
