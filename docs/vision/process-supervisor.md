# Process supervisor

## Purpose

Define the experimental path from one VM per OS process to one runtime per OS
user hosting many independent LKJML processes. One Docker container is one
runtime domain and is treated as one machine for this contract.

## Status

**Experimental.** This is a staged product plan, not current behavior. The CLI currently compiles
one entry and runs one synchronous VM in one OS process. Linux is the first
production target. The intended default is daemon-backed execution; a direct
standalone path may remain for bootstrap, recovery, and diagnostics.

## Accepted Direction

- Remain Linux-first until the daemon and scheduler are operationally sound.
- Make normal execution daemon-backed; standalone execution is explicit
  recovery tooling, not a second default runtime model.
- Run exactly one daemon per OS user; do not introduce machine-wide multi-user
  privilege management into the initial product.
- Treat one Docker container as one machine/runtime domain, with one daemon
  managing all logical processes inside that container.
- Optimize the singleton for resource efficiency and aggregate performance
  while preserving strict logical-process isolation.
- Use `lkjedit` only as an in-tree validation application and prepare it for a
  future standalone repository.

## Why One Runtime

The singleton is a performance and resource decision, not only a management
convenience. A per-user runtime can share immutable compiled chunks and
import caches, centralize epoll and timers, avoid one scheduler and service
layer per application, and enforce global CPU, memory, handle, and output
budgets. Cooperative quanta also avoid requiring one permanently active native
thread for every mostly idle process.

Logical processes still require isolated globals, stacks, heaps, handles, and
failures. The singleton must not become one global mutable heap or a serial
bottleneck. Measurements must compare resident memory per idle process,
throughput, latency, scheduler overhead, and cache reuse against separate OS
processes and the thread-per-VM prototype.

## Product contract

A future per-user runtime should:

- own a single local control endpoint and reject a second daemon;
- run arbitrary numbers and kinds of LKJML entries, bounded by configured
  resources rather than a hardcoded process count;
- isolate globals, stack, heap, handles, arguments, environment, working
  directory, output, cancellation, and failures per logical process;
- keep one process's `exit`, VM error, blocked IO, or terminal use from killing
  or stalling unrelated processes;
- cache immutable compiled chunks by source/import content hash;
- make lifecycle state and failure reasons observable without reading raw logs;
- make ordinary `run` daemon-backed while preserving direct foreground UX;
- retain an explicit standalone recovery path rather than silently starting a
  second independent runtime for the same OS user.

## Control UX

The target command vocabulary follows familiar process tools while keeping one
obvious path for each job:

```text
lkjscript run main.lkjml             foreground, ephemeral
lkjscript start main.lkjml --name api background, non-persistent
lkjscript deploy main.lkjml --name api persistent and started
lkjscript undeploy api                stop and remove persistent spec
lkjscript ps
lkjscript logs api --follow
lkjscript stop api
lkjscript restart api
lkjscript inspect api
lkjscript daemon
```

`run` should ensure the singleton daemon exists, submit an ephemeral process,
and attach its streams. It must not silently create an independent runtime when
the daemon is unavailable. An explicit `run --standalone` recovery mode may
bypass the daemon for diagnostics and repair. If an attached `run` client
disconnects, its process is cancelled. `start` is detached but non-persistent;
`deploy` is the separate operation that stores a persistent specification and
starts it. `undeploy` stops that process and removes its stored specification.

`restart` always resolves imports and recompiles the latest source from disk;
it does not reuse the old process's source snapshot. A daemon restart applies
the same rule to every deployed specification.

Human output should be a stable table with name, state, entry, uptime, restart
count, CPU/fuel, heap, and last failure. A versioned machine-readable mode must
exist before external UI clients. Protocol versions must fail clearly when
mismatched; backward compatibility is not required, and obsolete protocol
paths must be deleted rather than retained as shims. Interactive terminal
ownership is an exclusive lease; background processes never write directly to
the terminal.

## Required VM boundary

The daemon must not wrap the current blocking `Vm::run` and call that complete.
First introduce a process-safe execution API:

```text
run_quantum(max_ops) -> Running | Blocked | Completed | Exited | Failed | Cancelled
```

Required supporting changes:

1. Return `Exited(code)` instead of calling `std::process::exit`.
2. Move stdin, stdout, stderr, cwd, environment, clock, filesystem, network,
   and terminal access behind per-process host services.
3. Separate table handles from raw OS descriptors and reject stale generations.
4. Add instruction, heap, handle, output, and wall-time budgets.
5. Represent blocking operations as scheduler-visible waits.
6. Make terminal restoration a lease responsibility, not process-global state.

## Scheduler experiments

| Approach | Useful experiment | Adoption condition |
| --- | --- | --- |
| One native thread per VM | Fastest path to validate isolation and lifecycle UX. | Transitional only; measured memory/thread limits must fit intended scale. |
| Cooperative instruction quanta | Deterministic fairness for CPU-bound VMs. | Adopt when cancellation latency and throughput beat thread-per-VM. |
| Nonblocking epoll plus quanta | Scalable Linux IO and timers in one runtime. | Linux production target after host effects can return `Blocked`. |
| Async Rust framework | Broad ecosystem and portability. | Rejected by default because it violates the scratch-host law unless measured value justifies an ADR. |
| OS process supervisor | Strong isolation with existing VM unchanged. | Keep as a fallback mode, not the one-runtime architecture. |

The first implementation should compare native-thread and cooperative-quantum
variants with the same workload. The eventual Linux design is likely epoll for
IO plus round-robin VM quanta; a thread pool can absorb operations that cannot
yet be made nonblocking.

## Persistence and restart

`start` never persists a process specification. `deploy` stores a
`ProcessSpec` containing entry, arguments, environment, cwd, process kind,
resource limits, and restart policy, then starts it. `undeploy` stops the
process and removes that specification. Runtime state and open handles are not
checkpointed. On daemon restart, only deployed specs are resolved and compiled
from the latest source before starting again. Restart backoff must be bounded
and visible so a crash loop cannot consume the machine silently.

## Security boundary

The first local daemon is not a sandbox. Its per-user control socket must use
filesystem permissions, and process specs must not imply protection from
malicious source.
Capabilities for filesystem roots, network listeners, environment, and host
operations should be explicit before accepting untrusted workloads.

## Acceptance workload

A non-mock milestone concurrently runs:

- one CPU-bound numeric process;
- two HTTP processes on different ports;
- one sleeping worker;
- one foreground `lkjedit` validation process with the terminal lease.

The test must list all processes, stream distinct logs, stop and restart each
one, enforce a cancelled CPU process within its latency budget, and prove that
a failing process leaves the daemon and siblings alive. Measurements include
startup latency, scheduler fairness, resident memory per idle VM, throughput,
GC pause, cancellation latency, and descriptor cleanup.

## Sequence

1. Land process-safe outcomes and instruction quanta with focused VM tests.
2. Introduce host-service interfaces and typed generation handles.
3. Run multiple VMs in-process with thread-per-VM and quantum prototypes.
4. Measure the acceptance workload and retain both results.
5. Add the local control socket and lifecycle commands.
6. Add nonblocking Linux IO, timers, quotas, logs, persistence, and restart.
7. Make the daemon the default Linux Docker/native product shape while keeping
   standalone execution explicit for recovery and diagnostics.
