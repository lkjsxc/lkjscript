# Explicit-source replay: gated producer pin

This addendum completes the producer pin for the separate protocol in
`compiler-strict-scoped.plan.md`, prepared at
`546fe5165e5c6600877960f5c8e81dd6d43c485a`.
The producer-independent preparation manifest is SHA-256
`d293390427d42d63353562970bdddd5c29b669ce30173cccaa91456a2499858c`.

**Replay has not started.** At this freeze the full cold gate has passed, but
new source-scope CI and the root session's explicit replay-worker release are
still pending. The manifest binds the future invocation; it does not grant
permission to launch before that separate release. Keep this historical state
and record the later release/CI evidence alongside the actual campaign.

## Exact executable and source association

| Field | Frozen value |
| --- | --- |
| Canonical executable | `/home/coder/workspace/lkjscript/target/lkjscript` |
| Executable SHA-256 | `ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504` |
| Executable bytes | 31,724,996 |
| Producer source commit | `830896f51b449532e99c203a6531844fa399fb50` |
| Successful gate commit | `830896f51b449532e99c203a6531844fa399fb50` |
| `src` Git tree | `0ef9f66881dfaf2ee0f00bc945b5e664bf6d1a49` |
| `library` Git tree | `25bdb8ff8567b941790d3951c78e896b23e26e8e` |
| Eight-record pin SHA-256 | `80517ffcbd1d761fc12d311276b59da907aba76d5c18892f37724b26c919b771` |

The root supplied these values after its canonical full-gate bootstrap and
fixed point. The preparation independently rehashed the actual executable,
checked its size, resolved both source trees, and verified the gate-log hash.
These read-only checks are retained in `compiler-strict-scoped.producer-check.log`,
SHA-256 `2a64ece89c186ddd46153f4ce941b34d0c74a919b3484f55105c13afb15eb8e5`.
No compiler or replay was executed by those checks.

The source/executable association is the root's gated-build provenance.
At eventual launch the harness must still recheck the origin's exact bytes,
copy it into its newly owned fixture, verify the copy, and execute only that
copy. A changed root target fails the pin check; it cannot silently select a
different producer.

## Gate evidence

The root's four-worker cold gate passed in **657.932 s**, load
**6.68/6.06/6.12 → 9.29/8.70/7.56**, with 5,448 fresh case requests,
zero case reuse and the 128-mutant gate. GNU time separately records
657.93 s wall, 1,629.96 s user, 34.69 s system and 1,213,464 KiB peak RSS,
exit zero. These are gate observations, not replay measurements.

The original gate artifacts are under
`/home/coder/workspace/lkjscript/target/`:

| Artifact | SHA-256 |
| --- | --- |
| `compiler-replay-scope-cold-gate.log` | `aff46525d057e1195fa3d5fb2dda2d324d5a2884ab791069d485451f0294ed46` |
| `compiler-replay-scope-cold-gate.meta` | `da8ceaf3914b9526aa901dad9940b55d5043ea05d9926b2229388e8e113c21a9` |
| `compiler-replay-scope-cold-gate.time` | `85dc8bd5e5595e891a224a3b9532936a25e67e9d6d697ef1adc4917ccb74a17f` |

The retained producer-check log includes their hashes, the complete gate
metadata/time records, and the final gate summary. It preserves the distinction
between the old producer present before bootstrap and the pinned canonical
producer produced by the gate.

## Bound invocation

After the separate explicit release, the single permitted campaign invocation
uses these actual pins:

```sh
bench/compiler-strict-scoped.sh /home/coder/workspace/lkjscript \
  /home/coder/workspace/lkjscript/target/lkjscript \
  ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504 \
  830896f51b449532e99c203a6531844fa399fb50 \
  bench/compiler-strict-scoped.tsv \
  > bench/compiler-strict-scoped.launch.log \
  2> bench/compiler-strict-scoped.launch.err
```

The exact six cases, order, changed-language coverage, 4 GiB/900 s limits and
initially cold shared 2 GiB store remain as predeclared. Each command now has
the explicit desired source grant. No claim of a replay pass is made by this
pinning step; the original campaign remains 0/6 and the prior strict union
remains 24.
