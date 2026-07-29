# Application Cells And Typed Host Providers

## Purpose

Define application-private host authority and trusted/process execution cells
without letting a backend reinterpret source or application code inherit daemon
privilege.

## Status

**Accepted Contract.** Arguments, stdio, clock, logging, cancellation, portable
relative paths, directory providers, database providers, and process cells
become Current only for each operation exercised through validated VM execution
and named isolation evidence. Existing direct host paths remain Current only
until their complete provider replacement passes equal gates.

## Composed Host Environment

`ExecutionInputs` carries one application-private `HostEnvironment`. It is a
composition of narrow provider families, never one giant provider interface:

```text
arguments
stdio
clock
logging
cancellation
directory
database
```

Entropy, network, terminal, and display join only with complete contracts. A
package capability list is a request. The daemon grant selects which provider
instances enter the cell. Absence fails before the operation; a provider cannot
manufacture another authority.

Every provider-created resource is typed, provider and application scoped,
incarnation safe, quota charged, and closed by structured cleanup. Raw host
paths, descriptors, pointers, database files, and capability secrets never
become source values.

## Resource Admission

One FIFO coordinator ticket orders invocation admission across all applications.
Admission must satisfy the coordinator concurrent/total ceiling and the app's
concurrent/total quota before one private invocation receives its execution
fuel, stack, frame, heap, allocation, handle, output, cleanup, and wall limits.
Completion or cell-boundary failure returns the coordinator active reservation.
No application owns a private worker pool or bypass lane.

`RuntimeAccounting` reports coordinator active, total, peak concurrent, and
configured limits. Focused multi-app evidence sets the coordinator concurrent
ceiling to one, admits one invocation from each of two applications without
starvation, observes peak one, then enforces the coordinator total ceiling.

## Portable Application Paths

An application-visible path is relative, UTF-8, segmented, and normalized. It
has no root, platform drive, empty segment, current segment, parent segment,
NUL, backslash alias, or trailing separator. It resolves only beneath one
directory provider. Native host paths remain provider-owned and never
participate in application identity.

A provider rejects symlink or replacement escape before effects. Native names
that cannot be represented by the portable contract remain opaque provider
entries until a separate source contract exists.

## Trusted In-Process Cell

A trusted in-process cell is legal only when all of these hold:

- package code is validated and consumes verified typed IR;
- every granted provider is reviewed as in-process safe;
- no foreign native code or ambient authority is present;
- VM, heap, resources, metrics, cancellation, and transactions are private;
- failure cannot corrupt another application; and
- cleanup is exact on every structured outcome.

The initial safe set may be smaller than the provider registry. Unsupported
grants fail closed. A traced guest reference never crosses an application
boundary.

## Isolated Process Cell

A capability-bearing application uses an isolated process cell until its exact
in-process boundary is independently proven. The Current Linux x86-64 cut
launches the fixed `lkjscript-cell` executable, not an arbitrary package command,
with a cleared environment and private framed pipes. Application stdio is
buffered in the worker and relayed through the app-private parent provider.

Current bootstrap binds platform revision, runtime-control digest, coordinator,
application, incarnation, package digest, canonical package entry, grants, and
execution limits. The worker verifies and compiles once before ready. Unknown,
partial, stale, or oversized messages fail closed, and outcomes preserve the
closed semantic result rather than a debug summary. Principal impersonation,
heartbeats, live cancellation messages, and provider proxies remain Accepted
Target. See [Isolated Process Cells](isolated-process-cells.md).

The coordinator launches app code in the target principal context. If the
coordinator has elevated installation identity and cannot establish that
context, start fails. Application code never executes inside the elevated
daemon.

## Interactive Process Cell

An interactive process cell is launched through an authenticated session
broker and owns the native event loop on the required thread. Frame-local input,
state update, layout, scene preparation, rendering, and presentation do not
roundtrip through the daemon. Display, window, GPU, input, accessibility, audio,
and frame resources remain session, application, and incarnation scoped.

## Crash And Stop

Cell loss marks only its incarnation failed, closes provider leases, aborts its
database transactions, records bounded cleanup, and applies bounded restart
policy. Other applications, daemon control, database recovery, and session
brokers remain usable.

Stop closes admission, sends cancellation, waits one bounded grace period,
then kills an unresponsive process cell according to policy. It validates exit
identity and outcome before lifecycle publication. Process exit is not ordinary
resource cleanup.

## Communication And Sharing

Cross-cell calls use exact typed interface digests and bounded frames. An
in-process direct call is legal only when both cells are trusted, grants pass,
cancellation and resource charging remain separate, and no mutable or traced
reference crosses.

Immutable validated chunks, verified SSA, code images, literals, and metadata
may share content-addressed leases. Mutable globals, stacks, heaps, providers,
transactions, resource tables, and metrics remain incarnation private.
