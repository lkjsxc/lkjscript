# Six strict replays with explicit desired source grants: five passes

The fixed scoped campaign launched all six selected revisions once. **Five passed and one refused**. The five passes reproduce the selected root's canonical checked meaning while covering every language file changed by those commits. They add **45 operations and seven required file facts**.

At the frozen `6a187b94fb763f7b0565436a98ab5feda3cd642f` last-300-commit horizon, the unique audited union is **29 cases: 24 prior proofs plus five new proofs**. The thirty-case stopping condition remains unmet. The earlier six-case campaign remains **0/6**, and the separate four-case experiment remains unchanged. No case was replaced, retried, adapted or resumed.

The harness exited zero because it completed all six declared observations. Five compiler commands exited zero; case 3 exited one with `E_EDIT_SYNTAX`. Its empty stdout and unchanged archives remain evidence of a refusal, without an applied transaction or checked-meaning equality proof.

## Frozen protocol and producer

The preparation is commit `546fe5165e5c6600877960f5c8e81dd6d43c485a`; the producer addendum is `53824dc9a179a9506302f1faa78ffcd79f35c941`. The launch manifest is `0a7191669e924d862b031e6f3f389372c3fe9562f0c0df557d2a60f2355f5fe6`, binding the unchanged preparation manifest, exact producer pin and addendum. The subsequent release/CI record is separate; none of the frozen files was rewritten after measurement.

The copied executing producer was **31,724,996 bytes**, SHA-256 `ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504`. Its gated source is `830896f51b449532e99c203a6531844fa399fb50`, with `src` tree `0ef9f66881dfaf2ee0f00bc945b5e664bf6d1a49` and `library` tree `25bdb8ff8567b941790d3951c78e896b23e26e8e`. The binary was copied and rehash-verified before launch and rechecked after all six commands.

Root released the worker after the full cold gate and both successful jobs of CI run **43831972036**, revision `e7afd108`. The release record retains the native gate's 1,156.617 s, load 0.53 to 2.42, 5,448 fresh/zero reused cases, and optional witness's 276 value/output cases and 344 verdicts, with both downloaded-log hashes. These are separately reported gate observations, not replay measurements.

The new API condition was explicit `--against-source-dir`, introduced in `b41d3746` and integrated as `f48f14ff`. Each case used this exact shape:

```text
cwd = CASE/after
COPIED_PRODUCER edit src/main.lkj
  --source-dir CASE/before
  --against src/main.lkj --against-source-dir CASE/after
  --library-dir CASE/before/library
  --against-library-dir CASE/after/library
  --inline-limit 0
  --store FIXTURE/store --store-bytes 2147483648
  --store-key FIXTURE/key --stats
```

CASE/FIXTURE paths were absolute and canonical; both entry arguments remained relative. Each retained NUL-delimited argument vector, requested/physical cwd and grant table was audited. The key stayed outside all four source/library grants. Both source roots had explicit narrow Directory grants.

Every before/after source and library was its exact historical Git archive. There was no source adaptation, synthetic root, alternate configuration, omitted off-root language file or fallback library. No historical native compiler image was emitted or executed; there was no runtime selector. Both physical runtime inputs in every pair remained byte-identical, recorded in the runtime-inputs table.

One compiler worker, an external **4 GiB address-space ceiling** and **900-second deadline** applied to each command. One initially cold **2 GiB authenticated store** was shared in fixed case order, without filesystem-cache resets. These are sequential correctness observations with their costs, not independent cold-build samples or a speed comparison with the older producer/API condition. Free-space preflights passed before setup and every archive; the raw space table retains each available/reserved-byte calculation.

No resource/process refusal or recovery command occurred. Every private process-group drain returned zero, and no pending publication intent remained. All compiler workers from this campaign have finished.

## All six observations

Loads are the 1-, 5- and 15-minute averages immediately before and after each command. GNU time's wall/user/kernel/RSS values remain separate from retained launch-inclusive timestamp pairs and internal compiler stats.

