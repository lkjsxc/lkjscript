# Semantic Resource Plane
## Purpose
Define one bounded in-process authority for verified task legality, deterministic
resource outcomes, and replaceable topology-aware placement.
## Status

<!-- LKJ-STATUS id=semantic-resource-plane status=accepted-contract -->

**Accepted Contract.** The contract is binding before implementation. No
scheduler, topology query, parallel optimizer, or source concurrency capability
is Current merely because this decision exists.

## Boundary With Linux
Linux owns system-wide process and thread arbitration, preemption, interrupts,
page faults, physical allocation, reclaim, devices, filesystems, protection,
cgroups, cpusets, thermal safety, and fairness among unrelated processes.
lkjscript owns logical task formation, verified dependencies, admission,
structured scopes, bounded worker queues, task lifetime, memory homes, owner
transfer, result publication, cleanup, per-scope budgets, and task metrics.

A worker is one session-owned OS thread. Linux schedules workers; lkjscript
schedules verified logical tasks onto them. Ordinary correctness requires no
root privilege, debugfs, custom kernel, `sched_ext`, resctrl, DAMON, PMU, or
NUMA-policy permission. Missing observations reduce optimization only.
## Semantic Authority And Policy
`lkjscript.semantic-resource-plane` separates `VerifiedTaskGraph`,
`HardwareTopologySnapshot`, `ExecutionResourcePlan`, `SchedulePolicy`, and
`ScheduleDecision`. The graph determines legality from compiler-derived dependencies, exact
read/write/consume/produce/identity-only accesses, effects, capabilities,
ownership, cleanup, scope, portability, and result ownership. The policy chooses
workers, queues, affinity, steal victims, locality sets, and homes. Policy may
change performance but never meaning, proof authority, failure identity,
resource outcomes, cleanup, or artifact bytes.

Initial stable identities are `ResourcePlaneId`, `TaskClassId`, `TaskId`,
`TaskScopeId`, `TaskResultId`, `DataOwnerId`, `AccessRecordId`, `WorkerId`,
`WorkerGroupId`, `ExecutionDomainId`, `SchedulePlanId`, and
`SchedulerPolicyId`. Recyclable runtime identities include a generation.

The task-graph identity includes its verified compiler/SSA input, classes,
instances, dependencies, accesses, results, scope, and ceilings. It excludes
placement. A schedule-plan identity adds topology, worker plan, policy, and
parameters.

## Hardware Topology
The bounded model can represent machine, package, die, chiplet, NUMA node, LLC
domain, cache, core, processing unit, and memory node. Unknown remains unknown;
chiplet, LLC, and NUMA are not aliases.

Processing-unit facts include Linux CPU ID, online/allowed/cpuset state,
package/die/core identities, SMT siblings, cache-sharing sets, LLC and NUMA
membership, and reliable capacity class when exposed. Cache facts include
level, kind, ID, size, line size, associativity, and sharing mask. NUMA facts
include CPU and allowed-memory masks, capacity, and distance rows.

Locality is a deterministic relation: same processing unit, SMT sibling, core,
private cache, L2, LLC, chiplet, NUMA node, package, or remote NUMA. It is not
one forced tree. SMT proximity and execution contention are separate facts. Bounded Linux
CPU-list/map parsing rejects overflow, descending ranges, duplicates, malformed
separators, excessive entries, and empty required masks. Sysfs discovery never
follows an escape outside its anchored roots.

## Linux Scheduler Observation
Read-only evidence records kernel release, process/thread affinity, effective
cpuset, scheduler policy, NUMA balancing, sched_ext state and active scheduler,
and `CONFIG_SCHED_CACHE` or relevant debug state only when directly readable.
Every fact has source and certainty. Kernel version alone never establishes
Cache-Aware Scheduling. Ordinary execution never writes kernel-global controls.

Linux Cache-Aware Scheduling observes kernel tasks and shared address spaces;
it cannot see lkjscript task owners, privileges, dependencies, grain, results,
or cleanup. lkjscript complements it with exact semantic facts and does not
reproduce an RSS heuristic.

## Verified Task Graph
A logical task is a typed static internal operation, not an arbitrary untyped
closure. It records class and generation-safe ID, exact input/output owners,
dependencies, scope, effects, capabilities, blocking/trap/divergence facts,
portability, expected work/working set, criticality, and compiler origin.

Initial accesses are `read`, `write`, `consume`, `produce`, and
`identity-only`. Read/read is compatible. Write conflicts with overlapping
read/write. Consume conflicts with every live access. Produce remains private
until publication. Identity-only grants no payload access. Unknown or external
access is serial unless an exact type contract proves otherwise.

A unique owner crosses a task boundary only by move after proving no live loan.
The producer loses it before publication; one consumer gains one obligation.
No live borrow crosses workers in the initial slice. Every task belongs to a
scope that joins or cancels all children; detached tasks do not exist.

Construction and verification are independent traversals. Verification proves
IDs, acyclicity, access compatibility, exact moves, scope containment, result
ownership, portability, nonblocking compute work, ceilings, and cleanup. No
scheduler receives an unverified graph.

## Deterministic Resources And Failure

