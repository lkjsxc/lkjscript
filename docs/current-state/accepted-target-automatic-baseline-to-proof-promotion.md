# Current State: Accepted Target: Automatic Baseline-To-Proof Promotion

[Authority](../current-state.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Accepted Target: Automatic Baseline-To-Proof Promotion

The next automatic promotion slice has an **Accepted Implementation Selection**,
not a Current implementation. It preserves the existing 64-VM-entry automatic
baseline threshold and keeps proof promotion CLI-opt-in and disabled by default
until retained adoption. Candidate optimizing thresholds count exactly 64,
256, 1,024, or 4,096 baseline entries of one promotion root. The Nth entry
synchronously proves, lowers, and W^X-installs an optimizing object but must
invoke the captured baseline object; only a later entry can publish the pending
optimized object.

Opaque entry tokens bind function, object, and tier. One process-local session
exactly owns coexisting baseline/optimizing objects, one current selection, and
at most one pending selection. Bounded stale invalidated objects remain owned
until drop but are never selectable. The states are `BaselineCandidate`,
`BaselineCompiling`, `BaselineNative`, `OptimizingCandidate`,
`OptimizingCompiling`, `OptimizingPending`, `OptimizingNative`, and `Disabled`.
There is one attempt per explicit epoch, a bounded total, same-epoch
suppression, structured tier failure with baseline retained, and epoch-driven
optimized invalidation/retry back to baseline.

Auto entry remains scalar-only; source main remains in the VM. Generated
reference helpers may call and allocate inside their native group but cannot
transfer references at VM/native entry or become auto roots. Forced tiers remain
unchanged and fallback-free. Required evidence records thresholds/enables,
epochs, attempts/failures/suppressions, transitions, exact tier entries and
objects/code bytes, baseline entries and time before first optimized entry,
proof/W^X, stale invalidations, and bounded mappings/work/certificates/metadata.

The predeclared clean locked release gate randomizes at least four warmups and
31 samples for auto baseline-only, thresholds 64/256/1,024/4,096, unchanged
forced sentinels, and allocation/reference correctness. Adoption requires at
least 1.10x median process speedup, improvement greater than twice combined
MAD, p95 no more than 5% worse, compile cost repaid, exact oracle/stream/state/
proof/W^X results, historical scalar native and process medians within 5%, and
no repeated attempt or fallback. The largest passing candidate within twice
combined MAD of the fastest passing median is selected. Otherwise optimizing
stays disabled and the complete rejection is retained. See [Proof-Based
Optimizing JIT](../decisions/jit/proof-based-optimizing-jit.md) for the authoritative
contract.

Longer-term accepted sequences for [staged self-hosting](../decisions/platform/self-hosted-platform-roadmap.md),
[modules and reproducible packages](../decisions/platform/modules-and-packages.md),
[isolates and structured concurrency](../decisions/platform/isolates-and-structured-concurrency.md),
[the Web platform](../decisions/roadmaps/web-platform-roadmap.md), and [the first-party
relational database](../decisions/roadmaps/relational-database-roadmap.md) are explicitly
not Current implementation claims.
## Known Defects

The source identity cutover does not make the runtime semantically complete.
The highest-priority defects are:

1. some library file operations remain per-byte or quadratic; application-level
   storage recovery is a language-consumer responsibility;
2. source files, aggregate closure bytes, and source-unit counts now have high
   implementation maxima, but aggregate import edges, schema nodes, parser/type
   work, compiler memory, and compiler wall time do not yet have named profile
   budgets; bytecode tables/data/code/metadata and VM execution resources are
   bounded;
3. cooperative deadlines can overrun inside filesystem, console-write,
   send/write, terminal-cleanup, or other non-cancellable wrappers;
   hard-deadline mode reports those operations as unsupported `HostFailure`
   before effects; live-heap accounting is estimated at VM instruction
   boundaries, and `print` builds its host-format string before the output check;
4. stdin/stdout and the terminal guard remain process-global, so concurrent VM
   supervision is unsupported; handle metadata is VM-local and bounded but
   monotonically allocated until that VM ends.
