# Map-consumer cost study — 2026-10-09

## Subject and predeclared comparisons

Selected accepted runtime source: `b6a262f2e1ff95cb6f3889b0d10734a9ea93e078`.
Copied product SHA-256:
`f9d8d5aff6f40c391ce2e46be1b4ce7f804fc30197630acff891c861cb054e43`.
The experiment changes native programs, not this executable. The original source,
in-progress final-byte acceptance work and running applications are not edited.

A literal native workload constructs an immutable Map with keys `0..n-1` and
values `3*key+7`. Length controls return `n*rounds`; aggregating both keys and
values returns `(2*n*n+5*n)*rounds`, independently checked by a host closed form.
Six complete snapshots, from zero to 16,384 entries, separately matched every
key and value against a positional oracle. The dense-key lookup alternative is
specific to this input domain, not a general arbitrary-key iterator.

Three preregistered stages completed 750, 1,200 and 1,500 matrix invocations. Their
pilots completed 20, 32 and 40 additional invocations. Each matrix used sizes
0, 32, 1,024, 4,096 and 16,384; repetition counts 0, 1 and 8; and ten runs per
point. Run zero remains in the evidence as warm-up. Reported medians use runs
1–9; every sample is retained. Mode order rotates and reverses between runs.
Each stage has its own immutable artifact. Comparisons below are within the
third artifact, not across independently authored graph identities.

## Matched results

All values below are measured invocation milliseconds at eight repetitions.
The common Map construction is included. Preparation and result encoding are
recorded separately, not subtracted from process time and called execution.

| Consumer strategy | 32 entries | 1,024 | 4,096 | 16,384 |
| --- | ---: | ---: | ---: | ---: |
| Map length control | 0.196 | 2.819 | 11.448 | 49.045 |
| Fresh entries, then list length | 0.298 | 5.132 | 20.436 | 90.725 |
| Fresh entries and standard fold each time | 1.155 | 30.573 | 127.302 | 543.680 |
| Dense-key direct lookup loop | 0.783 | 20.672 | 84.024 | 351.719 |
| One entry list, standard fold | 1.071 | 29.539 | 123.371 | 522.617 |
| One entry list, static-step indexed loop | 0.915 | 24.566 | 101.613 | 434.957 |
| One entry list, concrete dynamic-step loop | 0.911 | 25.892 | 107.822 | 445.976 |
| One entry list, generic dynamic-step loop | 1.064 | 29.183 | 113.922 | 539.898 |
| One entry list, generic blocked-four fold | 0.849 | 22.231 | 92.684 | 395.712 |
| One entry list, generic blocked-eight fold | 0.799 | 20.934 | 83.151 | 342.668 |

At 16,384 entries the same-list standard samples span 484.820–560.797ms; the
blocked-eight samples span 338.355–360.918ms. The ratio of those group medians is
about 1.53, or about 34.4% less invocation time. This is a shared-host observation,
not a guaranteed speed-up, confidence interval or release benchmark. At zero
entries, blocked-eight is slower: 0.175ms versus 0.133ms for same-list standard.

For the same large comparison, modeled cumulative allocated bytes are 271,894,571
versus 178,193,579; modeled allocation charges are 3,983,114 versus 3,000,162;
and instructions are 3,555,651 versus 2,703,761. These are runtime accounting,
not actual allocator traffic. Whole-process peak RSS medians are 31,460 and
31,512 KiB: the modeled allocation decrease is not evidence of a peak-RSS decrease.

## Measurement and independent re-reading

A small Linux x86-64 Rust process adapter used `Instant`, exact-child `wait4`, a
90-second process bound, 1 MiB output bounds, and owned process-group cleanup.
Product-reported preparation, invocation and result-encoding clocks are the primary
phase measurements. The adapter's 2ms wait polling can add process-wall observation
latency; it does not change the product's internal clock. `wait4` CPU and peak RSS
are whole-process measures. Its six independent tests cover success, failure,
deadline, oversized output, scalar-field admission and the independent result law.

See the primary [Rust Instant documentation](https://doc.rust-lang.org/std/time/struct.Instant.html),
[Linux wait4 contract](https://man7.org/linux/man-pages/man2/wait4.2.html), and
[Linux getrusage field meanings](https://man7.org/linux/man-pages/man2/getrusage.2.html).
The host is an AMD Ryzen 9 9955HX, Linux x86-64, with CPUs 0–31 available. Other
work was not stopped. There was no CPU pinning, fixed frequency, isolated NUMA
placement or controlled cold page cache. Exact inputs, executable, harness and
artifact bytes were checked before and after each stage.

An independent reader decoded every retained execution, observation, cleanup and
executor record. It required the complete Cartesian sample inventory without
missing or duplicate points, unchanged artifact identity, matching phase times,
zero final live state, zero remaining tasks/workers and no cleanup failures.
Every exact input group's modeled work was deterministic. The experiment is not
an adversarial acceptance-certificate system; its complete originals remain the
source for reproducing or extending the analysis.

## Engineering decision

Repeated search was removed by the earlier Map change, but a consumer still pays
for entry construction, indexed traversal, callback dispatch and generic loop
state. Reusing entries alone did not remove the dominant cost in this workload.
Blocked sequential traversal provides a real language-only alternative while
preserving callback order and exact generic contracts. It should not become an
unconditional standard implementation based only on integer aggregation.

Keep the language's semantic contracts independent of this execution mechanism.
Next examine prepared generic-call/loop setup and reusable typed traversal with
real consumer evidence; do not enlarge the trusted kernel merely to expose an
unchecked cursor. This study is not evidence that a JIT, region system, compiler
self-hosting pass or whole-runtime rewrite has been implemented or is required.

The compiler-cache issue discovered during ordinary native composition is tracked
separately as issue #9: a fresh component/port/target can make the accepted edit's
cache update report `compilation_incremental_owner_domain`, after which clean
checking succeeds. The minimal public reproducer retains successful simple and
generic-function controls. It is not attributed to blocked folding, and the cache
owner's guard was not weakened to hide the symptom.

## Evidence locations and identities

Original study root:
`/home/coder/workspace/lkjscript-cost-study-20261009.7QHLUN`.
Predeclarations: `METHOD.md`, `DIAGNOSIS-METHOD.md`, `BLOCKED-METHOD.md`.
Matrices: `evidence/matrix`, `evidence/diagnosis-matrix`, `evidence/blocked-matrix`.
Each contains raw stdout/stderr/results, `samples.tsv`, byte identities, a complete
terminal marker and independently re-read observations. Copies of tabular results
and predeclarations are retained under the contribution checkout's
`.artifacts/native-fold-study/measurements/`; the raw originals are not replaced.

Third artifact SHA-256:
`d5a7073dc6fddf6eb14393f3d0eeef88e104d875746fc134f938d59a05cd5e01`.
Third native workload SHA-256:
`0c52fb14b6e7739e46c631b943928449f989c6e35da2f83c49cb74a0acb5f267`.
Third harness SHA-256:
`71ccea8820c37531ab24f991dc5492eb315b6a3ffddc8a86b97ea5b1fb5f9c16`.
The independently authored two-package maintained example uses fresh graph
identities and is correctness evidence, not the source of these wall-time samples.
