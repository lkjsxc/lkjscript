# Historical rename/comment candidates at the named horizon

This is a **static shortlist and exclusion review**, not a new compiler
comparison. It found no defensible rename-only historical example and two
textually comment-only revisions whose changed bytes also have observable
uses as data or test directives. No new semantic-equivalence result is claimed.

## Frozen scope and method

The population is the existing last-300 window at
`6a187b94fb763f7b0565436a98ab5feda3cd642f`, whose ordered revision list has
SHA-256 `bee88306e1c12adfb3e0f8dfc6a96bf71c6b8ad344d4d08c2cdb0b808293cd20`.
The [population table](compiler-history-comment-rename-data/population.tsv)
records every revision, first parent, changed language paths and exact textual
diff digest. **204** revisions change `.lkj` or historical `.rs` files.
This does not extend the historical population or introduce old-language
adaptations.

The retained [static screen](compiler-history-comment-rename-data/screen.pl)
compares complete changed-language diffs against each first parent. It screens
identifier-only changes as well as pathname renames, while retaining keywords,
punctuation and quoted literals and ignoring comments/whitespace. The two-item
[shortlist](compiler-history-comment-rename-data/shortlist.tsv) was then reviewed
against the actual diffs and embedded-file uses. Pure content-preserving
language pathname-renames numbered **zero**.

The lexical screen is intentionally only a shortlist: it is not a parser,
does not fully model raw strings or nested comments, and does not establish
semantic equivalence or a universal proof that every possible representation
of a rename is absent. The committed static evidence reproduces that bounded
screen; it does not rerun any historical compiler command.

## Two textual comment-only revisions, both excluded

| Revision and exact changed-language scope | Static finding | Why it cannot supply the requested full changed-language equivalence example |
| --- | --- | --- |
| `f71dd494d2c7b4ea04471bad6e75c26c4076eb67`, parent `26bf2c3c021dd79810a81fa1d9054d59f3768b06`; only `tests/diagnostics/external_child_authority.lkj` | Adds ten leading comment lines; no source code tokens change. | At that exact revision, `tools/consumer_edit_tests.lkj` embeds the whole file with `include_str!` and writes it as a fixture. The comments therefore change a real tool's emitted data. The diagnostic itself is a rejecting case. |
| `2d143a1fd27ce8e475653cebf6cdb85332032e32`, parent `77469a1075e54f8d61e0eba763e3e4feb1f404fd`; only `tests/diagnostics/formatted_panic_detail.rs` and `tests/diagnostics/unwrap_call_site.rs` | Replaces the two `// expect: trap` category lines with `R0004`. | `tools/runtime_diagnostic_tests.rs` embeds and writes the latter file. Separately, `src/test_runner.rs::case` parses these expectation comments into the expected trap category. These are observable fixture bytes and live test directives. |

The complete language diffs and exact-revision excerpts are retained in
[the evidence directory](compiler-history-comment-rename-data/).
The decisive references are the first revision's
`tools/consumer_edit_tests.lkj:289`, and the second revision's
`tools/runtime_diagnostic_tests.rs:56` and
`src/test_runner.rs::case` (the expectation parsing at lines 200–232).
No inference of meaning preservation is made merely because a language lexer
would ignore these comments.

The already retained main-root comparison rows have equal canonical meanings:
`8b3ddeefc395c7a11c533d873bb074ce7658b31236c492b6132a84a3d8af72d7`
for the first revision and
`72acbef8bb2065586a522c5b6f1d233f9fceacfb913e000f032d542d8485c718`
for the second. Both report **text identical, checked IR identical** for the
selected `src/main.lkj` root, which excludes the changed diagnostic inputs.
The second also used the original campaign's `legacy-rs-copies` framing.
Those old observations are preserved and cannot be promoted into coverage of
the changed tool/diagnostic roots. Their exact serialized IR digests differ.

## Rename exclusions

No additional identifier-only candidate emerged from the bounded lexical
screen. The plausible broad naming commit
`40acb5a7c83300e1d7d9fa1060d52e98ef1caf75` is not rename-only: besides names,
its IR parser begins requiring a `lkjscript-ir 1` header and its writer emits
that header. The naming-related `1fd442214a197d20a577debe7c37cbf694878f96`
and `9e64aa7fb65cf853866386c691d2511aacb0ca63` likewise add substantive
catalog/preservation behavior. The absence claim here is limited to the
reviewed shortlist and these concrete exclusions; there is no new checked
rename-preservation proof.

## Existing four-comment-line tree-pair proof

The separately recorded [strict derived-origin replay](compiler-strict-derived.md),
revision `0a410badb7e96ed9f6cf185df7d1f781d71c5fdb`, supplies a real
**desired-tree versus published-tree** comparison. Its five required
changed-language files are completely covered. Three published files differ
from the desired tree only by four leading documentation-comment lines:
one before `ast::LoadedModule`, one before `semantic::ModuleSource`, and
two before `structure_edit::Projection`.
The [exact retained diff](compiler-strict-derived.target-text.diff) has SHA-256
`e285c3bdcf840d7d076ab45ee81e81f34407d95f4bf913029728b750cdae5842`.

The existing replay proof gives both desired and candidate
`lkjscript-checked-meaning-1` identity
`2c178793d5b3aaefe27c4fce4dd36f90a69745e4f9b82878a5218a80eae70802`.
Its result commit is `fdd47054346e280ba7b2a9711a158340109c4c6c`.
This is an actual comment-only residual between two trees, not a comment-only
parent-to-commit revision, raw serialized IR equality, or equivalence for every
possible consumer that embeds those source bytes.

No compiler or historical replay was executed for this static investigation.
The earlier zero-of-six and five-of-six results remain unchanged.
