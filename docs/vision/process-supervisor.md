# Process supervisor

## Purpose

Define the experimental path from one VM per OS process to one runtime per
machine hosting many independent LKJML processes.

## Status

This is a staged product plan, not current behavior. The CLI currently compiles
one entry and runs one synchronous VM in one OS process.

## Product contract

A future machine runtime should:

- own a single local control endpoint and reject a second daemon;
- run arbitrary numbers and kinds of LKJML entries, bounded by configured
  resources rather than a hardcoded process count;
- isolate globals, stack, heap, handles, arguments, environment, working
  directory, output, cancellation, and failures per logical process;
- keep one process's `exit`, VM error, blocked IO, or terminal use from killing
  or stalling unrelated processes;
- cache immutable compiled chunks by source/import content hash;
- make lifecycle state and failure reasons observable without reading raw logs;
- preserve a foreground mode that feels as direct as today's `run` command.

## Control UX

The target command vocabulary follows familiar process tools while keeping one
obvious path for each job:

```text
lkjscript2026 run main.lkjml             foreground, ephemeral
lkjscript2026 start main.lkjml --name api
lkjscript2026 ps
lkjscript2026 logs api --follow
lkjscript2026 stop api
lkjscript2026 restart api
lkjscript2026 inspect api
lkjscript2026 daemon
```

Human output should be a stable table with name, state, entry, uptime, restart
count, CPU/fuel, heap, and last failure. A versioned machine-readable mode must
exist before external UI clients. Interactive terminal ownership is an
exclusive lease; background processes never write directly to the terminal.

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

A persisted `ProcessSpec` needs entry, arguments, environment, cwd, process
kind, resource limits, and restart policy. Runtime state and open handles are
not checkpointed initially. On daemon restart, only specs explicitly marked
persistent are started again. Restart backoff must be bounded and visible so a
crash loop cannot consume the machine silently.

## Security boundary

The first local daemon is not a sandbox. Its control socket must use filesystem
permissions, and process specs must not imply protection from malicious source.
Capabilities for filesystem roots, network listeners, environment, and host
operations should be explicit before accepting untrusted workloads.

## Acceptance workload

A non-mock milestone concurrently runs:

- one CPU-bound numeric process;
- two HTTP processes on different ports;
- one sleeping worker;
- one foreground editor with the terminal lease.

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
7. Only then make the daemon the default Docker/native product shape.
