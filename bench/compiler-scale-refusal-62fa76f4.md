# Current-producer 1M ordinary-build refusal

The prospectively frozen cohort completed once. Its fresh ordinary **100k build
succeeded**. The **1M build and its separately permitted cold check both refused**
under the selected 8 GiB address-space ceiling, each reporting:

```text
src/elaborate.lkj:2618: R0008 stack exhausted
```

Both refusals have complete wall/CPU/RSS/load and matching wrapper/GNU exit
records. The independent check shows that the shared compile route also fails
in this envelope. The repeated diagnostic does not establish an identical
runtime call chain, allocation, exact compiler phase or root cause.
There was no retry, larger limit, reduced specimen, alternative source or
additional workload. The original incomplete 1M cohort remains unchanged.

## Protocol, producer and execution

The [protocol](compiler-scale-refusal-plan.md) and corrected harness/helper were
committed before execution. The [actual release](compiler-scale-refusal-release-62fa76f4.json)
was committed at `15171eeaddd0029b23cbae17d4657b4252896b3e`, JSON SHA-256
`790022df0306aa06f6543caaa9c51a8dbf24bea2b0626fe60ffe45f9ccebb494`.
Root independently inspected that input and ran its release validator, then
explicitly authorized its exact invocation once.

| Identity | Value |
| --- | --- |
| Gated producer source | `62fa76f4f863a169b906094f0dd9998c8421e77a` |
| Source / library trees | `00d1109fcdc49041d13d7406ca74c8f5b4394e8d` / `25bdb8ff8567b941790d3951c78e896b23e26e8e` |
| Producer SHA-256 | `34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377` |
| Producer bytes / construction inline limit | 32,196,802 / 64 |
| Seed-only commit | `72a259e8889cff6e7c4ac712d7940b77343a9f5b` |
| Harness / helper SHA-256 | `3953f766928e37ceb6321d90210e63c827c31821357c14fd73c033efa62f1a70` / `33173a81506676580de71da43725db2e814931866da3339d770d979982fbdcf7` |
| Launch request / session | `compiler-scale-refusal-launch-015` / `pipe-njokambnlndiehlaijfjhcdclegmkppi` |
| Final gateway observation | `compiler-scale-refusal-poll-019`; exit 1, complete capture, no watchdog/signal |
| Workers / retries | One sequential worker / zero |
| Compiler commands | One generator-factory build, three current-producer commands |
| Generator commands | Two, one per declared rung |

The native compiler was copied into the owned fixture as a mode-500 executable;
its digest still matched after the cohort. Specimen output used inline limit
zero, statistics and Store I/O counters. Each measured build/check had a distinct
initially empty 2 GiB store and the same external private key outside all source
and library grants. No index, semantic question, edit, rename, profile, pass or
10M generation ran. File caches were not reset.

## All six timed phases

Wall is GNU time around the declared resource launcher. RSS is its observed
peak for that command; RLIMIT_AS is a separate address-space limit.
Load columns retain the 1/5/15-minute readings immediately before and after
each command. The two refused-command durations are **elapsed to refusal**,
not completed-build observations.

| Phase | Outcome / exit | Wall s | User s | Kernel s | Peak RSS KiB | Load before -> after |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Generator factory build | Success / 0 | 2.14 | 2.07 | 0.01 | 115,292 | 0.85/1.35/1.63 -> 0.85/1.35/1.63 |
| Generate 100k | Success / 0 | 1.11 | 1.07 | 0.00 | 2,112 | 0.85/1.35/1.63 -> 0.85/1.35/1.63 |
| Ordinary 100k cold build | Success / 0 | 11.20 | 10.74 | 0.18 | 713,512 | 0.85/1.35/1.63 -> 0.95/1.35/1.63 |
| Generate 1M | Success / 0 | 10.93 | 10.69 | 0.05 | 8,704 | 0.95/1.35/1.63 -> 1.17/1.39/1.63 |
| Ordinary 1M cold build | Located R0008 / 1 | 77.04 | 74.07 | 1.43 | 5,224,960 | 1.17/1.39/1.63 -> 1.94/1.56/1.67 |
| Separate 1M cold check | Located R0008 / 1 | 76.24 | 73.49 | 1.42 | 5,220,184 | 1.94/1.56/1.67 -> 2.34/1.77/1.74 |

The setup and 100k commands each had 4 GiB address space, 900 seconds and a
ten-second termination grace. Every wrapper exit equals its GNU-time exit;
all six derived captures are complete. The supervisor ended at
`2026-11-16T11:55:57Z` with exit 1, preserving the failed primary outcome.
This was not a missing-supervisor or timeout result.

The 100k control passed the predeclared no-reuse, no-inlining, positive instance/
image and error-free Store-counter checks. Its current-producer statistics
report 4,209 IR functions, 896,361 instructions and 11,194.718 ms total.
Neither 1M command emitted `LKJSCRIPT_STATS` or a Store I/O summary.
Their phase totals, checked instance counts and compiler-reported retained
bytes therefore remain unavailable.

## Bound derived from the fresh control

The new 100k command supplied R = 713,512 KiB and T = 11.20 seconds.
The frozen formulas give:

- Linear planning RSS at 1M: 7,135,120 KiB.
- Desired allowance after the declared headroom: 14,192 MiB.
- Applied ceiling: **8,192 MiB = 8,589,934,592 bytes**.
- Applied deadline: **456 seconds**, with ten-second termination grace.

