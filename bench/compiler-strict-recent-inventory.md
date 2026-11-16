# Recent strict replay inventory at 6a187b94

**This is static eligibility and membership evidence, not six new replay
successes.** No compiler or replay has run during preparation.

At `6a187b94fb763f7b0565436a98ab5feda3cd642f`, all **24** previously established
strict proofs remain in the last **300** commits. There are zero prior proofs
outside that named window. Their ranks are 42–170. The window's oldest commit is
`45c088f9053098fc5aeb50a7e9cc92bcc9f638e2`; its exact list has SHA-256
`bee88306e1c12adfb3e0f8dfc6a96bf71c6b8ad344d4d08c2cdb0b808293cd20`.
This claim does not move with future main commits.

## Six fixed candidates

The following list is chronological and disjoint from all prior strict proofs.
Each row's required set is the **complete** changed source/library set plus
every changed `.lkj` or `.rs` path anywhere, compared to the first parent.
All twelve selected path occurrences are modifications of existing direct
modules loaded unconditionally by `src/main.lkj` in both trees.

| Order | Revision | Changed lines, additions + deletions | Complete required path set |
| --- | --- | ---: | --- |
| 1 | `9047a677` | 26 | `src/semantic_facts.lkj` |
| 2 | `de65c09b` | 34 | `src/structure_edit.lkj`; `src/structure_write.lkj` |
| 3 | `0a410bad` | 482 | `src/ast.lkj`; `src/semantic.lkj`; `src/source.lkj`; `src/structure_edit.lkj`; `src/structure_replay.lkj` |
| 4 | `c5f67a55` | 433 | `src/structure_write.lkj` |
| 5 | `ba27e90a` | 27 | `src/semantic_store.lkj` |
| 6 | `71e821df` | 642 | `src/driver.lkj`; `src/store.lkj` |

The changes total 1,644 added/deleted source lines. The full revisions, first
parents, before/after source and library Git trees, and each required file's
before/after SHA-256 are retained in the adjacent candidates and required-paths
TSVs. The selected-order file has SHA-256
`dbefff80f9fd1164e81bf429a73a5978ef783868312309578e06cdf5ea1e4d21`.

A source/diff inspection found existing declaration bodies/items, new or removed
items inside existing files, and no required new directory, external-module
file, root import change, or reordered outer declaration framing. This supports
trying the current general replay operation; it does not prove that today's
compiler admits either historical program. The executable replay remains the
test, including its source-framing and independent checked-meaning conditions.

The physical historical library tree is unchanged within every pair:

| Cases | Both sides' library tree |
| --- | --- |
| 1–2 | `00d0571a0f4f36e2ea4a6d57a23d54afe360d763` |
| 3 | `e3ba335f2eeb20be7cf05404ecf97ba9c230e7ab` |
| 4–6 | `25bdb8ff8567b941790d3951c78e896b23e26e8e` |

## Population and exclusions

There are eleven language-changing commits between the previous self-history
horizon `a4fd685a9bb2e7736110a4ed2a43651dfe040d56` and this horizon, using
each revision's first-parent diff and `.lkj`/`.rs` paths anywhere. Six are
selected above. The other five remain explicit; none can silently replace a
refusal in the fixed sequence.

| Revision | Static classification and reason |
| --- | --- |
| `e03db719` | All five changed language files fit the existing alternate `src/structure_refactor_cases.lkj` root: elaboration, semantic evidence, and its cases. Main omits the cases root. This is a possible separate ordinary-root case, **not selected** in this six-case main-root campaign. |
| `579cf55e` | Runtime primitive change spans the compiler/library, off-root `src/semantic_tests/ir_binary.lkj`, and `witness/bridge/output.rs`. Excluded from a single current main-root proof. |
| `1c5d964e` | Directory identity change spans compiler/library, an off-root semantic test, `witness/bridge/output.rs`, and `witness/build.rs`. Excluded from a single current main-root proof. |
| `9c0d1797` | Main/driver/interpreter/writer changes also modify `src/semantic_tests/interpreter_contracts.lkj`. Existing main and programs-admission roots omit semantic tests; semantic-main omits the driver. No supported single root covers the full set. |
| `501db910` | Rust witness-only `witness/bridge/lib.rs` and `witness/bridge/main.rs`. Outside this source operation contract. |

The population-paths TSV retains every changed language path for all eleven
commits. No old configuration mode, Rust compatibility path, synthetic combined
root, or partial scope is introduced to fill a quota.

## Prior proof provenance and the counting rule

The 24 are two separately retained experiments:

| Experiment | Count | Exact producer SHA-256 | Proof-table SHA-256 |
| --- | ---: | --- | --- |
| Eligible imported self-use subset | 23 | `4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09` | `0aafd3a891bf224bd3113ab9de58219fecba44b84e0c5605a28f8ced0f25acb1` |
| Separate four-candidate probe | 1 | `4c4537ea7ccb31213f376d1402f619883eea6f8877fdb8b752b31df364099e09` | `02bd19ad873b7b20ae0a947513c3603443b97633aeefa2555f751d058c696daa` |

Their 137 operations and 46 required file facts remain historical evidence;
the old producer is not relabelled as today's producer. The one new success in
the four-candidate probe was `39d18d44`; the other three attempted cases and
their refusals remain intact. Earlier overlapping results are not added again.

The membership TSV lists each exact revision, original experiment and producer,
new-horizon rank, and proof-table digest. An eventual new result can establish
a union of `24 + new strict passes` at this horizon. **Thirty is conditional.**
A later moving 300-commit horizon would require a fresh membership calculation,
not an assertion that every historical proof stays in scope.

Here “strict” retains the established boundary: complete changed-language file
coverage plus canonical checked-meaning equality of the selected root. It does
not mean raw serialized IR identity, coverage of every root/configuration, or
reproduction of non-language effects of the Git commit. Exact target source text
equality, when observed, is separate.

## Reproduce the static audit

```sh
perl bench/compiler-strict-recent-inventory.pl \
  /home/coder/workspace/lkjscript bench/compiler-strict-recent --check
```

This command reads Git objects and existing proof tables. It verifies the fixed
300-commit list, all prior memberships, six disjoint candidates, exact changed
paths, ordinary direct-root declarations on both sides, file hashes, source and
library trees, and the eleven-commit classification. It never invokes
lkjscript. The separate predeclared runtime protocol is
`compiler-strict-recent.plan.md`; runtime results have not been substituted for
this static inventory.
