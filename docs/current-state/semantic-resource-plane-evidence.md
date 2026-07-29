# Current Semantic Resource Runtime Evidence

## Status

**Current measured slice.** This file describes implemented behavior, not every
Accepted Target in the semantic resource plane contract.

## Current

- `lkjscript-resource` provides generation-safe IDs, bounded CPU-list parsing,
  validated topology, exact task/access graphs, six deterministic policies,
  trace/replay, and deterministic failure selection.
- The real runtime uses scoped session workers, bounded lock-protected queues,
  bounded spin/park behavior, exactly-once task admission, stable failure
  publication, join, and zero-live-worker verification.
- Runtime observations include queue wait, task time, wakeups, parks, queue high
  water, same-group/cross-group/cross-NUMA steals, and exact outputs. Compiler
  metrics expose the exact resource-profile identity and all category totals.
  These observations do not change task legality or deterministic result merge.
- Linux discovery intersects online CPUs, process/thread affinity, effective
  cpuset, and cgroup quota. It reports uncertain or unavailable facts instead of
  inventing values. Affinity changes use safe wrappers in `lkjscript-linux-host` and
  require readback.
- The CLI exposes deterministic JSON topology, host scheduler, and resource-plan
  evidence for kernel-managed, CPU-pinned, and LLC-domain-masked plans.
- Owner homes use per-owner proof epochs and partitioned unique payload stores.
  Transfers require a fresh no-live-loan proof. Remote releases are bounded and
  home-drained; unrelated owner activity cannot stale a transfer proof.
- Forced multi-worker proof-edit discovery creates one task per verified SSA
  function. A serial coordinator partitions aggregate work/certificate grants
  and validates queue/scratch reservations before worker spawn. Local untrusted
  records merge stably; reconstruction and the independent checker are unchanged.
  Sequential remains the optimizer default.
- Scheduled generated scalar and unique byte-vector kernels execute through
  both forced native tiers with nonzero native entries, exact returns, zero VM
  fallback, zero collector activity, and zero final owner/loan/release backlog.
- One sealed generated image is shared by eight threads for 1,024 exact calls.
  Invocation state is per call, installer usage is synchronized, and dropping
  the last image owner returns executable accounting to zero.

## Measured Selection

All six runtime policies, three affinity modes, and six workloads were measured
at commit `52ddc5cbe3d95e455cd78afbab411a7a93c51596`. Corrected per-function proof
worker counts were measured at `3efc63a06f6908abea0ab4a994a32cf898590838`.
Owner compute had the best median normalized parallel p50 and is the runtime default. Kernel-managed placement is
the default because Linux retains placement authority and CPU pinning was highly
sensitive to a declared interference probe. Sequential proof discovery was
faster than 2- and 4-worker discovery on the retained one-function fixture.
Exact records are under `meta/results/resource-plane/`.

## Current Limits

- The measured host has one discovered LLC/NUMA group; cross-domain policy
  behavior is tested synthetically but not performance-validated on real
  multi-domain hardware.
- PMU, resctrl, DAMON, and OS migration observations were unavailable. They are
  not reported as zero.
- There is no source `spawn`, `await`, channel, detached work, or async surface.
- Elastic locality expansion/contraction, adaptive policy switching, blocking
  pools, and source structured concurrency remain Accepted Targets.

## Linux Boundary

Linux owns process/thread arbitration, preemption, physical CPU placement,
interrupts, memory policy, reclaim, thermal control, cgroups, and fairness among
unrelated processes. lkjscript owns verified logical task legality, bounded
worker admission, queues, task lifetime, owner homes, stable result publication,
and cleanup. Affinity is optional placement input, never a replacement Linux
scheduler.
