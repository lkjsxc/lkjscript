# Graph-native bootstrap and incremental-semantics campaign ledger

Status: active campaign evidence, not normative product authority.

## Campaign identity

- Active prompt: `prompts/202608221727.md`.
- Delivered policy: `AGENTS.md`, SHA-256
  `11e67d384512333de83511b47254fb9ce542b45272095a85768f6df0259b05e5`, 43,957
  bytes, 1,483 lines.
- Audited commit and upstream: `ae2bf52dd838f43a555dcf47320078b77a1fdc7e` on `main`, equal to
  `origin/main` at 2026-08-22T09:18:37Z.
- Initial user-owned worktree changes: modified `AGENTS.md` and untracked
  `prompts/202608221727.md`. No other modified or untracked path was present.
- Environment: Linux `7.0.0-29-generic` x86-64, 20 logical processors, 32 GiB memory;
  `rustc 1.96.0 (ac68faa20 2026-05-25)` and Cargo 1.96.0.
- Provider token, cache, retry, and monetary telemetry: unavailable. No cost conclusion is made.

## Reproduced graph-1 baseline

The release build completed with locked dependencies in 84 seconds. The stripped executable is
13,403,288 bytes with SHA-256
`7ca23a6a2264ed7847a897291b9a3046f1149baedf5e6cb9995370e5fba05458`.

One warm black-box sample per operation was run with the release executable. Elapsed values include
process startup; stdout and stderr are exact byte counts.

| Operation | Elapsed | stdout | stderr | Result |
|---|---:|---:|---:|---|
| top-level machine help | 4.828 ms | 782 B | 0 B | 40 command names |
| `lkjournal` status | 10.126 ms | 712 B | 0 B | revision `rev_441156fb...` |
| orientation, limit 20 | 6.807 ms | 1,710 B | 0 B | 3 modules, 2 targets |
| exact `handle` lookup | 4.372 ms | 645 B | 0 B | work 1 |
| exact body inspection | 6.313 ms | 5,613 B | 0 B | task function |
| standard tests | 6.901 ms | 414 B | 0 B | 6 pass, tiers equal |
| `lkjournal` tests | 28.515 ms | 419 B | 0 B | 11 pass, tiers equal |
| `lkjournal` build | 23.974 ms | 587 B | 0 B | 160,195-byte artifact |
| `lkjournal` deep doctor | 8.497 ms | 299 B | 0 B | 1 revision, 3 modules |
| `lkjournal` backup | 13.261 ms | 455 B | 0 B | 160,697 bytes, 7 entries |
| blank-directory restore | 51.209 ms | 992 B | 0 B | exact revision restored |
| restored tests | 24.608 ms | 419 B | 0 B | 11 pass, tiers equal |

The restored backup digest was
`backup_07401565b5930adf3645af1ad59ceac2b5ab29bf52679bdfe8f5de95c2a654ba`.
The restored repository retained repository and revision identity and passed deep doctor and both
execution tiers.

The public registry exposes 40 names. `text-project`/`export-text` and
`backup`/`export-bundle` are duplicate behaviors. All development commands are nested under
`semantic`; ordinary creation additionally exposes storage-shaped `id-allocate` and `import`.
There is no project-creation command.

The 10,000-added-module public scale fixture was reproduced from the standard artifact:

- one 10,000-operation apply: 493.483 ms and 5,378 response bytes;
- cold exact lookup: 69.171 ms; warm exact lookup: 17.402 ms, work 1;
- orientation: 124.724 ms;
- deep doctor: 93.362 ms across 10,024 retained module versions;
- build: 132.192 ms and a 1,330,505-byte artifact;
- backup: 96.648 ms and 2,183,176 bytes;
- canonical store: 1,832,489 bytes; store with derived indexes: 6,631,953 bytes.

The fixture remains `many_tiny_modules`; dense relations, long history, conflicts, and one million
owners are not covered by this baseline.

## Reproduced graph-1 baseline architecture findings

