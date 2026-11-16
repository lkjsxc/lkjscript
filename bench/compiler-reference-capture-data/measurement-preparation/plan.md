# Conditional reference capture: prospective ordinary-build comparison

**Preparation only; non-executable.** Candidate B and the new generated input are not frozen. No setup or measurement is authorized by these files. Separate root releases are required for setup and the later exact comparison.

## Question and finite population

Measure whole ordinary-build wall time, user+system CPU and peak RSS for exact baseline A versus the separately validated conditional-reference-capture candidate B. Use the same immutable input, explicit source/library/runtime roots, `--no-store`, `--inline-limit 0` and `--stats` on both. Full capture/export parity and checkpoint promotion belong to the correctness suites.

Two inputs, in fixed order: **new-100k**, then **compiler**. For each, run **A1, B1, B2, A2, A3, B3**. This gives twelve measured builds at most, one worker, no warm-up, retry, added seed, profile, observer, index, edit or 1M command. Operating-system caches are not reset. This is a no-store comparison, not a cold-file-cache claim.

## Exact retained baseline and compiler input

A is SHA-256 `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`, 32,584,542 bytes. It was built from `d461c2d98e93aeed6061508ee80ab36b9eeaa04f` with producer inline64. Its src/library trees are `c710586b4d1b7ab476fd179a7d2c699477a2054a` / `25bdb8ff8567b941790d3951c78e896b23e26e8e`. The live target matched these exact bytes during preparation request252.

If the live copy changes before setup, recover A from `7d0731e99aeaf241c4334fc5ff065a341fb388c8:seed/lkjscript.gz`: compressed SHA-256 `505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2`, 6,547,345 bytes. During authorized setup verify both compressed and decompressed identities. Do not rebuild or substitute another producer.

The compiler input is existing `target/compiler-reference-capture-preparation-1/source.tar`, SHA-256 `79aa24a9eb6f8e7940b4424fc20839759a79dd8d480b1b746f7e29b17171d806`, 10,741,760 bytes. Its 1,742 src/library/tests regular members come from `c887b7c5aca7bb80d70ca26d7dfdf1a44213a163`; src/library equal A's trees, and tests is `e2b30ac0f2fa8f897f366293189d6af552a41980`. Authorized setup must verify all member paths, types, sizes and digests against the pinned Git trees before freezing the resulting inventory.

Use that archive's physical library for both inputs, with explicit runtime `library/runtime.lkj`, SHA-256 `eeeb173d23c597d7b666aca949a32ba171fa5407be4173ad4626abe1ad295d14`. Keep identical physical source/library/runtime selectors within every A/B comparison.

B is pending. Its separate addendum must pin exact binary bytes, source/library trees, construction command/policy, reviewed delta, focused/core/query results, full-gate result and actual CI status. Construct B with established producer inline64, matching A's policy; measured outputs stay inline0. Do not add scalar simplification or another transformation. Baseline CI is not validation of B.

Before setup, copy exact A into an owned frozen executable path and verify its bytes. Before comparison, likewise copy and verify B; never run the mutable main target as a measured producer. The release must bind those two owned paths.

## New 100k setup: freeze before A/B

The historical scale/container generated sources were cleaned. Manifest `315f2c153190f8f1f0b12fa0bd8d4f0200f0b17c407f2e018617f278f820794e` identifies their old bytes but cannot reconstruct them. Root therefore explicitly proposed a NEW deterministic generation, with a distinct identity; no old timing or output digest is imported as a result.

Use current `bench/scale.lkj` SHA-256 `557d6b124fe739f17c678c77b92e5f6259d87ebca69c2a56aca411071c5ff1e9`, blob `fe4346b63ff00f7688ec0c18f5329a5c649b39e8` at `0b0b27803d27b932a5b91cde2d766bd3c8bb82b9`. Use the existing shape at `bench/compiler-query-container-data/measurement-1/shape`, SHA-256 `0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c`, blob `9fe912c104951fde476ffe30e3642ad96a65878f`. It is the documented `0b30ca8987bd4ba343ab45ab01751b6721c7151a` compiler census: 83,992 lines, 3,364,828 bytes, 164 files. No fresh census or shape tuning.

Recover the compiler archive into a separate disposable factory tree. Replace only factory/src/main.lkj with the pinned generator bytes and retain its complete before/after inventory. The generator imports sibling ast/diagnostic/diagnostics/json/names/resolve/source/syntax modules, so its real dependency closure must be present. No tracked source is changed.

The actual setup API, with R denoting a future frozen owned run root, is:

```text
A build src/main.lkj --source-dir R/factory --library-dir R/factory/library
  --runtime-source R/factory/library/runtime.lkj --inline-limit 0 --no-store
  -o R/generator
R/generator generate R/generated-100k 100000 20261116
  stdin = exact pinned shape file
```