| Case / revision | Outcome / operations | GNU wall s | User / kernel s | Peak RSS KiB | Load before, 1 / 5 / 15 | Load after, 1 / 5 / 15 |
| --- | --- | ---: | ---: | ---: | --- | --- |
| 1 / `9047a677` | passed / 2 | 70.95 | 68.49 / 0.50 | 1,308,592 | 7.63 / 8.72 / 8.89 | 6.30 / 8.22 / 8.71 |
| 2 / `de65c09b` | passed / 5 | 71.13 | 69.18 / 0.47 | 1,303,884 | 6.30 / 8.22 / 8.71 | 7.33 / 8.04 / 8.60 |
| 3 / `0a410bad` | refused / 0 | 47.74 | 46.31 / 0.47 | 1,153,684 | 7.33 / 8.04 / 8.60 | 6.77 / 7.78 / 8.49 |
| 4 / `c5f67a55` | passed / 21 | 74.94 | 72.69 / 0.69 | 1,286,132 | 6.77 / 7.78 / 8.49 | 8.35 / 8.22 / 8.60 |
| 5 / `ba27e90a` | passed / 3 | 66.52 | 65.06 / 0.55 | 1,333,040 | 8.35 / 8.22 / 8.60 | 6.09 / 7.58 / 8.35 |
| 6 / `71e821df` | passed / 14 | 67.52 | 65.53 / 0.54 | 1,348,812 | 6.09 / 7.58 / 8.35 | 6.15 / 7.33 / 8.20 |

These six subprocess wall values sum to **398.80 s**, with **387.26 s user + 3.22 s kernel CPU**. The sums include the refusal and exclude archive setup and post-run audits. Peak RSS ranged from 1,153,684 to 1,348,812 KiB. The external address-space ceiling does not establish an unconstrained memory requirement.

## Proof and complete changed-language coverage

The five successful commands reported applied transactions and equal desired and candidate hashes in domain `lkjscript-checked-meaning-1`. The full hashes and source-snapshot identities are in `compiler-strict-scoped.proofs.tsv`; each is tied to its original stdout/stderr SHA-256.

| Case | Every required changed-language path | Published file facts | Exact target text |
| --- | --- | ---: | --- |
| 1 | `src/semantic_facts.lkj` | 1 | 1 of 1 |
| 2 | `src/structure_edit.lkj`, `src/structure_write.lkj` | 2 | 2 of 2 |
| 3 | `src/ast.lkj`, `src/semantic.lkj`, `src/source.lkj`, `src/structure_edit.lkj`, `src/structure_replay.lkj` | 0; refused | no applied result |
| 4 | `src/structure_write.lkj` | 1 | 0 of 1 |
| 5 | `src/semantic_store.lkj` | 1 | 1 of 1 |
| 6 | `src/driver.lkj`, `src/store.lkj` | 2 | 1 of 2 |

The retained-record auditor compared the complete Git changed set under `src/` and `library/`, plus every changed `.lkj` or `.rs` path anywhere. All seven before hashes equal the exact first-parent blobs, and all seven after hashes equal actual published bytes. Five published files also match the historical target text exactly.

The two text differences are recorded separately in the per-case file tables and `compiler-strict-scoped.target-text.diff`. Case 4's published `structure_write.lkj` lacks five added leading documentation-comment lines; case 6's published `store.lkj` lacks two added leading comment lines. The frozen item-span projection does not attach those comments to their following declarations. Both cases still passed the required checked-meaning comparison. Exact target text was never an acceptance prerequisite.

The separate archive-state auditor verified **22,202 file instances across twelve archives**, complete path sets and no pending intents. Every desired archive exactly matches its Git revision. Every before archive matches its first parent with only its explicitly proven published file changes. Case 3 has no overrides: all 1,847 before and 1,848 desired files remain exactly their archived Git bytes. The runtime table independently records all six unchanged input pairs.

The claim remains **complete changed-language coverage plus selected-root canonical checked-meaning identity**. It does not establish raw serialized IR byte identity, equivalence of every root/configuration, or reproduction of non-language Git effects. The 24 prior proofs retain their own `4c4537ea…99e09` producer and frozen source windows. All 24 and these five are disjoint members of the named horizon; the old six-refusal cohort contributes zero.

## Case 3: derived source-origin mismatch

The exact final diagnostic was:

