# Selected-filesystem authority contract

This specification owns selected-filesystem contract 2: one Linux selected-root grant, ordered
semantic paths, bounded directory/read/search observation, expected-base save, unknown visibility,
and reconciliation used by `lkjedit`. It grants no ambient filesystem and makes no path semantic
program, buffer, view, tab, or file-origin identity.

## Grant and confinement

A process-local grant binds one canonical selected root, explicit list/read/search/write classes,
and exact limits. Selection requires a non-symlink directory and pins an open descriptor. The host
path remains deployment state.

Every semantic path is an ordered sequence of validated UTF-8 components. Empty, `.`/`..`, slash,
NUL, non-UTF-8, more than 32 components, more than 255 bytes per component, or more than 4,096 bytes
total reject. Linux `openat2` resolves from the pinned root with `BENEATH`, `NO_MAGICLINKS`,
`NO_SYMLINKS`, and `NO_XDEV`; descendant opens also use no-follow. Symlinks and mount crossings are
unsupported.

The adapter assumes one trusted local operator/OS account. It confines its own ordinary resolution;
it is not a hostile-native-code or hostile-same-account sandbox.

## Directory and file observation

Listing reads at most 4,096 entries, rejects non-UTF-8/excessive names, observes without following,
and sorts by UTF-8 byte spelling. An entry reports name, file/directory/symlink/other kind, and size.
A page contains 1–256 entries. Its continuation binds the digest of the complete ordered
observation and next offset. A changed observation before continuation is `filesystem_conflict`,
never a silent splice.

A read accepts one stable regular file no larger than 8 MiB. Metadata before/after the bounded read
must agree on type, size, device, inode, modification time, and change time. The returned version
binds those facts plus a domain-separated BLAKE3 content digest and ordinary permission mode. The
digest is expected-base authority; metadata is substitution/race evidence, not semantic identity.

Opened, absent, wrong-type, and changed remain distinct. The generic adapter returns bytes;
`WorkbenchHost` classifies invalid UTF-8 as unsupported text without replacement characters. A
product origin token binds grant/path/observed version outside buffer content.

## Recursive literal search

A search request binds contract 2, one relative start directory, and a nonempty exact UTF-8 literal
of at most 4,096 bytes. Traversal is depth-first in sorted byte-spelling order. Files are stable-read
through the same owner; invalid UTF-8 files count as unsupported. Matching is exact,
case-sensitive, non-normalizing, overlapping byte search. Each match reports relative path, exact
UTF-8 byte start/end, and a bounded safe preview.

Admission stops explicitly at 4,096 directories, 8,192 files, 64 MiB total declared/observed bytes,
or 4,096 matches. At most 64 per-path failures and 256 preview bytes per match are retained.
Cancellation is an atomic read-only checkpoint observed between tasks and matching work; the result
states `cancelled` or `truncated` and is never labeled complete. Search holds no application state
and cannot open a tab itself.

## Save publication

A save binds contract 2, a 1–64-byte ASCII action identity, exact path, content, and either
create-no-replace or replace-with-expected-content-digest. Content over 8 MiB rejects before host
publication work. Create succeeds only for an absent target and publishes deterministic mode 0644.
Replace requires a stable regular file at the expected digest and preserves its
observed ordinary mode bits. Matching intended bytes return unchanged without replacement.

The owner creates one no-follow mode-0600 temporary in the target parent, changes it to the selected
publication mode, writes and synchronizes all bytes, then uses atomic no-replace rename for create or
atomic exchange for replace. After exchange it verifies the old file against the expected base,
synchronizes the parent, removes the exchanged-old temporary, and synchronizes again. No automatic
retry occurs.

Outcomes are published, unchanged, conflict, absent, wrong type, known previsibility failure, and
unknown visibility. Before rename, failure is known not visible and temporary cleanup is bounded
best effort. After rename/exchange may have occurred, parent sync/old-base verification/final
observation failure is unknown. The opaque bounded token binds action, path, expected/intended
digests, and temporary name. Terminal output cannot change publication classification.

## Reconciliation

Reconciliation independently stable-reads the target under the same grant. Intended digest visible
is present. For create, absent target is absent. For replacement, the expected old digest visible is
absent. A third digest or missing replacement target is conflicting. Wrong type, unstable or denied
observation, or insufficient evidence is indeterminate. Present may report cleanup pending and makes
one bounded best-effort cleanup attempt.

Reconciliation observes; it never repeats save or infers success from an earlier response. Product
meaning blocks duplicate publication while visibility is unknown and treats present, absent,
conflicting, and indeterminate as separate user-visible transitions.

## Explicit absences

There is no ambient current-directory grant, raw absolute application path, hard-link prohibition
beyond returned device/inode evidence, durable directory snapshot, binary editor, permission editor,
owner editing, mkdir/delete/rename, watch service, advisory lock, multiple roots, symlink following,
network-filesystem guarantee, Windows adapter, or cross-root atomicity. Each requires a complete
product command and separate race/publication/recovery contract.
