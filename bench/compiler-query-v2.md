# Pinned snapshot v2: forty compiler questions

**Thirty-eight of forty whole commands finish below 100 ms; the universal
100 ms target is still not met.** The observed minimum / median / maximum is
**2.707 / 6.4985 / 207.807 ms**. All twenty original questions are below 100 ms;
two of their twenty bounded slices exceed it. Both exceptions remain in the
data, without faster replacement runs.

The median depth-one slice is **3,761 bytes**, and **2/20** slices require a
continuation. All forty requests pass the recorded producer, source, checked-IR,
snapshot, reply-byte and fact-count checks. Each reports
`producer_bytes_hashed=0` on the explicit snapshot route.

## Contract and comparison scope

This measures an **explicit authenticated historical snapshot**. The v2 locator
and snapshot bind the exact producing compiler digest; authenticated pages
supply its recorded facts. Replies state
`validation=authenticated-producing-compiler current-check=not-performed`
and `freshness=not-asserted`. Reading the snapshot does not assert that the
current tree is unchanged or that the current compiler has checked it.
Ordinary admission, proof reuse and builds retain their producer-specific checks.

The original frozen baseline in [compiler-query-final.md](compiler-query-final.md)
and its [forty rows](compiler-query-final.tsv) remain unchanged. This run uses
new source `20d5f7e`, versus the baseline's `0b30ca89`, and a different producer.
It also removes the baseline harness's unmeasured per-subject metadata query.
The bounds and twenty subjects remain the same. These are direct new command
measurements, **not a speedup calculated by subtracting the old producer phase**,
and not a controlled same-source ratio.

The old `producer_hash_ms` instrumentation brackets all of `Memo::new`,
including private-key/store setup as well as executable hashing. It is a
producer-setup phase, not an isolated hash measurement. The new explicit route
records `snapshot_auth_ms` and `semantic_query_ms` instead; its absent
producer-setup field is **NA**, not a measured zero.

## Frozen inputs and artifacts

| Identity | Value |
| --- | --- |
| Source revision | `20d5f7e98fe4501d441b3ac357733a8a7ba942e7` |
| Source tree | `5daafd1a48ef844ec00c4edc76c3e3d87bf41445` |
| Library tree | `00d0571a0f4f36e2ea4a6d57a23d54afe360d763` |
| Producing compiler and query reader SHA-256 | `51708066a1c150ba51800697751b887ad279792f8b4e4d4a57d5df3e291d934c` |
| Producing compiler / reader bytes | 20,606,806 |
| Source snapshot digest | `96b34c0949e2079172ab900cad016141e31bc863cdb035f56ee669f22b7a8548` |
| Canonical checked IR | `e6d5997570b086d279f884426c2014c73e93b1ac2d6da48028f620eb5b9a0c19` |
| Authenticated snapshot | `fd0c32f70312d53943f098f3e01d05d3befc9f2c486aedaf964bdaa94de83c74` |
| Indexed output image SHA-256 | `8e10a841bba80107d589094b0af4a32852c05e99966d1d3a18e3e79c32ef1fc5` |
| Indexed output image bytes | 20,625,214 |

The indexed image and executing producer are separate artifacts. Cold and
unchanged indexed outputs are byte-identical to each other; the image is not
byte-identical to the supplied reader. This benchmark does not claim a compiler
fixed point. It explicitly selects the archived physical source library and
runtime, with native IR inlining disabled by `--inline-limit 0`. [The metadata](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-query-v2.meta) preserves the actual selector
path and every pin; the temporary archive itself has been removed.

## Protocol and reproduction

```sh
bash bench/compiler-query-v2.sh \
  --compiler /path/to/compiler-query-v2 \
  --revision 20d5f7e98fe4501d441b3ac357733a8a7ba942e7 \
  --producer-sha256 51708066a1c150ba51800697751b887ad279792f8b4e4d4a57d5df3e291d934c \
  --results target/compiler-query-v2.tsv
```

