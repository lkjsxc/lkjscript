# Checkpoint fingerprint indexing — preparation only

Baseline main reported by root: d84cac6e; source file remains SHA-256 760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127.
Target identity: `elaborate::incremental::fingerprint`.
`replacement-body.txt` is the brace-delimited payload for one replace-body operation.
`candidate-fingerprint.txt` includes the unchanged signature for review.
`baseline-fingerprint.txt` retains the exact original item. No production file is changed.

## Implementation and order invariants

The existing library BTreeMap is an AVL tree in an indexed arena
(`library/ordered_map.lkj`, SHA-256 c307dfaf987976ce401a8404f5ddc23dbfd44f07f94d81eb627b95ce8bb339cb).
Its find_node/get/contains_key follow logarithmic tree height; insert balances ancestors.
The proposal uses existing contains_key/get/insert and Option::copied forms, with explicit
owned String keys and usize values. Only unique path strings are cloned; raw input bytes
are borrowed throughout.

A first-path table preserves the first matching input, including an invalid/truncated
first duplicate. FileId maps to Option<input index>, so a missing source still returns
None at its original per-file visit. The temporary path table is dropped after creating
that map. Function indices are grouped once in original function order, only for
body-bearing functions with a FileId below source.files.len(). Bodies are not dereferenced
during grouping: an out-of-range function FileId, even with an invalid body index, stays
ignored exactly as before.

Every existing per-file path/crate-root encoding, body-region validation/sort, raw gap,
body digest and suffix remains in its original order. Regions still sort by
(start,end,function index), and body digests still occupy original function-index slots.
No-body or ignored functions retain empty body strings. Missing-input and region/type
bounds failures retain None; extra allocation before a failure is not claimed resource-identical.

All extra non-source inputs, including duplicate unparsed paths, remain encoded in
original input order. Type nodes use their original FileId through the new input map.
Types, expressions/expanded String and Bytes literals, patterns, blocks, scopes and
functions retain their original serialization order. Local-item refusal, SHA-256 preimage,
topology domain, checkpoint format and capture policy are unchanged.

The removed searches are O(F*N+(F+T)*I). Index preparation is
O((I+F) log I+N+F), then direct FileId lookup for T type nodes; existing sorting,
serialization and hashing remain. Temporary storage is O(unique path bytes+I+F+N).
This is not a claim of bounded-by-edit overall cost or a measured speed/RSS result.

## Fixed correctness boundary

`fixtures.json` freezes two small source/input sets and seventeen explicit cases.
Goldens are intentionally pending: under a separate root release, use the existing
b01eabfa producer with the unchanged original fingerprint to retain full fixture bytes,
resolved coordinates, None/Some outcome and every shape/body key once. The eventual
tests assert those fixed vectors; they must not copy the new indexing algorithm.
Positive/source-valid base cases can also retain checked IR; intentionally malformed
AST-coordinate cases claim only the fingerprint helper's conservative behavior.

Proposed closed module: `semantic_tests::incremental_fingerprint`, four named groups
listed in the fixture. Register in semantic_tests::core and change semantic_main::closed_core
from 192 to 196 only if those four groups are adopted. Both changes and the module itself
must go through structural operations.

Existing store-test contracts cover successive one-body resume and cold byte equality,
generic/control/length misses, and parsed-source include_str/include_bytes invalidation.
Existing semantic-query-test contracts cover ordinary one-body resume followed by full
index re-inference and fresh/cold fact equality. Preserve independent IR admission and
generation-bound authenticated reuse. No new observer or mutation campaign is proposed.

No native/compiler/helper execution, performance run, compression, main-source edit,
or Git write was performed in this preparation. The closed twelve-command no-store
comparison does not exercise this path; any store-enabled measurement is a separately
frozen future cohort. No R0008 diagnosis follows.