These are planning values and external limits. They are not measured 1M
resident memory. Both commands stopped normally before the watchdog limit.

All three fixed headroom preflights passed. The initial reading was
388,072,198,144 free disk bytes and 59,123,710 KiB MemAvailable
(load 0.84/1.36/1.63). Before 1M generation it was
388,075,933,696 bytes / 58,821,765 KiB (0.95/1.35/1.63).
Before the conditional check it was
388,044,357,632 bytes / 59,104,771 KiB (1.94/1.56/1.67).
No limit changed. The values are launch observations, not reservations or an
assertion about other shared workloads.

The primary exit was ordinary status 1 with a located R0008, complete metrics
and equal wrapper/GNU exit values. This satisfied the existing conditional rule.
Before the separate check, all 1,881 input-manifest records verified again and
the fixed headroom preflight passed. The conditional check used another empty
store, the same source/library/producer/key, and the same 8 GiB/456-second bound.

## Frozen specimen and observed artifacts

The generator remained SHA-256
`557d6b124fe739f17c678c77b92e5f6259d87ebca69c2a56aca411071c5ff1e9`;
the full compiler-derived shape remained
`0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c`.
Both rungs used seed 20261116. The factory src/library and its old compiler
closure remained exactly those declared in the protocol; its newly constructed
generator was `8a7a1383217079991eaded758a9dc0ccd6b2c32688415f07d1e46e5c9bb72243`.
No evolving compiler source was substituted for the shape or specimen library.

| Generator-reported shape | 100k | 1M |
| --- | ---: | ---: |
| Lines | 100,000 | 1,000,000 |
| Source bytes | 3,923,070 | 40,327,556 |
| Source files | 169 | 1,880 |
| Functions | 3,764 | 38,005 |
| Generic functions | 155 | 1,661 |
| Call sites | 47,607 | 487,567 |
| Nominal types | 196 | 2,249 |
| Fields / variants / nominal edges | 637 / 204 / 375 | 7,583 / 3,813 / 4,462 |
| Tail constants | 6 | 0 |

These are generated-source counts, not admitted instance counts. Each complete
source inventory also includes its workload report, giving 170 and 1,881
manifest records respectively. The 100k manifest is byte-identical to the
accepted container specimen, SHA-256
`315f2c153190f8f1f0b12fa0bd8d4f0200f0b17c407f2e018617f278f820794e`.
The new 1M manifest is
`5b98ab02c36013ee49fc45063945b5e7bf3601ed0345def83ef0ab916a8a9b06`.
The 215-record factory and 28-record specimen-library manifests are retained.

| Artifact observation | 100k build | 1M build | 1M check |
| --- | ---: | --- | --- |
| Image bytes | 4,472,259 | Absent | Not produced by check |
| Observed store apparent bytes | 26,716,768 | 0 | 0 |
| Compiler-reported retained bytes | 26,716,768 | Unavailable | Unavailable |
| Admitted IR instances | 4,209 | Unavailable | Unavailable |

The 100k image SHA-256 is
`cf6568cea58777168705a5d6ed11aa0eb57c0ca0f811eebd55f5b3c3e9370bea`.
Observed empty store sizes after the refusals do not supply missing compiler
statistics or prove an internal phase boundary.

## Interpretation and remaining ladder cells

This cohort establishes a successful 100k control and complete, located
1M stack-refusal observations for both the ordinary build and shared check
route under the declared envelope. The matching diagnostic is a source-level
trap location in this producer. No stack trace, dynamic recursion depth,
allocation-site proof or OS observer was added.

The old 1M observation used another producer and source population and reported
a different trap with missing completion metrics. Neither its missing values
nor its cause are filled by this new result. Historical container timings
continue to serve only their explicitly labelled planning role here.

There is only one successful completed build in this cohort. **No new
100k-to-1M wall, CPU or RSS exponent is defined.** Refusal times and peaks are
not substituted as completed-build points. No historical/current producer
measurements are mixed into an exponent.

10M was not generated or executed, with reason
`after_one_million_resource_refusal`. Its measured wall, CPU, RSS, store,
image, instances, query and rename cells remain not run with no load reading.
The protocol's historical extrapolations remain labelled illustrations and
are not new observations. The owner's complete structural ladder, including
questions and edit costs, remains pending.

## Retention and review

All original target evidence remains unchanged. Its regular files are copied
byte-for-byte under
[compiler-scale-refusal-data/measurement-1](compiler-scale-refusal-data/measurement-1/).
That includes full raw stdout/stderr, argv/cwd/stdin metadata, statuses,
GNU-time records, load readings, captured failure diagnostics, derived rows,
frozen release, source/library/factory inventories and the conditional decision.
[Launch records](compiler-scale-refusal-data/launch-1/) retain the exact gateway
invocation, complete supervisor output and fixture ownership metadata.
Private key bytes are not copied.

The live fixture remains:
`/home/coder/workspace/lkjscript-compiler-scale-refusal-20261116/target/compiler-scale-refusal-run.tCSZMe`.
Its external key directory is `/tmp/compiler-scale-refusal-key.aD8jW6`.
The supervisor has ended. No cleanup has run; source trees, stores, images,
generator and copied producer remain available for independent root acceptance.
This report starts no follow-up experiment and changes no compiler or seed.
