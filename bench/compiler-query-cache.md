# Validated fact-page cache: forty compiler requests

**All forty selected whole commands finish below 100 ms in this follow-up
campaign.** Minimum / median / maximum launch-inclusive wall time is
**2.334 / 4.037 / 53.525 ms**. All forty subsequent old-reader validations
produce **byte-identical stdout**, including continuation tokens. The median
depth-one slice is **3,772 bytes**, with **2/20** continuations.

The [ten additional query-kind measurements](compiler-query-kinds.md) form a
separate depth-two series: all ten return nonempty, correctly labelled facts
below 100 ms, and all ten also match the old reader exactly. Neither series
replaces the original [v2 campaign](compiler-query-v2.md), including its
207.807 ms and 102.204 ms observations.

## Change and bounded attribution

Reader source commit `a98c749cd4e55145338dc0365cab751bd8c2ba5a`
adds a cache of successfully validated logical fact pages, including validated
absence, inside one reader with an immutable snapshot. A hit avoids repeating
the same radix traversal and decoding. Failed validation is not cached.
First access still follows the authenticated, content-addressed raw-page route.

The motivating source slice makes 266 logical page requests for 90 distinct
logical pages. A focused old/new-reader pair against **one identical stored
snapshot** retains 276 raw-page reads in both outputs and exact stdout parity:

| Reader | Wall ms | Internal total / query phase ms | User / kernel s | Peak RSS KiB | Load before → after, 1/5/15 min |
| --- | ---: | ---: | ---: | ---: | --- |
| Old `51708066` | 92.990485 | 89.855 / 89.768 | 0.08 / 0.00 | 8,064 | 6.29/5.34/4.83 → 6.29/5.34/4.83 |
| Cached `fac8db08` | 53.725724 | 51.018 / 50.934 | 0.04 / 0.00 | 8,044 | 6.29/5.34/4.83 → 6.29/5.34/4.83 |

This single focused pair observes a **42.22% wall reduction**, clearing the
10% retention criterion on the measured query workload. It does not assign
that gain to fewer authenticated raw pages, and it is not a whole-compiler
speedup or a replicated statistical estimate. Both answers have SHA-256
`fa1552a9303f9c92b2de8063bc4c8bd563f05bacfc2d6a19c63c3fe244dbe3fd`.

The old `driver::Memo` wall outlier remains unexplained. Its faster value in
this changed-reader campaign is not a diagnosis of that earlier observation.

## Producing compiler, reader and source are separate pins

| Identity | Value |
| --- | --- |
| Indexed source revision | `20d5f7e98fe4501d441b3ac357733a8a7ba942e7` |
| Indexed source / library Git trees | `5daafd1a48ef844ec00c4edc76c3e3d87bf41445` / `00d0571a0f4f36e2ea4a6d57a23d54afe360d763` |
| Snapshot-producing compiler SHA-256 | `51708066a1c150ba51800697751b887ad279792f8b4e4d4a57d5df3e291d934c` |
| Snapshot-producing compiler bytes | 20,606,806 |
| Query reader SHA-256 | `fac8db084310bc8168493cb040944d8e463652a1f680534c91c56a6d0da3bc1a` |
| Query reader bytes | 20,623,190 |
| Reader source commit / source tree | `a98c749cd4e55145338dc0365cab751bd8c2ba5a` / `d0721527a10975b311bc67135507944a0d9dcac2` |
| Source snapshot digest | `0ce5d020f8ba1b1c17f56ad7fc4e637bd3dd46e2d7cc7b9228271c6fedbbebf6` |
| Stored checked-IR digest | `0e7c945857c1904ed07f21967c0c1e14207f3ee3d3a2aa16cabb6461003ec180` |
| Authenticated snapshot | `cc171466d0b2b5ff3319737d33b7ff633bd572a749f0916fc7e4babfe3a85280` |

Every timed response reports the **old producing digest**, not the new reader
digest, together with the exact source, checked-IR and snapshot pins above.
It states `validation=authenticated-producing-compiler`,
`current-check=not-performed`, `freshness=not-asserted` and explicit selection.
All fifty timed requests report `producer_bytes_hashed=0`. Ordinary builds,
admission and proof reuse retain their separate producer-specific checks.

The indexed source revision matches the previous v2 campaign, but this
independently created fixture has a different physical source/library selector
path and different source/IR/snapshot identities. Every slice is eleven output
bytes longer because its item fact contains that longer absolute file path.
No latency ratio between the two forty-row campaigns is asserted. The focused
pair and all exact-stdout validations use one identical snapshot and selector.

## One index and the full warm sequence

The semantic-query worker created the sole fixture, with a fresh store and
private key. This benchmark reused it; it did **not** build a second index.
The indexed source came from the frozen revision above and explicitly selected
its archived physical library and runtime, with `--inline-limit 0`.

| Shared fixture operation | Wall / user / kernel s | Peak RSS KiB | Load before → after, 1/5/15 min |
| --- | ---: | ---: | --- |
| Cold indexed build | 132.088316245 / 120.66 / 4.76 | 2,770,148 | 5.88/4.50/4.52 → 6.29/5.34/4.83 |
| One unchanged indexed build | 0.592697837 / 0.44 / 0.01 | 34,320 | 6.29/5.34/4.83 → 6.29/5.34/4.83 |

The unchanged build returned the same snapshot with one reused build, five hits
and zero writes. The indexed artifact was 20,627,161 bytes; it is distinct from
both the producing executable and the cached reader. The index contains
12,438 IR functions and 1,197,212 instructions. Cold catalog construction was
21,125.774 ms and index publication 85,354.979 ms. These phases belong to the
cold row and its load window, not additional measurements. The fixture worker
used a 4 GiB external address-space ceiling and a 2 GiB store budget.

