# Six recent strict replay cases: six retained refusals

The fixed v2 campaign attempted all six selected cases once. **All six refused
with `E_REPLAY_FILES`; none published a transaction or produced a
desired/candidate checked-meaning equality proof.** There are zero new strict
passes. The established count remains **24** at the named
`6a187b94fb763f7b0565436a98ab5feda3cd642f` 300-commit horizon.

The harness exited 0 because it completed its declared observation sequence.
Every compiler command exited 1. The result table retains all six refusals;
the proofs TSV contains only its header. No result has been replaced, retried,
relabelled as success, or merged into the previous four-case experiment.

## Executed protocol and provenance

The executed preparation is commit
`33a1933edecbbde6cd9588204c9593bd652d6ad3`, with v2 manifest SHA-256
`a353d55e82519b2c79e82e989408cf3ff5fcfc6bc46ab5647fadcc2071b85cb6`.
The original unexecuted preparation `803c7367` and its manifest remain
historical evidence. The prelaunch amendment removed the absolute desired-path
grant of `/`; no command used that superseded form.

The copied executing producer was 31,716,730 bytes, SHA-256
`adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7`,
with gated source provenance
`4f8c213559b7de96b3b6bd6199da661dfc95f778`,
source tree `5ae1f2cdd58843b421bcac42368d8efdb1a9a416`, and library tree
`25bdb8ff8567b941790d3951c78e896b23e26e8e`. Root explicitly released the
worker after the successful full gate; the copied producer's exact bytes were
checked before launch and by the retained-record auditor.

Every case used its exact first-parent and target archives, explicit physical
historical libraries, the same relative source entry, and the exact
`CASE/after` working directory:

```text
cwd = CASE/after
COPIED_PRODUCER edit src/main.lkj
  --source-dir CASE/before --against src/main.lkj
  --library-dir CASE/before/library
  --against-library-dir CASE/after/library
  --inline-limit 0
  --store FIXTURE/store --store-bytes 2147483648
  --store-key FIXTURE/key --stats
```

The actual argument vectors, requested/physical cwd, and canonical four-root/key
grant tables are retained for every invocation. The key was outside both source
and both library roots. One initially empty 2 GiB store was shared across the
six commands; filesystem caches were not reset. These are sequential
correctness observations with recorded costs, not independent cold-build samples.

The fixed worker ceiling was one, with an external 4 GiB address-space limit
and 900 s deadline per command. No limit, timeout, process, or allocation refusal
occurred. No recovery command ran. All six private process-group cleanup records
are 0. The measured compiler worker has finished.

## Every observation

All rows below have outcome `refused / exit 1 / E_REPLAY_FILES`, zero
operations, and zero stdout lines. Load columns list the 1-, 5-, and 15-minute
averages immediately before and after that command.

| Case / revision | GNU wall s | User / kernel s | Peak RSS KiB | Load before, 1 / 5 / 15 | Load after, 1 / 5 / 15 |
| --- | ---: | ---: | ---: | --- | --- |
| 1 / `9047a677` | 42.19 | 40.98 / 0.47 | 1,107,996 | 2.18 / 4.62 / 6.64 | 2.30 / 4.33 / 6.46 |
| 2 / `de65c09b` | 44.62 | 43.38 / 0.52 | 1,100,644 | 2.84 / 4.41 / 6.47 | 3.74 / 4.44 / 6.39 |
| 3 / `0a410bad` | 45.93 | 44.59 / 0.54 | 1,103,184 | 3.74 / 4.44 / 6.39 | 5.21 / 4.69 / 6.37 |
| 4 / `c5f67a55` | 46.19 | 44.39 / 0.55 | 1,120,048 | 5.21 / 4.69 / 6.37 | 6.16 / 5.05 / 6.42 |
| 5 / `ba27e90a` | 43.85 | 42.19 / 0.49 | 1,149,400 | 6.16 / 5.05 / 6.42 | 4.23 / 4.70 / 6.23 |
| 6 / `71e821df` | 42.78 | 41.53 / 0.40 | 1,175,340 | 4.23 / 4.70 / 6.23 | 3.34 / 4.38 / 6.06 |