1. `execute_transaction` calls `reconstruct_current`, clones the complete root and module vector,
   and canonicalizes the complete candidate before plan, validate, or apply can return.
2. `publish_locked` validates the complete proposal again through direct and packed-reconstruction
   paths, encodes every proposed module, and invokes immutable writes for every module version.
3. the canonical root is one sorted vector of every module reference. Exact module-by-ID and
   module-by-name reads scan that vector.
4. module rename walks every module to replace name-bearing imports and also rewrites target
   locator fields. Stable identity is therefore not yet sufficient to make rename local.
5. broad indexes are disposable and safe on loss, but cold construction reconstructs the complete
   revision. The current shard layout created 256 name and 256 owner files for a three-module
   application, which is measurable small-project metadata overhead.
6. initialization is already private-stage plus one rename visibility point, but its only public
   route requires a pre-existing current graph artifact and preserves that artifact's repository
   identity.
7. backup is a monolithic in-memory version-1 container capped at 128 MiB; restore verifies that
   container before a private-stage rename.
8. the standard package has 12 module objects and 6 graph-owned tests; `lkjournal` has 3 module
   objects, 2 targets, and 11 graph-owned tests.

## Selections at the graph-1 baseline (historical)

Decisions are recorded only when selected implementation and tests make them durable.

- Vocabulary and command grammar: pending the executable-registry audit. Compatibility aliases
  will not survive the cutover.
- Bootstrap ownership: evaluate an integrity-checked embedded standard artifact generated from the
  maintained standard graph, plus graph-native normalized recipes applied through the sole change
  protocol. The runtime must not consult a mutable ambient artifact.
- Persistent root: pending two measured prototypes. The selected root must keep deterministic
  iteration and a simple full reconstruction oracle while making exact lookup and leaf updates
  page-local.
- Incremental validation: retain the existing full validator and packed reconstruction as explicit
  or differential oracles; accepted incremental facts must bind exact content and validator
  contracts.
- Abstraction: pending consumer-duplication audit. Closure capture is not presumed necessary.
- Stdio session: standalone correctness remains mandatory; retain a session only after an equal
  complete-workflow measurement.
- Physical packs: no selection has been made. One-file-per-object and immutable-pack prototypes
  must be compared before a pack dependency or format is adopted.

## Predecessor graph-3 cutover observation (historical)

The shared working tree was inspected again after the direct cutover. This observation is not a
commit-bound final performance receipt and does not replace the historical raw rows above.

- Current contracts are meaning graph 3, persistent root storage 2, CLI 3, change 2, transaction
  3, revision/receipt 3, draft 3, diff/merge 2, semantic summary/validator 2, executable/package
  artifact 4/3, backup 4, and bootstrap 2.
- The accepted root is a manifest over six canonical persistent maps: module ID, module name,
  dependency package ID, dependency alias, target ID, and typed tombstone.
- Imports bind exact package/module IDs. Targets bind exact module/component/port IDs. Module
  rename has a local path that does not rewrite importers or targets; declaration rename remains
  outside the local transaction classes and uses complete preparation.
- Content-addressed disposable module summaries and a revision-bound reverse-dependency index are
  persisted. Each revision authenticates the exact fact set with a semantic certificate. Missing
  cache bytes rebuild; a rebuilt certificate mismatch is corruption.
- Exactly three precondition-free classes prepare locally: eligible pure-body replacement,
  independent empty-module creation, and module rename. Other changes retain complete logical
  reconstruction, canonicalization, and validation. No general incremental compiler or
  delta-maintained broad query index is claimed.
- Direct CLI discovery reported 17 commands, 13 change forms, 16 type forms, 24 owner kinds, 14
  relation roles, and schema digest
  `74b6011e56c065f8ced1b80fa282d23fe7ee3992bad4016317d813ea4c9dc81d`.