The harness copies and verifies the producer, archives only `src/`, `library/`,
`tools/` and `tests/`, and creates a private key outside all source roots.
It performs one cold indexed build, one unchanged indexed build, then the
question and slice for each subject in the table below, in that order. Each
timed command runs **exactly once**. No metadata query precedes a timed request;
no continuation is followed. Provenance and bounds are validated afterward
from the captured output, without reading snapshot pages for validation.

Every request uses depth **1**, at most **96 facts**, and at most **32,768 output
bytes**. External wall time is the launch-inclusive envelope: it includes the
checkpoint write after the start timestamp, GNU time and resource-wrapper launch,
the compiler process and output delivery. The separate `subprocess_wall_s`
column is GNU time's elapsed subprocess time, including prlimit/timeout, at
0.01-second resolution; it was copied from the original logs without rerunning
requests. Internal timings come from the compiler's stats. GNU time's
user/kernel CPU values also have 0.01-second resolution.
Each timing has the 1/5/15-minute load immediately before and after it. No
file-cache reset is claimed. This agent used one worker; the parent's optional
Rust witness could run concurrently on the shared host.

The harness requires 4 GiB free space; this run began with 115,153,532 KiB free.
Indexed builds have a **6 GiB** external address-space ceiling, fitted to the
previous measured 2,435,680 KiB resident peak, and a 900-second watchdog.
Queries have a **2 GiB** ceiling and 30-second watchdog. These are experiment
limits, not language allocation policy. Normal exit removes the source tree,
store, private key, copied producer and generated image. An interrupted
supervisor can be cleaned with `--cleanup OWNED_RUN_PATH`, which checks ownership
and refuses a live or referenced run before removing it. Raw timed outputs and
the checkpoint are retained for a failed attempt.

[The query TSV](compiler-query-v2.tsv) preserves all forty rows, including
the internal phases, page reads, traversal steps, byte counts, continuations,
four provenance digests, answer digests and paired loads.
[The build TSV](compiler-query-v2-build.tsv) preserves both construction rows.

## Index construction and unchanged build

| Operation | Wall / user / kernel s | Peak RSS KiB | Load before → after, 1/5/15 min |
| --- | ---: | ---: | --- |
| Cold indexed build | 125.211713 / 114.58 / 4.39 | 2,647,832 | 10.60/6.48/5.23 → 5.95/5.91/5.17 |
| Unchanged indexed build | 0.479580 / 0.45 / 0.01 | 32,740 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |

The cold build contains **12,438 IR functions** and **1,197,212 instructions**.
It retains **761,742,379 store bytes**, unchanged after all forty queries.
The unchanged build returns the same snapshot, source digest, checked IR and
image, with **one reused build, five store hits and zero writes**.

Cold internal phases include 169.702 ms source read, 1,068.645 ms parse,
232.774 ms resolve, 6,711.194 ms type work, 6,803.632 ms IR construction,
6,691.526 ms body checking, **18,490.302 ms catalog construction** and
**81,463.808 ms index publication**. Producer setup takes 183.580 ms cold and
178.767 ms unchanged. Those phase values share their enclosing row's load
window; they are not additional timed runs. Index construction remains a
substantial whole-program cost.

## Twenty bounded pairs

All times below are external whole-command milliseconds. A shared load cell
applies to both commands unless the slice window is stated separately.

