# Isolated Process Cells

## Status

**Current.** Linux x86-64 execution is covered by the named evidence below.

## Decision

An installed application declares exactly one execution cell class:

- `trusted-in-process` runs one fresh validated VM per invocation inside the
  coordinator process; or
- `isolated-process` runs behind one coordinator-owned worker process per
  application incarnation.

The class is part of the application manifest. Installation code and manifest
class must match. There is no automatic fallback from one class to the other.

## Deployment Binding

An isolated package declaration contains a normalized relative Semantic Source
entry. Installation separately binds the package root and the coordinator's
fixed worker executable. An application cannot choose an executable, command
line, environment, or loader search path. The coordinator resolves the entry
beneath the package root before publication.

The worker executable identity is coordinator configuration. Production service
launchers pass one absolute reviewed worker path. Tests may bind the Cargo-built
worker explicitly.

## Process Boundary

The coordinator starts a worker with cleared environment, piped standard input,
piped standard output, and null inherited standard error. Application stdio is
a private buffered provider and never shares the protocol stream.

Messages use a four-byte little-endian body length followed by one closed typed
body. The maximum body is 8 MiB. Strings are exact UTF-8 with bounded lengths.
No unknown tag, trailing byte, incomplete frame, identity mismatch, stale
incarnation, or oversized field is accepted.

The closed message flow is:

1. coordinator sends `bootstrap`;
2. worker verifies and compiles the package entry once, then sends `ready` or a
   bounded `ready-failure`;
3. coordinator sends ordered `invoke` messages and receives one exact `outcome`
   for each execution-cell identity;
4. coordinator sends `stop`, worker replies `stopped`, and exits.

Bootstrap binds the platform revision, runtime-control contract digest,
coordinator identity, application identity, incarnation, package content
identity, resolved package entry, exact capability grants, and execution
limits. An outcome binds the same incarnation and exact execution-cell serial.

## Bounds

- frame body: at most 8 MiB;
- package entry: at most 4 KiB encoded bytes;
- invocation arguments: at most 256;
- one argument: at most 4 KiB;
- aggregate argument bytes: at most 256 KiB;
- one buffered application output: at most 1 MiB;
- one diagnostic: at most 4 KiB;
- graceful stop interval: coordinator-configured and bounded at two seconds.

Bounds are checked before a frame or application fact is published. Partial
writes and reads are completed exactly or fail the cell.

## Semantic Parity

The worker consumes the same resolved compiler pipeline and validated bytecode
VM as trusted execution. It creates private invocation inputs and VM state for
every request. Capability grants and resource limits are identical semantic
inputs in both cell classes.

Process transport preserves every closed `ExecutionOutcome`, including owned
returned values, structured resource limits, host failures, and retained cleanup
failures. Debug summaries are not a semantic transport.

## Lifecycle And Failure

A worker becomes visible as `running` only after a matching `ready`. Bootstrap,
compile, protocol, EOF, or unexpected-exit failure marks only that incarnation
`failed`, reaps the child, decrements active accounting, and wakes waiters.
Other applications remain runnable.

Stop closes admission, waits for admitted invocations, requests graceful worker
stop, and kills then reaps the child if the bounded interval expires. Restart
creates a new application incarnation and worker process. Requests using the old
incarnation fail as stale.

Per-application tickets, concurrency limits, total invocation limits, metrics,
and bounded logs apply before either execution-cell class dispatches.

## Current Cut

The dependency-free framed protocol, lossless closed outcome codec, fixed
`lkjscript-cell` worker, manifest-class checks, supervisor lifecycle, private
stdio relay, bounded startup/stop, restart, and per-app crash isolation are
Current on Linux x86-64. Focused tests execute validated Semantic Source twice,
replace the worker on restart, reject stale incarnation use, kill one worker,
and continue executing another application.

Daemon-persistent isolated-app reconstruction and application control are
Current for the fixed sibling worker and exact registry evidence. Non-Linux
execution, OS sandbox profiles, trusted-artifact reconstruction, and provider
families beyond arguments, direct stdio, and clock are not Current.