- Standard is revision
  `rev_3fcc3b60df8b3f4fe0a0823ee71870ad2136c7003af585efc466b6e0ff8866e5`, package artifact
  `artifact_ca8a5bbfe3c4ffcb55600ecb696a514a15595b0e8cf63da72751bf81f5146a8d`, bundle digest
  `artifact_d0173dd79cec054a7b34febeab9a876c62117129f65f8a031d92d50fdf1196be`, and 7 passing
  graph tests.
- `lkjournal` is revision
  `rev_16056a4d1487ac5ac18c42449c9031a08a7670f97650125b9dd8602803810edc`, root package artifact
  `artifact_530f61981bfc4afece8f05abb6b0649b417e31def0efea6096aae58bc25471e6`, bundle digest
  `artifact_41939c3bcedcf02ec49da94ee4a27d9e012d52bbe2a5a3d8b7f593c983cceff7`, and 12 passing
  checks across its exact two-package closure.
- Graph-3 scale, service, RSS, I/O, fsync, and million-owner results were unavailable.
  No historical timing is relabeled as current evidence.

## Predecessor graph-4 exact-reference observation (historical)

This section records the working-tree identities after the exact declaration-reference cutover and
before revision contract 4. It is historical and not a commit-bound final performance receipt.

- Current contracts are meaning graph 4, persistent root storage 2, CLI 4, change 3,
  transaction 4, revision/receipt 3, draft 4, diff/merge 2, semantic summary/validator 2,
  executable/package artifact 4/3, backup 4, and bootstrap 2.
- Imports bind exact package/module IDs; exports bind declaration IDs; types, interfaces, direct
  calls, named function values, records, variants, and constants bind exact
  package/module/declaration references. `Variable` is lexical-only and `Constant` is a distinct
  expression form.
- Four precondition-free transaction classes prepare locally: eligible pure-body replacement,
  independent empty-module creation, module rename, and declaration rename. Structurally different
  body replacement records removed nested identities as tombstone-map deltas. Exact-reference
  importers and callers are not rewritten on either rename.
- Local exact-index contract 3 stores revision-independent content-addressed owner/name shards and
  one revision/root-bound manifest with 256 digest slots per index. The four local profiles update
  touched buckets and reuse other shard digests; body-only differential coverage observes zero new
  exact-index objects. Initial and complete-candidate publications seed from already available
  graph values. Missing/corrupt shards and predecessor v2 manifests rebuild; the broad relation
  index remains lazy and is not delta-maintained.
- Direct CLI discovery reports schema digest
  `1980273fe10405fbf7aa7940c607af819c1261bd8b89019243326da31841df6c` and documents request-local,
  local declaration-ID, and exact package/module/declaration selector forms.
- Standard is revision
  `rev_af36c21e869a22a992b982aafe959c6230311293094e9ded162e29872ce0afdf`, root
  `root_object_61f185e6332b885353acf6312c779369bcca9ca82acc5141b9beb4bcc2e1aeeb`, package artifact
  `artifact_cef17b4730c708a9e3dfdaa934af28fad58902fb011db1e1305fd840f459c57a`, bundle digest
  `artifact_b2f39efc64b987378a6abcb81ade2f14de354ace122dbea22f02a984de875cea`, 22,259 bundle bytes,
  and 7 passing graph tests.
- `lkjournal` is revision
  `rev_583079ff88a142c5a8553bb7fd3beffeda8e7d181651370cb6322819eb9f5dfc`, root
  `root_object_1d2e3529202868c508db87b475ac41b31d6241a01c791ac1db47fb0b1a4e7090`, root package artifact
  `artifact_231583fc727b1ce12854227f2031ed62332ef94eb7b7c6dfe58487047c94dcfd`, bundle digest
  `artifact_3ea0c5e71f763319514a6747e580b02c02efbf7e35086420b9bfed74e3cd0444`, 178,756 bundle bytes,
  and 12 passing checks across its exact two-package closure.