| Identity / question | Question bytes | Slice bytes / facts | Slice continuation | Question wall ms | Slice wall ms | Load before → after |
| --- | ---: | ---: | --- | ---: | ---: | --- |
| `driver::Options` / outline | 4642 | 3694 / 9 | none | 13.328 | 10.612 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |
| `driver::Memo` / outline | 4281 | 4913 / 11 | none | 10.517 | 207.807 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |
| `driver::compile` / item | 1768 | 6853 / 21 | none | 4.332 | 25.882 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |
| `driver::execute_source` / signature | 889 | 31740 / 96 | available | 3.029 | 96.742 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |
| `driver::read_path` / signature | 830 | 3990 / 16 | none | 2.916 | 20.477 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |
| `driver::diagnostic_source_path` / signature | 865 | 4623 / 14 | none | 2.969 | 16.718 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |
| `ir::TypeKind` / outline | 5647 | 4203 / 6 | none | 14.161 | 6.794 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |
| `ir::Op` / outline | 5738 | 6854 / 10 | none | 10.931 | 10.747 | 5.95/5.91/5.17 → 5.95/5.91/5.17 |
| `ir::Function` / outline | 1654 | 3196 / 8 | none | 3.964 | 10.522 | 5.95/5.91/5.17 → 5.95/5.91/5.17; slice 5.95/5.91/5.17 → 6.03/5.93/5.18 |
| `ir::Program` / outline | 1179 | 2379 / 6 | none | 3.632 | 8.197 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `ir::Local` / outline | 990 | 1595 / 4 | none | 3.383 | 5.550 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `ir::ReturnSource` / outline | 1119 | 2354 / 5 | none | 2.886 | 6.695 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `semantic::Catalog` / outline | 2608 | 3828 / 8 | none | 5.960 | 12.351 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `semantic::Item` / outline | 3128 | 2363 / 5 | none | 7.919 | 6.302 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `semantic::Reference` / outline | 1423 | 1733 / 4 | none | 4.069 | 6.076 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `reuse::AuthenticatedRoot` / outline | 1473 | 1735 / 4 | none | 4.059 | 5.685 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `test_runner::closed_contracts` / signature | 830 | 2900 / 8 | none | 2.990 | 10.426 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `source::load_report` / signature | 950 | 32552 / 95 | available | 2.716 | 102.204 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `native::PreparedImage` / outline | 1093 | 1968 / 4 | none | 3.375 | 5.496 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |
| `authority::analyze` / authority | 767 | 14947 / 61 | none | 2.707 | 63.645 | 6.03/5.93/5.18 → 6.03/5.93/5.18 |

## Remaining latency and context limits

| Group | Minimum / median / maximum wall ms |
| --- | ---: |
| Twenty original questions | 2.707 / 3.798 / 14.161 |
| Twenty slices | 5.496 / 10.567 / 207.807 |
| All forty requests | 2.707 / 6.4985 / 207.807 |

The initial private-key and authenticated-locator opening phase is **0.060–0.222 ms**
(median 0.0785 ms). The following query phase is **0.923–204.364 ms**
(median 4.4635 ms). That second phase includes authenticated page reads,
payload/identity verification, decoding, traversal, formatting and stdout;
the samples do not separately attribute disk I/O, authentication CPU or
scheduling delay.

The `driver::Memo` slice is the retained **207.807 ms** outlier:
204.364 ms in the query phase, 43 pages, 11 facts and 4,913 output bytes.
GNU time reports 0.01 s user and 0.00 s kernel, at load
5.95/5.91/5.17 → 5.95/5.91/5.17. Its cause is **unknown**; this evidence does
not identify disk wait or scheduler contention, and the row was not rerun.

The `source::load_report` slice takes **102.204 ms**, including 99.806 ms
in the query phase. It reads 276 pages in 266 traversal steps and returns
95 facts / 32,552 bytes with a continuation, at load
6.03/5.93/5.18 → 6.03/5.93/5.18. Its internal total is 99.883 ms, but the
requirement concerns the whole command. The other wide slice,
`driver::execute_source`, is close at **96.742 ms**, with 273 pages,
96 facts and 31,740 bytes; that observation also remains unchanged.

Slice output ranges from **1,595 to 32,552 bytes**, with the 3,761-byte median
reported above. The two continuations explicitly expose the contexts that do
not fit this bound. Query RSS ranges from 2,132 to 5,844 KiB. These measurements
establish bounded historical-snapshot retrieval on these forty requests;
they do not establish the universal 100 ms target, a latency guarantee under
other loads, or independence across unmeasured larger program rungs.
