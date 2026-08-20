# Selected-filesystem authority contract

This specification owns the Linux selected-root grant, ordered semantic paths, bounded directory
observation, regular-file read, expected-base save, unknown visibility, and reconciliation used by
`lkjstudio`. It does not grant ambient filesystem access or make paths semantic program identity.

## Grant and confinement

Selected-filesystem contract version 1 binds one process-local grant identity, a canonical selected
root, explicit list/read/write classes, and exact limits. Selection requires a non-symlink directory
and pins an open descriptor. The canonical host path is deployment state and is never exposed as a
buffer, file, application, workspace, or entity identity.

Every semantic path is an ordered sequence of validated UTF-8 components. Empty components,
`.`/`..`, slash, NUL, non-UTF-8 names, more than 32 components, more than 255 bytes per component, or
more than 4,096 bytes total reject. Linux `openat2` resolves descendants relative to the pinned root
with `BENEATH`, `NO_MAGICLINKS`, `NO_SYMLINKS`, and `NO_XDEV`. Directory traversal and regular-file
open also use no-follow flags. Symlinks and mount crossings are deliberately unsupported.

The deployment and adapter run under one trusted local operator/OS account. The contract confines
ordinary path resolution; it is not a hostile-native-code sandbox and does not defend against the
same account replacing the selected root through privileges outside the adapter.

## Directory observation

A list reads at most 4,096 entries, rejects non-UTF-8 or excessive names, observes each entry without
following links, and sorts by UTF-8 byte spelling. An entry reports name, file/directory/symlink/other
kind, and observed size. A page contains 1 through 256 entries.

The page digest binds the complete ordered observation. Its continuation contains that digest and
the next offset. If the directory observation changes before continuation, the next page is
`filesystem_conflict`; it is not silently spliced into an earlier listing. Listing is an observation,
not a filesystem snapshot, so a replacement during enumeration may instead return a typed I/O or
changed failure.

## File read and origin

A read accepts only one stable regular file no larger than 8 MiB. It opens relative to the pinned
root, records metadata before and after the bounded read, and returns `changed` if type, size, device,
inode, modification time, or change time differs. The returned version binds a domain-separated
BLAKE3 content digest plus size, device, inode, modification time, and change time. Content digest is
the expected-base authority for save; metadata is additional substitution/race evidence, not
semantic identity.

Read outcomes are opened, not found, wrong type, and changed. Invalid UTF-8 is not a filesystem
error: the generic adapter returns bytes, while `WorkbenchHost` classifies non-UTF-8 content as
unsupported for the text-only product. A workbench file-origin token binds grant, relative path,
and observed version and remains outside rendered content.

## Save publication

A save request binds contract version, a 1-to-64-byte ASCII action identity, exact path, content,
and either create-no-replace or replace-with-expected-content-digest mode. Content over 8 MiB rejects
before allocation or host work. Create succeeds only when the target is absent. Replace requires a
stable regular file with the exact expected digest. Matching intended content returns unchanged.

Publication creates one mode-0600 no-follow temporary under the target parent, writes and
synchronizes its complete content, then uses one atomic no-replace rename for create or atomic
exchange for replace. For replacement, the exchanged old file is checked against the expected
digest. The parent directory is synchronized; the old temporary is removed and the directory is
synchronized again. No automatic retry occurs.

Outcomes are published, unchanged, conflict, not found, wrong type, known previsibility failure,
and unknown visibility. A failure before rename is known not visible and disposable temporary state
is cleaned best-effort. After rename/exchange may have succeeded, after parent synchronization
fails, after old-base verification fails, or before final observation, visibility is unknown. The
outcome carries a bounded reconciliation token with action identity, path, expected digest,
intended digest, and temporary name. Terminal output never changes this classification.

## Reconciliation

Reconciliation independently reads the target under the same grant. Intended digest visible is
present. For create, absence is absent. For replace, the expected old digest visible is absent. A
third digest or missing replaced target is conflicting. Wrong type, unstable observation, denied
inspection, or insufficient evidence is indeterminate. Present may report cleanup pending and tries
to remove a retained exchanged-old temporary.

Reconciliation observes; it does not repeat the save and cannot infer success from a prior response
or terminal frame. The product blocks deliberate follow-up on an unknown action until the user asks
for reconciliation.

## Explicit absences

There is no ambient current-directory grant, raw absolute application path, hard-link prohibition
beyond the returned device/inode evidence, directory snapshot, recursive traversal, binary editor,
permission editor, owner/mode preservation, arbitrary create-directory operation, deletion, rename
command, watch service, advisory lock, network filesystem guarantee, Windows adapter, or cross-root
atomicity. These operation classes have no current product consumer.