```text
"<ast::Program as std::prelude::Default>::default" E_EDIT_SYNTAX expected `{`, found `struct`
```

The following is a read-only trace of the pinned producer source and retained archives, not a second compiler observation or a new catalog dump.

`src/syntax/default.lkj::derive_default` assigns the defining struct's whole declaration span to its generated `default` function, its generated body block, and its generated implementation. `semantic::catalog_mode` publishes these generated items alongside the struct without a separate source-origin or editability classification. `semantic::append` extends each span over the same visibility and derive-attribute prefix.

`structure_replay::items` sorts by file, start, decreasing end and identity. These equal-span items therefore sort with `<ast::Program as std::prelude::Default>::default` first. The actual `ast::Program` struct and generated impl are subsequently skipped as contained. When the commit adds `loaded_modules: Vec<LoadedModule>`, the selected function's textual prefix and trailing frame are unchanged, so `same_frame` chooses `replace-body`. Its supposed body is the struct declaration.

The exact retained source offsets, derived statically from the archived `src/ast.lkj` bytes, are:

| Side | Attribute-extended item start | Struct declaration / generated block start | Exclusive end | Whole-file SHA-256 |
| --- | ---: | ---: | ---: | --- |
| before | 8,939 | 8,969 | 9,684 | `242272272934546ee162f147b4cfe5551d5e0627190c3a9febd2bef42cbfc183` |
| desired | 9,147 | 9,177 | 9,935 | `12cde633da8d5ac54bc71fe68b02017259a29ca3c44af23163565544464317bb` |

Both prefixes are exactly `#[derive(Clone, Default)]` followed by newline and `pub `. The extracted desired body begins `struct Program {`. `structure_edit::one_item` prepends `fn __lkjscript_transaction_body() `; the ordinary body parser then correctly expects an opening brace and refuses the `struct` token. No publication occurs.

The smallest real contract mismatch is treating a generated semantic identity as an independently authored source declaration. A future repair should preserve its semantic identity and relation to the owning declaration while selecting the actual authored struct for source edits. Merely changing the lexicographic tie-break or special-casing this method name would not establish source-origin evidence or protect other operations on the generated identity.

The same parser expansion creates the generated Default impl and generated block/field-call expressions with defining spans, for named, tuple and unit structs. These are known related provenance cases. The pinned parser's only additional AST function/impl insertion beyond ordinary declarations is this Default expansion. Other built-in derived helpers inspected in `src/elaborate/derive_methods.lkj` are emitted as IR `$builtin:*` functions; that is not evidence that Clone has the same source-catalog defect. No production repair or replay retry was made in this campaign.

## Durable evidence and completed cleanup

`compiler-strict-scoped.results.sha256` covers this report, the fixed preparation/pin/release records, all six original log sets, argument/cwd/grant records, timing and space tables, proof/file/archive/runtime/refusal tables, both auditors and their outputs, and the two text-only differences. The original `compiler-strict-recent.results.sha256` was reverified unchanged.

All measured source archives, the copied producer, private key and store were retained at this owned fixture for root's independent review:

```text
/home/coder/workspace/lkjscript-compiler-strict-recent-20261116/target/compiler-strict-scoped.8wKkOL
```

Root accepted the evidence after independently rerunning both frozen auditors against the live fixture and comparing the proof/archive/runtime/refusal tables byte for byte. The accepted report SHA-256 was `7969415775e7531799de0214fec6cee726e94852793f0007d1ee1066b5f10704`; the accepted results-manifest SHA-256 was `fad8d739aea17d845abe02c3d5e1b20e7dd54df1131363df4d2c8612f857a21b`.

Cleanup waited until the unrelated exclusive container timing campaign had drained. On 2026-11-16 at 09:37:02 UTC the pre-reviewed guarded cleanup removed only this fixture; absence was verified. Its before/after load observations were both 4.16 / 4.50 / 5.05. The copied compiler, private key, store and all case archives are gone. The cleanup records retain the actual result.

All original observations, arguments, diagnostics, source-state digests and proof/audit records remain unchanged. Their fixture paths are historical. The full live archive audits preceded removal. No compiler or audit job is active, and this report authorizes no further execution.