The six recorded subprocess wall times sum to **265.56 s**, with
**257.06 s user + 2.97 s kernel CPU**. Each sum is derived from these six rows;
there is no successful-replay cost or speedup observation. Launch-inclusive
start/end timestamps additionally retain wrapper and bounded process-group
cleanup overhead. The GNU measurements, timestamps, loads, and raw internal
stats remain separate.

Peak RSS ranged from 1,100,644 to 1,175,340 KiB. The 4 GiB limit was an
address-space ceiling, not an RSS allocation target or a claim about an
unconstrained requirement.

## Exact refusal and source alias evidence

Every refusal names its own absolute before-tree path:

```text
CASE/before/library/runtime.lkj
E_REPLAY_FILES source file has no stable identity or unambiguous relative-path counterpart
```

The six complete quoted identities, rule, and detail are retained in
`compiler-strict-recent.refusals.tsv` and the original stderr files.

Both sides of every pair contain the same expression in
`src/embedded.lkj`:

```lkjscript
Some(include_str!("../library/runtime.lkj").to_string())
```

All twelve physical runtime files have SHA-256
`eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14`.
The runtime-inputs TSV records each pair and its exact source expression.
Thus this particular refusal does not report changed runtime data.

The pinned-code trace explains the path-spelling limitation. Before source
diagnostics carry the explicit absolute before prefix. Desired source reads
through `.` at the after cwd and retain relative diagnostic paths. Explicit
standard-library items carry the separate absolute library prefix.
`structure_replay::file_pairs_allowed` pairs stable items/modules and maps
include-only files through the longest known directory correspondence. The
absolute standard-library correspondence therefore does not identify the
relative desired source-include spelling of this runtime input. It refuses
before deriving an applicable transaction.

That explanation is an inference from the captured selectors, exact archive
bytes, and pinned production code; these failed commands did not emit a catalog
dump. It is not a measured successful comparison or a reason to ignore the
missing counterpart. A general explicit desired-source Directory selection can
address this API boundary without filename guessing or a root grant. Any
changed-producer/API campaign must be separately frozen and retain this entire
refusal cohort.

## Independent checks after the campaign

The frozen retained-record auditor verified:

- all six planned revisions were launched once in the declared order;
- each exact argv, physical cwd, source/library/key grant table, exit status,
  timing record, and complete changed-language path set;
- all 24 prior strict proofs remain inside the named window and disjoint from
  these cases;
- zero new transactions, operations, file proofs, or accepted meaning proofs.

A separate read-only archive-state audit compared every archived file to its
first-parent or target Git blob using SHA-256, and checked the complete path
sets under `src/`, `library/`, `tools/`, and `tests/`. All **22,202 file
instances across twelve archives** matched exactly. No pending publication
intent remained. These checks establish retained source bytes after refusal;
they do not turn an unchanged candidate into a successful replay.

The state-audit helper initially failed a Perl syntax check before it read any
archive; its lexical declaration was corrected. Its later checks read only
retained files and Git objects. They did not invoke the compiler or repeat any
timed command. The final helper, aggregate manifests, and logs are retained.

The acceptance boundary remains **complete changed-language coverage plus
selected-root canonical checked-meaning identity**, in domain
`lkjscript-checked-meaning-1`. This cohort contributes no such proof. It does
not establish raw serialized IR identity, all-root/configuration equivalence,
or reproduction of non-language Git effects. There is no new target-text
equality result from an applied transaction.

## Durable records and retained fixture

The TSV, metadata, six full log sets, exact arguments, cwd/grant records,
refusal table, archive and runtime digests, header-only proofs TSV, audit logs,
and read-only audit helper are covered by
`compiler-strict-recent.results.sha256`. The original preparation files and
all previous history experiments remain unchanged.

For independent review the complete fixture is still retained at:

```text
/home/coder/workspace/lkjscript-compiler-strict-recent-20261116/target/compiler-strict-recent.TAz4Df
```

It contains only this campaign's copied producer, private key, store, and exact
case archives. No compiler or audit job is active. Cleanup awaits root's evidence
acceptance and uses the already reviewed guarded cleanup path. Textual evidence
will remain after cleanup; no further execution is authorized by this report.
