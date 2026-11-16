# Actual producer pin for the separate derived-origin replay

This addendum binds the unchanged preparation in commit
`9720996d6600eff04e633733c977ab58c2d6148c` to the now-gated canonical
producer and the completed current50 prerequisite. It is **preparation only**:
no historical command has run, and the root must separately review this pin
and release one worker before the exact launch below.

## Producer, source, gate and seed

| Identity | Exact value |
| --- | --- |
| Canonical binary path | `/home/coder/workspace/lkjscript/target/lkjscript` |
| Executable SHA-256 | `34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377` |
| Executable bytes | 32,196,802 |
| Producer source / gated commit | `62fa76f4f863a169b906094f0dd9998c8421e77a` |
| src tree | `00d1109fcdc49041d13d7406ca74c8f5b4394e8d` |
| library tree | `25bdb8ff8567b941790d3951c78e896b23e26e8e` |
| Seed refresh commit | `72a259e8889cff6e7c4ac712d7940b77343a9f5b` |
| Completed CI run, reported and inspected by root | `43853670261` |
| CI head | `535c4db0746c95742a44c83222f7de81fdd2243e` |

Read-only checks verified the live canonical binary size/hash and that the
seed from the named refresh commit decompresses to that same hash.
The CI head has the same src/library trees as the gated producer source.
The root reported both CI gate and witness jobs successful; this preparation
does not launch CI or a compiler.

The retained local gate completed with status zero:
**606.794 s**, load **1.15 -> 3.70**, four workers and **5,448 fresh case
requests / zero case reuse**. GNU time separately records 606.79 s.
The source-to-executable association is the root's gated build/fixed-point
record, not an unsigned binary self-description.

Exact gate evidence was copied without modification from
`/home/coder/workspace/lkjscript/target/compiler-origin-container-gate.uQZd6X/`
into `compiler-strict-derived.producer-evidence/`:

| File | SHA-256 |
| --- | --- |
| gate.log | `2aeaa17467a5fe6019efac9073b79e0d4f8520e9d5e88c87a5ed615dd53cedf7` |
| gate.meta | `e14540a9663da219dc251046bb6c9298cb6adca2a78ffb8b0712f44694001244` |
| gate.time | `75886f80cd67c61e98fd835c19f2271a0ba3894f81efe6482378091aecde052f` |
| gate.status | `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa` |

The gate metadata's earlier ab2 producer and old seed are its starting inputs;
its final producer is the 34491766 identity pinned above.

## Completed current50 prerequisite

The checkpoint continuation was frozen in commit
`fc0147a6ceef94a3aea9eee6729205130b690393`, file
`bench/compiler-query-current-continuation-62fa76f4.json`, SHA-256
`05c2206cfbb2d27f69842e02076ad94e1c641b6bf1d785cb7eced855bbd9241b`.

Its stable retained outcome directory is:

```text
/home/coder/workspace/lkjscript-compiler-query-container-20261116/target/compiler-query-current-continuation-62fa76f4
```

The original setup stopped at offline validation before any query. The
separately released continuation performed all fifty timed commands for the
first time, with no repeated builds. The checkpoint records this pause and
its cache/load limitation; it is not an uninterrupted initial harness run.
This addendum preserves that distinction and makes no causal comparison to
the older v2 cohort.

The semantic agent reported completion and the root independently accepted
the captured outcomes. Read-only checks here verified all fifty TSV statuses,
protocol flags, aggregate counts, final completion markers and producer pins.
All fifty recorded responses are protocol-valid and below 100 ms in the
declared cohort; four bounded continuations remain unconsumed. This is a
prerequisite observation, not a historical replay result.

Exact copies are retained beside the gate evidence:

| Copied file | Original outcome / SHA-256 |
| --- | --- |
| current50-checkpoint.json | checkpoint from fc0147a6 / `05c2206cfbb2d27f69842e02076ad94e1c641b6bf1d785cb7eced855bbd9241b` |
| current50-queries.tsv | queries.tsv / `21dd0f8bbe8d6806d58df6b7cb6306f4109ffbd59fa73d474b7b68710c23bea8` |
| current50-summary.json | summary.json / `c7aaf9418e51f8c8aa064f5a71a9b2d3201352f883b11bbc2d477a8b9af56a95` |
| current50-completion | completion / `4d96fd904ba119a08b1aed4ddfd2bc3a4590cfa34d52858103012c14f255dbd7` |
| current50-timed-completion | timed-completion / `af75fc0c2df473ac870f1438b1df17c4e5bfc878078a9f6b5887aa5788d3c8fa` |

The completed current50 snapshot is
`9ff914ad9f4ef15c9bc5c61cb5d0718abcfde00322243a7a52a73f6b08d3534e`,
source identity
`d01c7eb47d92c7aefc7490b83f9bbc05025f6197871a7f90c51dfcad1576bb2b`,
checked IR
`4613bd709ee1d9238dca31e51b25d091abcd21a77f018090fedb8db8032c0fa2`.
Its producer and indexed image both equal the canonical 34491766 bytes.
No query fixture, key or store is reused by the historical campaign.

## Frozen preparation and exact future invocation

The unchanged preparation manifest is
`compiler-strict-derived.prepared.sha256`, SHA-256
`9cc4f036872314546636368636c9501c83f74ad796204ff22f4a4b71ea4b72c9`
(35 entries). Its exact harness remains
`ed641768023b0b0fedfcc9b07a2d7116d8d18ecd01a9c0d89e2474ac2cbdd614`.
Neither the plan, one-row population, auditors, old 0/6 cohort nor scoped 5/6
cohort has changed.

The eight-line producer record has SHA-256
`526384984d2fa54a435fe8af557575d9317e458e67ff45f83af4a1271d35fbe7`.
The nine copied gate/current50 records are bound by
`compiler-strict-derived.producer-evidence.sha256`, SHA-256
`c3254aa10bf343ccf9115126ca7bab000ed8cadec185f3c655322ac75849a548`.
The static producer/checkpoint checks are retained in
`compiler-strict-derived.producer-check.log`, SHA-256
`987dd10704724cab1a529bf029c5ecd1e0d282da14f1f4b386b5d245587a0692`.

Only after a separate explicit root execution release, from
`/home/coder/workspace/lkjscript-compiler-strict-derived-20261116`:

```sh
bench/compiler-strict-derived.sh /home/coder/workspace/lkjscript \
  /home/coder/workspace/lkjscript/target/lkjscript \
  34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377 \
  62fa76f4f863a169b906094f0dd9998c8421e77a \
  bench/compiler-strict-derived.tsv \
  > bench/compiler-strict-derived.launch.log \
  2> bench/compiler-strict-derived.launch.err
```

The harness will copy and rehash the producer before its one fixed case,
retain exact current source/library grants with the key outside all four,
and apply the frozen 4 GiB / 900 s and initially cold 2 GiB store limits.
It will not retry, replace the revision, alter historical sources, or reuse a
previous campaign's store. The old campaign totals and union of **29** remain
unchanged; **30 remains conditional on the actual new complete proof**.

The new `compiler-strict-derived.launch-ready.sha256` binds exactly the
unchanged preparation manifest, actual producer record and this addendum.
It does not substitute for the still-required root execution release.
