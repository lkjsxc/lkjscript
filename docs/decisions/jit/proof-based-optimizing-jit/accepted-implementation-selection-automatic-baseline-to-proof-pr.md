# Proof-Based Optimizing JIT: Accepted Implementation Selection: Automatic Baseline-To-Proof Promotion

[Authority](../proof-based-optimizing-jit.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Accepted Implementation Selection: Automatic Baseline-To-Proof Promotion

This section retains an **Accepted Implementation Selection** and is **not yet
Current**. The AI-native platform decision supersedes its former status as the
immediate repository-wide priority; bounded topology, repository graph/context,
agent work state, and first Semantic Source/profile slices land first. When this
promotion slice resumes, this exact process-local synchronous contract and its
predeclared gate remain authoritative. Existing
automatic baseline behavior is unchanged: ordinary `auto` still uses the
64-VM-entry baseline threshold, the threshold call compiles synchronously but
runs in the VM, and only a later call may select baseline native code.
Automatic optimizing promotion will initially be disabled by default and
available only through explicit CLI opt-in, with a separate deterministic
optimizing-threshold control. The retained gate will test exact optimizing thresholds 64,
256, 1,024, and 4,096 baseline entries; no default optimizing threshold is
selected by this record.

Optimizing hotness counts exact baseline-native entries of the promotion root.
VM entries, generated helper entries, direct native callees, compile attempts,
and install events do not increment it. An exact opaque entry token identifies
the source function, owning code object, and tier; counters, current selection,
and invocation all validate that token rather than matching a source name or
raw address.

On the Nth baseline entry, the session captures the exact current baseline
object and entry token, transitions synchronously through proof optimization,
lowering, and bounded W^X installation, and retains the resulting optimizing
object as pending. That triggering entry must still invoke the captured
baseline object. Only a later root entry may publish and select the pending
optimizing object, so the earliest optimizing entry is N+1. Publication checks
that the captured baseline is still current and both tokens remain valid.
There is no on-stack replacement, background compilation, deoptimization,
guard, or speculative assumption.

The selected automatic states are:

```text
BaselineCandidate
BaselineCompiling
BaselineNative
OptimizingCandidate
OptimizingCompiling
OptimizingPending
OptimizingNative
Disabled
```

The state and exact object ownership are explicit: once baseline is installed,
an auto-native entry has one selectable current object and at most one non-
selectable pending object. Baseline and optimizing objects coexist under exact
session ownership after
promotion; selecting optimizing code does not destroy its baseline recovery
object. Invalidated or superseded objects are marked stale, unlinked from all
selectable entry tokens, and retained under a bounded stale-object/mapping
budget until synchronous session drop. A stale object is never selectable or
invocable.

Each tier records saturating exact-entry counters, the explicit configuration/
resource epoch, attempts, structured failures, and same-epoch suppressions.
There is at most one optimizing attempt per epoch and a bounded total attempt
count. Failure keeps the baseline object current and records a structured tier
reason; another entry in the same epoch is suppressed rather than retried. A
newer explicit epoch permits at most one further attempt within the total
bound. It also invalidates any pending or current optimizing object and selects
the still-valid baseline object before counting new promotion eligibility.
Promotion transitions to `Disabled` when the total attempt bound is exhausted
or the root is permanently unsupported; baseline remains current. Reentrancy cannot observe or publish
an object in `OptimizingCompiling` or `OptimizingPending`.

The auto VM adapter remains scalar-only at function entry. A generated group
may contain reference-signature helpers that call, allocate, collect, and use
exact roots internally, but such helpers cannot transfer references between VM
and native code, cannot become auto entry roots, and cannot increment a root's
promotion count. Source main remains in the VM. Forced `baseline-jit` and
`optimizing-jit` semantics are unchanged: each requires its requested tier and
reports an engine error rather than falling back.
## Selected Automatic Metrics And Limits

The implementation must expose the configured baseline/optimizing enables and
thresholds; exact epoch; per-tier attempts, failures, and same-epoch suppressed
attempts; every state transition; exact baseline-root entries before the first
optimized entry; time from session start and threshold trigger to first
optimized entry; and exact entries, opaque object IDs, entry tokens, and code
bytes by tier. It also retains proof/certificate/checker facts, W^X transition
and mapping accounting, stale-object invalidations, current/pending selection,
and zero-fallback evidence. Normal stdout remains untouched.

Limits separately bound current plus stale retained mappings and bytes, object
count, per-epoch and total attempts, optimizer discovery/check/reconstruction/
cleanup work, certificate records/bytes, code and retained metadata. Hitting a
limit is a structured tier failure; it cannot make an object selectable, weaken
proof checking, or evict a still-callable object.
## Predeclared Automatic Promotion Benchmark

The implementation does not become enabled by default without a retained clean,
locked release result. The standard-library harness must use deterministic
randomized ordering with at least four warmups and 31 unremoved measured samples
per case. It compares auto baseline-only with auto optimizing opt-in at exact
thresholds 64, 256, 1,024, and 4,096, plus unchanged forced-baseline and forced-
optimizing sentinels and allocation/reference correctness runs. Every sample
checks the exact independent oracle, stdout/stderr expectations, state and token
transitions, tier/object entries, proof/certificate acceptance, W^X, bounded
attempts, and zero forced or auto fallback.

A candidate is adoptable only if all of these predeclared criteria pass:

1. median end-to-end process time is at least 1.10x faster than auto
   baseline-only;
2. its baseline-only median improvement is greater than twice the sum of the
   two process-time MADs;
3. its nearest-rank p95 process time is no more than 5% worse than baseline-only;
4. measured optimization, lowering, and install cost is repaid before workload
   completion, both in the recorded break-even entry and cumulative saved time;
5. exact results, streams, state/token/object transitions, proof, W^X,
   allocation/reference behavior, and limits all pass;
6. each epoch has at most one attempt, there is no repeated attempt or fallback,
   and the first optimizing entry occurs only after exactly N baseline entries;
7. the unchanged forced scalar sentinel's native and process medians are each
   no more than 5% above the retained 7,647,935 ns and 9,372,036 ns historical
   medians.

Among candidates passing every criterion, the selected threshold is the
largest whose process median is statistically indistinguishable from the
fastest passing candidate: the absolute median difference must be no greater
than twice the sum of their process MADs. If no candidate passes, automatic
optimizing remains disabled by default and the complete rejection, including
all samples and attempted thresholds, is retained. A passing benchmark still
requires a later documentation/implementation change to make its selected
default Current.
## Metrics And Acceptance

Metrics distinguish baseline and optimizing compile/pass/install/entry times,
object bytes, metadata, cache peaks, allocation/collection facts, failures,
fallbacks, and exact tier transitions. Optimization statistics retain separate
executed discovery, checker, reconstruction, cleanup, and ordinary-validation
pass counters; `optimizing_passes` is their actual optimizing-pass total rather
than a value inferred from object and iteration counts. Certificate and retained
optimization metadata byte fields are explicitly named `_estimate`: the
certificate estimate is 8 bytes of canonical header plus 31 fixed bytes and 4
bytes per operand for each record, and retained metadata adds eight bytes per
reported scalar statistic. These are deterministic accounting formulas, not
Rust allocator-size claims. Forced tests require nonzero optimizing entries and
zero baseline/VM downgrade.

The adopted forced workload demonstrates 2,424 optimizing versus 13,656
baseline generated code bytes, 72 checked-I64 GVN records, and 10,001
optimizing entries with zero baseline entry or fallback. Current-thread stack
bounds are queried once per invocation rather than once per generated frame
reservation; every reservation still uses the cached guarded bounds.

At `cc967ff`, four warmups and 31 measured samples per forced case in one
deterministically randomized interleaving produced exact native medians of
1,999,889 ns baseline (MAD 10,469 ns) and 670,029 ns optimizing (MAD 2,174 ns):
2.984780x. The 1,329,860 ns improvement exceeded twice the combined MAD,
25,286 ns. Process-wall medians were 3,565,363 and 2,411,023 ns, a 1.478776x
speedup. Exact I64 `3333`, optimizing entries, zero downgrade, W^X, proof, and
code-byte gates all passed. Same-commit scalar forced baseline measured
7,982,586 ns native and 9,207,038 ns process wall. Against the retained callable
baseline's 7,647,935 and 9,372,036 ns, the ratios were 1.043757 and 0.982395;
both passed the 1.05 ceiling. Every predeclared criterion passed, so the forced
first-tier performance verdict is **Adopted**.

The earlier `063668e` run remains retained and **Rejected**. Its local optimizer
result was 2.930761x, but scalar native measured 8,182,742 versus 7,647,935 ns,
a failing 1.069928 ratio; scalar process wall passed at 9,340,049 versus
9,372,036 ns, ratio 0.996587. The later performance recovery followed folding
the mandatory generated-function entry poll into canonical native contract frame registration,
removing a separate runtime transition without weakening polling or optimizer
proofs. Neither cross-commit scalar comparison attributes performance to the
optimizer, and adoption does not erase the first run's negative evidence.
Automatic promotion remains disabled and unmeasured. Its implementation and
benchmark remain selected above but are not Current and no longer have immediate
priority; OSR, deoptimization, and speculation were not measured or added.