The exact sequence was:

1. One cold indexed build, then one unchanged indexed build.
2. One old-reader `source::load_report` slice, then one cached-reader slice,
   both at depth 1 / 96 facts / 32,768 bytes, with exact stdout comparison.
3. The forty cached-reader timings below, each once, without metadata prequeries.
4. Forty old-reader exact-stdout validations, only after all forty timings.
5. Ten supplemental cached-reader timings at depth 2, each once.
6. Ten old-reader exact-stdout validations, only after all supplemental timings.

Thus the focused source slice was already read before this forty-row series,
and the supplemental series follows the forty oracle validations. This warm
history is part of the protocol. There was no file-cache reset, retry, timing
replacement, or continuation traversal.

The timed harness copies and verifies both executable digests but does not
hash either executable inside a query. It uses a 2 GiB external address-space
ceiling and 30-second watchdog per request, one worker, and the explicit
historical snapshot. The snapshot/private-key fixture remains owned by its
creating worker until the reports are accepted; this harness removes only its
own executable copies. After acceptance, the owner removed the source archive,
store, key, producer and images and verified their absence. The free-space preflight before this campaign reported
115,692,376 KiB available.

## Forty bounded requests

All commands use depth 1, at most 96 facts and at most 32,768 reply bytes.
The wall envelope includes checkpoint writing, GNU time/resource-wrapper launch,
the compiler process and output delivery. The TSV separately retains GNU time's
coarse subprocess elapsed value and the compiler's internal phases. No oracle
latency is included in this table or used to select a timing.

| Identity / question | Question bytes | Slice bytes / facts | Slice continuation | Question wall ms | Slice wall ms | Load before → after |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| `driver::Options` / outline | 4642 | 3705 / 9 | none | 3.351 | 6.729 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `driver::Memo` / outline | 4281 | 4924 / 11 | none | 3.994 | 8.086 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `driver::compile` / item | 1779 | 6864 / 21 | none | 2.856 | 14.441 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `driver::execute_source` / signature | 889 | 31751 / 96 | available | 2.595 | 52.775 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `driver::read_path` / signature | 830 | 4001 / 16 | none | 2.702 | 11.600 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `driver::diagnostic_source_path` / signature | 865 | 4634 / 14 | none | 2.429 | 10.732 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `ir::TypeKind` / outline | 5647 | 4214 / 6 | none | 4.079 | 5.113 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `ir::Op` / outline | 5738 | 6865 / 10 | none | 3.271 | 6.621 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `ir::Function` / outline | 1654 | 3207 / 8 | none | 2.344 | 6.651 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `ir::Program` / outline | 1179 | 2390 / 6 | none | 2.362 | 5.410 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `ir::Local` / outline | 990 | 1606 / 4 | none | 2.361 | 4.206 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `ir::ReturnSource` / outline | 1119 | 2365 / 5 | none | 2.383 | 4.426 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `semantic::Catalog` / outline | 2608 | 3839 / 8 | none | 2.696 | 6.909 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `semantic::Item` / outline | 3128 | 2374 / 5 | none | 3.165 | 4.820 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `semantic::Reference` / outline | 1423 | 1744 / 4 | none | 2.334 | 4.011 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `reuse::AuthenticatedRoot` / outline | 1473 | 1746 / 4 | none | 2.337 | 4.510 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `test_runner::closed_contracts` / signature | 830 | 2911 / 8 | none | 2.693 | 6.619 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `source::load_report` / signature | 950 | 32563 / 95 | available | 2.909 | 53.525 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `native::PreparedImage` / outline | 1093 | 1979 / 4 | none | 2.509 | 4.063 | 4.09/5.03/4.81 → 4.09/5.03/4.81 |
| `authority::analyze` / authority | 767 | 14958 / 61 | none | 2.344 | 35.482 | 4.09/5.03/4.81 → 4.09/5.03/4.81; slice 4.09/5.03/4.81 → 3.93/4.97/4.79 |

| Group | Minimum / median / maximum wall ms |
| --- | ---: |
| Twenty original questions | 2.334 / 2.644 / 4.079 |
| Twenty slices | 4.011 / 6.636 / 53.525 |
| All forty | 2.334 / 4.037 / 53.525 |

Snapshot key/locator opening takes 0.053–0.083 ms (median 0.059 ms).
The query phase takes 0.531–51.340 ms (median 2.3185 ms), including raw-page
authentication and identity checks, decoding, traversal, formatting and stdout.
These data do not isolate storage I/O or authentication CPU. Query RSS is
4,188–7,872 KiB. The wide `execute_source` and `load_report` slices still
return continuations after 273 and 276 raw-page reads respectively.

## Durable evidence and reuse

- [Forty timings and provenance](compiler-query-cache.tsv)
- [Forty subsequent exact-stdout validations](compiler-query-cache-parity.tsv)
- [Shared index and focused-pair timings](compiler-query-cache-witness.tsv)
- [Campaign metadata](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-query-cache.meta) and
  [shared-fixture metadata](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-query-cache-witness.meta)
- [Ten separately labelled query kinds](compiler-query-kinds.md)

`bench/compiler-query-cache.sh` accepts an existing source/library selector,
store, private key, snapshot/source/checked-IR pins, plus separate reader and
producer executables and SHA-256 values. It writes the forty timings first and
then the non-benchmark parity table. Snapshot pins come from the existing
indexed build's output; no discovery query is required.

The original `compiler-query-v2*` artifacts are unchanged. These new data
establish the 100 ms bound for these selected requests under the recorded
conditions. They do not establish a latency guarantee for arbitrary bounds,
host loads, every possible query identity, or the unmeasured larger rungs.