Admission pre-reserves descriptor, dependency/access records, result slot,
queue entry, scope child, transfer/cancellation state, and task-local budget.
Failure publishes nothing. Every queue and pool is bounded.

Tasks journal resource work locally. Compiler journals merge in stable task-ID
order, and aggregate exhaustion uses the stable accepted prefix. A shared
racing decrement cannot select semantic failure. Primary task failure is the
lowest stable scope/task order after shutdown; additional task and cleanup
failures remain bounded, ordered, and distinct.

## Resource Plan And Scheduling

The planner intersects topology, effective affinity/cpuset, resource profile,
requested parallelism, graph shape, deterministic mode, workload class, and
available memory. It prefers unused physical cores before SMT, starts with a
compact reliable locality set, caps workers by tasks/profile, and uses one
worker for single-core or unknown topology.

Supported affinity plans are `kernel-managed`, `cpu-pinned`, and
`llc-domain-masked`; a strict mode is legal only after effective-affinity
readback. A worker group is one exact LLC or closest reliable domain with its
workers, mask, queue, capacity, and known NUMA/die facts.

Complete policy candidates are `sequential`, `static-partition`, `global-fifo`,
`local-work-stealing`, `hierarchical-locality`, and `owner-compute`. The
single-thread reference scheduler has exact states, stable ready order, virtual
workers/homes, bounded decision traces, replay, cancellation, and deterministic
failure. The real runtime uses scoped session-owned workers, generation-safe
descriptors, bounded lock-protected deques, bounded spin then park, wakeup,
join, and zero-live-state verification. Compute workers do not perform blocking
host I/O in the initial slice.

Elastic locality sets are soft, compact, demand-sized, bounded, expandable, and
contractible. Hierarchical stealing increases topology cost from same group to
LLC, die/chiplet, NUMA, then remote NUMA. Hysteresis, minimum residence,
cooldown, score-improvement thresholds, and age/service debt prevent ping-pong
and starvation. Critical-path delay may outweigh locality.
## Memory Homes

Task descriptors, topology, workers, queues, traces, metrics, and scheduler
state never use `GcHeap`. Each worker owns first-touched bounded scratch. A data
owner may record home worker/group/LLC/chiplet/NUMA, last worker, and last-use
epoch; these are placement facts, not artifact identity.

Owner-compute prefers moving computation to the dominant owner. A unique-owner
transfer records owner, source/destination tasks and homes, no-live-loan proof,
backing-transfer policy, and resource charge. Proof freshness is per owner, so
unrelated owner activity cannot spuriously stale a valid transfer. Remote destruction uses one
bounded home release or a separately verified direct release. Static bytes are
readable everywhere; dynamic bytes remain affine without implicit copy or RC.

The first measured memory candidate is a worker-sharded `UniqueStore`; a
partitioned session store is the complete comparison. Only one production store
may survive adoption. Live borrowed data is never migrated.

## First Integrations And Adoption

The first compiler task class creates one proof-edit task for every verified
SSA function from immutable function-local facts. Before dispatch, the serial
coordinator partitions aggregate work, record, and byte grants and validates
queue and scratch reservations. Tasks return untrusted local records and
journals. Stable merge orders function, block, value, and edit kind, then assigns
global sequence. Candidate reconstruction and the independent checker remain
sequential proof authority. Sequential and scheduled outputs must be equal.

A second suite schedules actual installed collector-free native scalar/bytes/
byte-vector kernels. Acceptance requires nonzero native entries, exact output,
zero VM fallback, zero collector construction/allocation/collection/root/
barrier activity, and zero owner/task/cleanup leaks. Sealed installed images
are immutable shared code: concurrent calls use independent invocation and
runtime-service state, synchronized lease accounting, and last-owner teardown.

All policy comparisons use one commit, binary, host configuration, declared
schema, warmup/sample count, and unchanged gates. Retained evidence records wall
time, throughput, p50/p95/p99, queue delay, scheduler time, utilization, parks,
wakeups, locality/steal distances, migrations, transfers, remote releases,
copied bytes, peaks, and output identity. Optional counters report unavailable,
never false zero.

A parallel/locality policy becomes default only after exact correctness and
same-commit evidence with bounded overhead, acceptable p99/fairness, safe
unknown-topology fallback, and no unexplained catastrophic regression. The
resource runtime default is `owner-compute` with `kernel-managed` placement;
small proof-edit discovery remains sequential by default, while forced
multi-worker discovery uses static partitions. These are separate defaults:
owner-compute was the lowest aggregate parallel policy in the first retained
mixed-workload comparison, but scheduling overhead made sequential discovery
faster on the first one-function optimizer fixture. Negative evidence is
retained and losing production hot paths are removed.
## Source Concurrency And Rejections

This internal plane does not add source `spawn`, `await`, channels, async, or
detached work. A later source slice requires exact static calls, accesses,
moved/immutable arguments, no crossing live loan, result ownership,
cancellation, cleanup, bounded admission, and reference/native semantics.

Rejected are unbounded/global queues, legality by annotation or AI hint,
parallel unknown effects, racing budgets/failures, execution-order artifacts,
silent owner copy, NUMA/LLC conflation, blind logical-CPU use, required
privilege, process-global scheduler ownership, scheduler metadata in `GcHeap`,
and sched_ext as language semantics.
