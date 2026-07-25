# Experiment Registry: C7 Automatic Baseline-To-Proof Promotion: Predeclared, Not Run

[Authority](../experiments.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## C7 Automatic Baseline-To-Proof Promotion: Predeclared, Not Run

- Status: **Accepted Implementation Selection**, not Current and not measured.
  The selected automatic-optimizing policy is CLI-opt-in and disabled by
  default. This predeclaration does not alter the adopted forced `cc967ff` result or the
  retained rejected `063668e` result.
- Question: can synchronous proof promotion repay its compilation cost and
  improve end-to-end auto process time without tail, scalar, correctness,
  ownership, retry, or fallback regressions?
- Baseline/candidates: one clean locked release binary compares auto baseline-
  only (existing VM-entry threshold 64) against optimizing opt-in at exact
  thresholds 64, 256, 1,024, and 4,096 baseline-native entries of the promotion
  root. Unchanged forced baseline/optimizing cases are tier sentinels; a scalar
  forced-baseline case is the historical performance sentinel; allocation and
  reference-group cases are untimed correctness sentinels.
- Protocol: fixed recorded seed, deterministic randomized interleaving, at
  least four warmups and 31 measured samples for every timed case, monotonic
  process-wall measurement, nearest-rank p95, median absolute deviation, no
  removed samples, exact repository cleanliness/commit/environment/tool/source/
  binary identities, one shared artifact tree, and compact retained output.
- Oracle and streams: a separate reference VM supplies the exact result;
  expected stdout/stderr are checked byte-for-byte before a sample is accepted.
  Metrics remain on their opt-in file/stderr channel and cannot contaminate
  program streams.
- Transition oracle: the Nth exact baseline entry of the root must synchronously
  prove, lower, and W^X-install while invoking the captured baseline object.
  The optimizing object is pending until a later root entry. Exact opaque tokens
  identify function/object/tier; helper/direct-callee/VM/install events do not
  count. Main remains VM and reference-signature helpers may call/allocate only
  inside the native group, never through auto VM/native entry.
- Ownership/failure oracle: one process-local session owns one current and at
  most one pending selection plus bounded stale mappings until drop. Stale code
  is never selectable. Each epoch permits one attempt under a bounded total;
  same-epoch attempts are suppressed. Structured failure leaves baseline
  current. A newer explicit epoch invalidates pending/current optimizing code
  to baseline before permitting one bounded retry.
- Required metrics: exact enables/thresholds, epoch, attempts/failures/
  suppressions, all state transitions, baseline entries before first optimized,
  trigger-to-first-optimized and session-to-first-optimized time, exact tier
  entries/object IDs/tokens/code bytes, proof/checker/certificate and W^X facts,
  stale invalidations, current/pending facts, fallback, and retained mapping/
  attempt/optimizer-work/certificate/metadata bounds.

Predeclared adoption is mechanical. Every candidate must have exact correctness,
streams, state, proof, W^X, allocation/reference, and limit results; no repeated
attempt or fallback; at least 1.10x baseline-only median process speedup; a
median improvement greater than twice the sum of candidate and baseline MAD;
nearest-rank p95 no more than 5% worse than baseline-only; and a recorded
break-even entry plus cumulative saving showing optimization/lowering/install
cost repaid before workload completion. The forced scalar sentinel's native and
process medians must each be no more than 5% above retained 7,647,935 ns and
9,372,036 ns.

If candidates pass, select the largest threshold whose process median is
statistically indistinguishable from the fastest passing candidate, defined as
an absolute median difference no greater than twice the sum of those candidates'
MADs. If no threshold passes, automatic optimizing remains disabled and the
complete clean rejection is retained. No C7 command has run and no outcome is
claimed in this documentation-only selection.
## Deferred Matrices

After process-safe VM outcomes exist, scheduler experiments will compare OS
processes, native threads, cooperative instruction quanta, and epoll plus
quanta using identical mixed workloads. Baseline JIT candidates require typed
SSA, callable bounded code objects, exact outcomes, precise native stack maps,
and separate total/steady-state evidence. Loop OSR requires exact VM/SSA/native
state mapping. Proof-based optimizing JIT does not require general
deoptimization; guarded specialization does and remains a later separate gate.
## Disk Policy

Use one Cargo target directory, run variants sequentially, retain compact text
or structured summaries rather than build trees, keep at most two candidate
executables, run Docker only for final acceptance, and recheck free space after
each experiment batch.