Build cwd is R/factory; generation writes to a freshly empty owned directory. There is one generator construction and one generation, each bounded as below. Record their exact command/cwd/status/output/timing/load separately from measured builds. The positional API reads shape on stdin; no invented --shape flag is used.

Verify the reported seed/100,000 lines, independently count .lkj lines, and inventory every regular member including workload.txt. Reject unsafe paths or special members. Freeze the complete new source in a deterministic archive with a path/size/hash inventory; verify every archived member against the generated bytes. Record the generator image, factory manifest, archive hash/size and actual shape. Retain and commit both complete input archives before A/B release. No source mutation occurs during A/B. Any historical-manifest agreement is reported separately without revising old campaigns or claiming old bytes were recovered.

## Ordinary build template and output contract

All placeholders remain non-executable until the release addendum supplies exact paths and ordered argv. Generated ENTRY is main.lkj; compiler ENTRY is src/main.lkj. Each cwd is its exact input root.

```text
/usr/bin/time -f '%e\t%U\t%S\t%M\t%x' -o PHASE.time
  timeout --kill-after=10s 900s prlimit --as=4294967296 --
  PRODUCER build ENTRY --source-dir INPUT
  --library-dir R/compiler-input/library
  --runtime-source R/compiler-input/library/runtime.lkj
  --no-store --inline-limit 0 --stats -o OUTPUT
  stdin=/dev/null
```

Use one fixed OUTPUT pathname per input; preserve its completed bytes under each phase label before reuse. Within an input only the exact producer path changes. No store or operation key is created. Set/record named nonsecret settings such as LC_ALL=C and TZ=UTC; do not dump the environment.

The first successful A1 image for each input is its explicit output reference. Every later A/B output must compare byte-for-byte with it; retain all images, sizes and digests. The current physical input selectors and inline0 output policy do not match an established seed recipe. Do not substitute A's own executable digest or the unrelated frozen26bf seed. Equal emitted outputs are evidence for this computation, not a bootstrap fixed-point or generated-runtime milestone.

## Bounds and raw records

One sequential compute worker. Every setup/generator/build command has a 4,294,967,296-byte address-space limit, 900-second timeout and 10-second kill grace. Require 3,221,225,472 free bytes before setup and each command. These are fixed experimental bounds, not language policy or inferred unconstrained requirements. Prior below-1M runs fit below about 1.3 GiB observed RSS; RSS does not measure virtual-address-space demand.

Retain actual argv/cwd, wrapper/GNU status and signal/timeout evidence, UTC start/end, outer nanosecond timestamps, load1/load5/load15 before/after, stdout/stderr, GNU wall/user/system/RSS, complete compiler statistics, input/producer pins and each output identity. Record load beside every reported duration. Hashing, output comparison, archiving and record processing stay outside the primary GNU interval. Keep outer launch/capture wall separately and do not mix it into GNU ratios.

Verify source/producer identities before and after the sequence. Require no restored/stored work where the corresponding reuse counters are emitted; preserve missing-field discrepancies instead of treating them as zero. No optional missing field supports a performance conclusion.

## Stop and interpretation

Stop the entire cohort on the first setup failure, pin mismatch, incomplete capture, nonzero command status, timeout/signal, resource refusal, invalid source count, malformed required record or image mismatch. Preserve failure/partial artifacts and mark every remaining row not-run with that reason. No retries, alternate input, larger limits, tuning, conditional 1M check or expansion.

Keep every completed observation and outlier. Separately for each input, report all three paired wall, user+system CPU and peak-RSS rows; give arithmetic means and B/A ratios, with medians/ranges as descriptive summaries. An incomplete sequence yields individual results, not a complete-cohort ratio. Do not pool the two inputs.

There is NO 10% adoption threshold: the generated-code optimization rule does not govern this frontend work-avoidance change. Peak-RSS differences are observed process peaks, not an allocation census or statistical significance. The controlled exact-binary comparison can support an association with the reviewed change; it cannot assign every byte to those two vectors or diagnose the earlier 1M R0008. Scheduling, file-cache state and producer image differences remain limitations. Alternating order does not eliminate them. No exponent or 1M claim follows.

## Release sequence

1. Root accepts the plan and separately releases the two-command setup.
2. Setup succeeds once; complete inputs and generator/factory controls are archived, member-verified and durably committed.
3. B is validated and frozen with its exact reviewed delta, construction and actual gate/CI status.
4. An immutable addendum binds actual paths, archives/manifests, A/B binaries, tool identities, twelve ordered argv and one launch command; all pending fields are rejected.
5. Root explicitly releases that exact comparison once. Keep artifacts until independent acceptance; cleanup is a later separately scoped action.
