# Cache I/O attribution protocol

Frozen before the ordinary-build observations. This is attribution, not a page
migration or a compiler speed milestone. No historical replay or million-line
rung is included.

## Producers and source

The common immutable emitter is the pushed producer with SHA-256
`71b2fc066cd74e9e37cb3134173903c9b322b83472b9293f1a49df7c1d18f3b7`.
It constructs both producers with explicit inline limit 64, physical library and
runtime selectors, no store, and the same isolated compiler source path. Baseline
source is `b7e8ed035d155e75438406c7474a747220ff9537`;
the instrumented source changes only ordinary store attribution and driver stats.
Construction time, CPU, peak RSS, loads, image sizes and digests are separate from
the ordinary-program observations.

`bench/scale.lkj` takes its census from archived baseline src/library. It generates
exactly 10,000 then 100,000 lines with seed 20261116. The census and each generated
workload report are retained, with relative source-file digest manifests. Both
producers use the same generated source path and physical frozen library/runtime.
Ordinary emitted outputs explicitly use inline limit zero.

## Preselected sequence

For 10k, then 100k, run baseline before instrumented, once each:

1. Empty-store build.
2. Unchanged stored build.
3. One checked `replace-body edit::target` transaction changing xor 1 to xor 2.
4. Build the changed body using the original producer's prior store.
5. No-store build of that same changed source.

The body transaction uses the common immutable emitter and a separate fresh
operation store. It therefore cannot prewarm the ordinary build's frontend,
checking or native records. Its costs are retained as preparation observations
and do not enter the paired producer-overhead comparison. The instrumented
producer starts from a restored exact pristine projection and an independent
empty store. Store capacity is fixed at 1 GiB.

Cold/changed/unchanged/no-store output bytes must agree between producers and with
the relevant no-store control. The body output must differ from the cold output.
Every attempted command and refusal is retained; a failed phase stops later
observations. There are no timing retries or data-dependent order changes.

Every compiler/generator command runs sequentially under an external 4 GiB
address-space limit and 900 s watchdog. These are experimental host guards, not
language policy. Free space is checked before setup and each rung. Each measured
command records paired three-component host loads, launch-inclusive wall time,
GNU time wall/user/kernel/RSS and exact producer/source/output identities. No file
cache reset is claimed. Source manifests and output comparisons are outside
timed intervals; source is hashed after generation or publication, not before
each individual stored build.

## Counter meanings

Stats are opt-in. Existing LKJSCRIPT_STATS fields remain; LKJSCRIPT_STORE_IO adds
one summary and at most eleven fixed namespace-family rows. It never prints a
secret, raw payload, source text, physical path or namespace identity digest.

- Read bytes count accepted authenticated payload snapshots and their complete
  envelopes. Failed or partial reads contribute attempts/time, not accepted bytes.
- Pack decode bytes are authenticated bytes offered to pack decoding, including a
  subsequent decoder refusal. Pack-load count requires successful decoding.
- Retain time covers the existing per-key retention pass after pack decoding.
- Seal bytes count standalone payloads; pack reseal bytes separately count the
  full encoded pack payload. Reseal time includes pack serialization and HMAC.
- Published bytes count envelopes whose atomic publication succeeded. Publication
  time includes temporary-file creation, write, rename and cleanup; it is not
  merely a write syscall duration.
- Budget-header bytes count complete headers scanned before the budget validity
  check. They are neither authenticated payload bytes nor all bytes read.
- Metadata time covers remaining budget/index/eviction bookkeeping. Prepare time
  covers remaining named-publication bookkeeping. Nested completed intervals are
  subtracted, so all category phase durations are nonoverlapping.
- Summary timing-failure and overflow counts are global and emitted once.
  Pending checkpoint marks a report that can omit a later Drop retry. The report
  covers work through the driver's explicit finish/report point, not every
  possible failed-exit retry.
- Producer/key setup, source reads, query cursor HMACs and the separate historical
  QueryReader path are outside these ordinary record-I/O counters.

Successful observation rows require zero global timing/overflow flags and no
pending checkpoint. Missing, backward or inconsistent timestamps flag an
unmeasured interval; optional counter arithmetic cannot introduce an overflow
trap. A deterministic nested-arithmetic control is checked with native execution
and the original interpreter. Owned-envelope tests compare counters with actual
file lengths and verify unchanged default output.

The counters do not report physical disk bytes, syscall traffic, bandwidth or
time saved by paging. Paired wall differences include host/order variation,
producer size/authentication differences and the method itself; one observation
per phase cannot isolate a causal instrumentation overhead.