- Focused graph-4 checks currently pass for maintained repository status, graph checks, bytecode /
  reference equality, deep doctor, copied-binary change, and a 256-importer declaration-rename
  differential case. After the exact-index v3 slice, `cargo test --locked --lib` passed 114 tests,
  `cargo test --locked --test public_cli -- --nocapture` passed 7, and
  `cargo clippy --locked --lib -- -D warnings` passed. A working-tree fresh full profile is recorded
  below. Final commit-bound and fresh-checkout profiles, the scale matrix, RSS, I/O, fsync, and
  million-owner results have not yet been run and are not claimed.

## Current revision-4 semantic-fact cutover observation

This section records the current working-tree identities after replacing the monolithic semantic
dependency generation. It is not yet a commit-bound final performance receipt.

- Current contracts are meaning graph 4, persistent-root storage 2, CLI 4, change 3,
  transaction 4, revision/receipt 4/3, draft 4, diff/merge 2, semantic-summary 2,
  semantic-fact 3, validator 2, executable/package artifact 4/3, backup 4, and bootstrap 2.
- Semantic-fact contract 3 stores module-to-summary bindings, graph-owned test owners, and flat
  typed reverse dependency edges in three path-compressed persistent Merkle maps. A constant-size
  certificate over their roots replaces the predecessor packed complete reverse index. Local
  changes use key-sorted batched edits and retain only new reachable path pages.
- A bounded dependency frontier distinguishes unchanged, private-implementation, and
  public-signature deltas, traverses exact typed edge prefixes, selects dependent test modules,
  accounts page/byte work, and rejects stale or exhausted traversal. The general transaction
  validator does not yet consume this frontier, so only the existing four local profiles are
  claimed.
- Focused tests cover 10,000-module full/delta root equality with fewer than 16 new pages,
  dependency retargeting, test-owner replacement, private/public propagation, stale revision,
  budget exhaustion, missing/corrupt derived fact-page rebuilding, and predecessor contract
  rejection. `cargo check --locked --all-targets` and the focused semantic-fact test set pass.
- Standard is revision
  `rev_1af582dbebc01b43cd1050349f208b7c71c92ca4efd3f6b65624745f7d9c988e`, root
  `root_object_61f185e6332b885353acf6312c779369bcca9ca82acc5141b9beb4bcc2e1aeeb`, package artifact
  `artifact_6ea73654d153ac4410ff4aaad329373dce27a58bb0d8c61eaa31cd6d66bcb3f6`, bundle digest
  `artifact_3648f87daea0164ef6e94ea6e731dd687db590b8889583f63cac6587f5e7a4d1`, 22,264 bundle bytes,
  and 7 passing graph tests.
- `lkjournal` is revision
  `rev_eb60847c2ebc2098c65a3e425398fb63ae74e08f47cdda3067069acacea7fa90`, root
  `root_object_f67b6e91af36e61f306ca80b315a82e1ffdceb36227be21bb554df6903d786f1`, root package artifact
  `artifact_55c3b229f8cbdd53fb153e0859375404df5e31f66f6128736f5d8f95f71dfe98`, bundle digest
  `artifact_fd1b07fbf5caafc92499eead7077f2ffe638bbf1a8c48f154eb9a09fcc3bf78d`, 178,766 bundle bytes,
  and 12 passing checks across its exact two-package closure.
- The predecessor maintained store directories were moved to
  `/tmp/lkjscript-revision4-cutover-old-20260822` before direct replacement. This is recoverable
  local scratch, not retained repository authority. No revision-3 reader remains in current code.
- Updated 10,000/100,000/million-owner public scale, RSS, I/O, fsync, dense-fanout, and long-history
  evidence remains unavailable and is not claimed.

## Verification notes

The initially attempted command `tools/check --profile full --machine` failed at argument parsing;
the executable's discovered grammar is `tools/check full --machine`. This was a harness invocation
error, not a product gate result. The corrected baseline profile passed 15/15 gates in 9.519
seconds at `.artifacts/check/20260822T091825.896930Z-501228/receipt.json`. Cargo outputs were warm;
the checker reports the run as fresh and performs no cross-run gate reuse.

