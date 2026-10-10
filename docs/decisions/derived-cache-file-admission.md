# Derived-cache file admission must not wait for a FIFO peer

## Boundary and defect

The accepted meaning graph and its publication receipt own program meaning. The
compiler cache is derived state: a failed cache observation does not refuse an
otherwise valid semantic publication, and an accepted idempotent retry keeps its
original result rather than applying the change again.

Opening the cache's `derived/compiler/CURRENT` with blocking read-only access
violated that separation before the existing file-type check could run. A FIFO
without a writer blocked inside `openat`. This prevented `check` from reaching its
normal cache-recovery path and prevented `change apply` from reaching publication
or an already-accepted result. Checking metadata after a blocking open does not
establish prompt rejection of an unsuitable entry.

The copied pre-fix executable reproduced the defect in an owned disposable Command
project: a valid baseline completed, replacing only CURRENT with a writerless FIFO
made check reach the test's five-second timeout, and restoring the original regular
file recovered normal checking. The timeout is a failed predecessor observation,
not evidence that the cache was rejected or that any interrupted change rolled back.

## Selection

Open the candidate descriptor with `O_NONBLOCK` as well as the existing `O_NOFOLLOW`
and `O_CLOEXEC`, then retain the existing descriptor-based regular-file admission.
Do not read from a FIFO or reinterpret it as an absent cache. It produces the exact
Corrupt diagnostic `compilation_cache_regular_type`. A missing entry remains absent;
regular bytes, canonical decoding, revision binding and symlink refusals are unchanged.
There is no pathname precheck that can be exchanged for another type before opening.

On the supported Linux host, nonblocking read-only FIFO open does not wait for a
writer. This flag does not make ordinary regular-file I/O asynchronous or impose a
hard latency bound. Descriptor-based admission also does not establish hostile
filesystem containment, immunity to concurrent regular-file mutation, or bounded
storage/lock latency. Those are separate contracts, not claimed by this correction.
The [Linux open contract](https://man7.org/linux/man-pages/man2/open.2.html) and
[FIFO contract](https://man7.org/linux/man-pages/man7/fifo.7.html) describe that scope.

The existing clean-build recovery may replace a bad CURRENT atomically after
constructing valid derived output. Observing the bad entry during change application
instead reports a failed derived cache alongside the independently accepted semantic
change. An idempotent replay neither repairs the cache nor changes the original
receipt; subsequent explicit checking owns cache recovery.

## Evidence obligations

A bounded, joined library subprocess must reach the exact corrupt-type diagnostic
for a writerless FIFO and leave its device/inode intact. A completion marker prevents
an unavailable executable or zero-test subprocess from counting as success. Missing,
empty and nonempty regular files, directories, and ordinary/dangling symlinks retain
separate controls. No writer is introduced to make the FIFO test finish.

Copied-product regressions must demonstrate both ordinary check recovery and semantic
acceptance with a failed derived cache. They retain original accepted receipts across
replay, unchanged files and FIFO identity during replay, exact-current subsequent
checking, deterministic complete artifact bytes, the original Command result and a
subsequent incremental edit. Owned children have bounded observation and kill/join
cleanup on assertion failure; a timeout is never accepted as a product refusal.

These public tests live under the already selected
`native_declarations::incremental_units::` family, whose source-matched final-byte
owner selects all present and future cases. No new acceptance generation, public
syntax, product identifier, semantic encoding or artifact encoding is needed for
this additional family coverage. Exact source and executable availability remain
[status-owned](../status.md); this document alone is not acceptance or publication.

## Long-term consequence

Keep finite, independently checkable failure at bootstrap boundaries while moving
compiler and tooling semantics into lkjscript. A native pass cannot repair an outer
host operation that blocks before admitting its input. This correction strengthens
that host boundary; it is not a native compiler pass, a scheduling model or proof
of complete self-hosting. Replace host mechanisms when a native successor owns the
same useful semantics and failures, rather than hiding them behind successful demos.
