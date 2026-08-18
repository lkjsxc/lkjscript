# Future evidence gates

This file contains only unresolved consumer-driven gates. Implemented reality is in
`docs/status.md`; normative contracts are under `docs/spec/`; measurements are in
`docs/performance.md`. An absence below is not a permanent prohibition.

## Additional host interfaces or reusable interface artifacts

Trigger: a third independent production adapter/application cannot use the two closed built-in
interfaces without runtime vocabulary growth or duplicated exact interface declarations.

Compare another narrow built-in, exact release-export identity, and a dedicated immutable interface
artifact. Preserve application-owned nominal commands/outcomes, exact slot routing, immutable grant
binding, pure replay, fake/production disjointness, and no mutable registry. Delete an interface
artifact or catalogue if it lacks independent distribution/binding consumers.

## Grant rotation or revocation

Trigger: a current long-lived instance must change authority without creating a replacement
instance.

Specify grant revision identity/publication, attenuation, revocation, execution versus
reconciliation authority, pending commands, restart, audit, and unknown-outcome behavior. Compare a
new instance, explicit immutable grant revision, and named external lookup. Never let mutable lookup
silently broaden accepted authority.

## Parallel or unattended operation

Trigger: measured independent instances need concurrency, or a retained product needs durable work
after the caller exits.

First compare per-instance locks plus metadata lock, one bounded transition pool, and the current
store-wide serial kernel. Per-instance order and cross-process exclusion remain exact. A durable job
requires a separate durable authority; it must not be smuggled into an in-memory queue. Do not add
mid-transition preemption, actors, async runtime, worker pool, or timers without a concrete
workload and shutdown/replay contract.

## Resident supervisor

Trigger: a real multi-client deployment or aggregate admission requirement is not served by the
caller-owned foreground session.

Compare explicit foreground ownership, Unix socket under private permissions, and no supervisor.
Require at least 35% additional complete-workflow benefit or an otherwise impossible current
coordination need, plus exact framing, peer/deployment authorization, bounded connections,
backpressure, stale socket/singleton recovery, shutdown, upgrade, and diagnostics. Peer UID is not
an application grant and the process boundary is not a sandbox. Delete a losing socket prototype.

## Application/Core cache

Trigger: release-profile stage distributions show application decode, graph validation, flattening,
lowering, or Core verification costs at least 20% of a complete repeated workflow and a bounded
cache improves the whole workload by at least 20%.

Compare no cache, a session-only independently validated application load, verified Core units, and
only then persistent cache. Bind all correctness inputs and keep miss/eviction semantically
invisible. Persistent storage additionally requires hostile decode, atomic publication, target/
compiler/representation binding, bounded eviction, and restart benefit that repays lifecycle cost.
No current cache format exists.

## Execution tier

Trigger: execution—not loading, replay, publication, or host blocking—dominates a representative
application and an acceleration improves complete cold/warm work by at least 30% after compile cost.

Compare interpreter dispatch improvements, compact verified bytecode, one baseline compiler, and no
new tier. An optimizing JIT requires a stable hot workload beyond that gate. Preserve values, traps,
fuel, frames, managed-byte semantics, diagnostics, and the explicit-frame differential oracle.
Generated native code additionally needs platform/feature/ABI/trap/W^X/crash contracts and bounded
code memory. Delete losing formats and dependencies.

## State/history compaction

Trigger: at least two representative histories cross explicit retained-byte or replay service
thresholds; current small full records are insufficient evidence.

Measure state size, history growth, replay, corruption traversal, backup, and recovery. Compare
checkpoints plus exact unresolved evidence, append-only journal, content-addressed state, and full
records. Preserve event-key behavior, pending attempts/outcomes, exact replay, and an independent
oracle. Delete the losing format and recovery paths.

## Application state migration

Trigger: one continuity identity must survive an incompatible exact application/state-type change.

Compare a replacement instance, explicit pure old/new migration artifact, and exact rebinding. Any
retained migration names exact old/new applications, state mapping, failure publication, and
rollback. Mutable `latest` resolution and compatibility readers remain inadmissible.

## Host isolation and multi-user deployment

Trigger: untrusted native adapters, broad filesystem/network/process authority, or multiple OS users
enter the supported boundary.

Write the threat model first. Compare in-process validation, a minimal worker with enumerated
descriptors/environment/mounts/signals/limits, WASI components, containers, and Linux-specific
controls. Process separation and containers are not automatically sandboxes. Specify deployment
authentication separately from application grants and measure startup, IPC, recovery, and residual
trust.

## Secrets and confidentiality

Trigger: a retained adapter requires credentials or durable state exceeds local-filesystem
confidentiality assumptions.

Separate named secret grants, redaction, memory lifetime, state encryption, key management, backup,
and semantic validation. No secret may enter release/application artifacts, prompts, diagnostics,
profiles, fake fixtures, or benchmark evidence. Delete secret/encryption prototypes without a
current consumer.

## Runtime resource expansion

Trigger: a retained deployment needs exact aggregate memory, open-file, disk-temporary, CPU, or
connection governance beyond current logical/synchronous admission.

Name accounting unit, owner, reservation/release, peak/retained distinction, overload class,
restart, and observation. RSS remains noisy observation unless an OS containment policy is selected;
cgroups/service-manager limits cannot replace semantic fuel/state/history bounds. Do not add a
resident process solely to make counters global.

## Contract ownership derivation

Trigger: frozen equal changes repeatedly require broad source reads or duplicate facts across the
global workspace and command-local contract owners.

Compare current manual owners, a table/macro, proc macro, and narrower local ownership. Measure
source opened, diagnostics, expanded output, build cost, and deterministic task success. A generator
loses if its output becomes another mutable authority.

## Managed immutable bytes

Trigger: a durable/compute workload reverses the current copy/peak-byte benefit or planner/verifier
maintenance dominates semantic changes.

Compare the verified plan, allocate-new oracle, safe reference-counted bytes, and a bounded arena.
Preserve identical values, traps, fuel, visible/retained accounting, and cleanup. Retain one
production route and one simple oracle.

## Provider-backed agent trials

Trigger: the harness exposes stable model identity, attempts, token classes, cached-input telemetry,
and pricing, or an authorized independent runner becomes available.

Run frozen create-interface/import/application/grant and denied/unknown/reconcile/resource/cache
diagnosis tasks. Deterministic validators decide correctness. Record actual provider telemetry only;
never infer tokens or cost from UTF-8 bytes. Delete losing syntax/help/schema surfaces.

## Fuzzing, Miri, sanitizers, and model checking

Trigger: the relevant tool is available in CI/local execution or a new unsafe/native/concurrent
boundary raises risk.

Keep deterministic mutation and publication-state enumeration as baseline. Fuzz hostile public
decoders and turn findings into focused regressions. Miri/sanitizers target application value
ownership, instance/adapters, and runtime/session framing. Retain no stale unlicensed corpus or model
whose production mapping can drift.

## Platform, CI, and distribution expansion

Trigger: a concrete user needs another OS/architecture/filesystem, collaboration CI, or public
installation.

List rename, hard-link, permission, locking, sync, and path assumptions. Compile success is not
support. Run public workflows and relevant crash/corruption tests on each claimed platform.
Separately specify binary provenance, installation, rollback, and format rejection. Do not add
signing, packaging, release services, or network dependencies before naming the consumer/operator.