The first graph-4 fresh full DAG attempt exposed a verifier dependency defect: `workspace_tests`
executed a copied debug binary while the parallel Clippy Cargo process still had that executable
open for replacement. Linux rejected the launch with `ETXTBSY`; 16 other gates passed. The failed
receipt is `.artifacts/check/20260822T131438.189833Z-722356/receipt.json`. The DAG now makes Clippy
depend on formatting, workspace tests depend on Clippy, the focused public-CLI gate depend on the
other debug test gates, and the release build depend on formatting. Checker self-test retains this
dependency contract. One later run showed that the release Cargo process also had to complete
before workspace tests could safely launch copied executables, so workspace tests now depend on
both Clippy and release build. Six consecutive fresh full repetitions then passed. The first
corrected fresh 17/17 profile passed with no reuse in 6.078 seconds at
`.artifacts/check/20260822T131807.529974Z-726352/receipt.json`; a later final receipt will supersede
this working-tree evidence.

## Persistent-root publication locality correction

A post-index audit found that `StoredGraphRoot::apply_delta` path-copied bounded map paths but then
used the exhaustive reachable-page copier on every final map. That cleanup traversal decoded the
complete persistent root merely to discard intermediate generated pages, so ordinary local root
publication was still linear in total root pages. The corrected overlay retains every generated
page, including exact physical reuse, and final extraction walks generated pages reachable from
changed map roots only. Unchanged accepted-base subtrees are structurally reused under the exact
base and exclusive publication lock; deep doctor remains the exhaustive corruption route.

The focused test
`platform::graph::tests::local_delta_page_reads_do_not_scan_a_large_unchanged_root` constructs
10,000- and 100,000-module roots, counts physical base reads including overlay write probes,
requires bounded page/byte reads and retained pages across the tenfold size increase, proves exact
equality with full rebuilding, and reconstructs from base plus retained pages. It passed in 4.62
seconds on the warm debug worktree. The interrupted-orphan regression
`reused_physical_ancestor_still_retains_new_staged_descendants` and parent/child edge corruption
test `reachable_copies_reject_parent_child_prefix_mismatch` also passed. All-target Clippy with
warnings denied passed after the correction. A fresh full profile passed 17/17 gates with no reuse
in 9.956 seconds at `.artifacts/check/20260822T141831.947091Z-808336/receipt.json`. This is
in-process property evidence, not million-owner evidence.

The repaired public scale harness then completed a 10,000-background-module release workflow. The
raw receipt and exact binary/source/environment bindings are retained in
`docs/evidence/20260822-graph4-scale-10000.json`. The initial 10,000-module transaction took 44.913
seconds. A subsequent one-module creation and rename retained their one-module validation profiles
and completed in 22.781 ms and 45.653 ms, but wrote 842,666 and 843,519 derived-index bytes. Backup
took 21.216 seconds for 3,053,899 payload bytes. These observations select monolithic semantic-index
storage and bulk persistent-map mutation as prerequisites; 100,000 and one-million public runs were
not attempted on the known linear path.

## Semantic-fact cutover verification

After the direct revision-4 authority and semantic-fact contract-3 cutover, the release executable
opened both maintained stores, standard passed 7/7 graph tests, `lkjournal` passed 12/12 tests across
its exact dependency closure, and both execution tiers agreed. Deep doctor passed both stores.
Fresh release builds reproduced `applications/lkjournal/dependencies/standard.lkja` and
`applications/lkjournal/lkjournal.lkja` byte-for-byte. The copied executable reported the exact
built-in standard identities recorded above.

`cargo test --locked --lib --no-fail-fast` passed 122 tests, the public CLI suite passed 7 tests,
and all-target Clippy passed with warnings denied. The working-tree authoritative DAG then passed
17/17 fresh gates with no reuse in 116.312 seconds at
`.artifacts/check/20260822T152519.342035Z-853126/receipt.json`. This is pre-commit evidence; final
commit-bound and fresh-checkout receipts remain required.
