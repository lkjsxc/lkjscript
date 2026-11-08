# Reuse one checked package projection per artifact link

Date: 2026-11-08

## Problem and scope

Exact manifest loading reconstructs the current logical package revision to bind
its repository, package, revision, semantic state and interface. The previous
link entry point discarded that checked projection and reconstructed it again.
This repeats complete owner reads and canonical projection work during every
link, including large trusted workloads.

This change removes the second reconstruction within one link invocation. It
changes no wire format, canonical artifact bytes, public policy or persistent
cache key. It does not narrow complete source admission or claim additional
Tier 2 semantic families.

## Contract

`compiler/cache.rs` returns the loaded compilation together with the exact
`BuiltPackageRevision` used by `bind_manifest_to_current`. All six existing
bindings remain: repository identity, package identity, revision, package-revision
digest, semantic-state digest and interface digest. Other cache callers continue
to discard the returned projection after successful binding.

`compiler/link.rs` constructs that pair inside the existing
`RepositoryView::with_link_read_reuse` scope and passes the same checked projection
to `link_prepared`. The owned projection is released inside the operation; only
the completed link and its copied work observation survive into final rereading.
There is no cross-invocation projection cache or new current-permission shortcut.

Capturing the projection inside the scope matters: a canonical owner used only
by complete package projection must still be reread before the link is returned.
The scope retains its existing cancellation checkpoints, final physical object
verification, retained transport-selection checks, and cleanup on errors or
unwinding. A still-admitted pinned view remains valid after later public HEAD
publication; a new current view rejects a manifest from the old revision.

The capture mechanism records reads through the view. Compilation-manifest reads
through the independent object store retain their existing strict decoding and
artifact-admission checks; moving the caller into the scope does not convert them
into recorded view reads. Likewise, opening the view validates authoritative
revision records before this scope begins. This change does not claim an added
final reread of those earlier reads or require a stored copy of a legitimately
derived package-revision object.

`link_prepared` still performs its independent export/dependency binding,
canonical re-lowering, artifact construction and strict admission. Explicit
finite policies, checked framing and passive counter saturation retain their
existing meanings.

## Discriminating controls

A small public-authoring disk fixture includes an unused module whose canonical
owner is sealed into a pack containing exactly that object. Public publication
remains the operation that adopts the prepared meaning. The linked artifact must
omit the victim from both its runtime and reference closure.

The corruption and missing-pack tests first build the projection outside capture.
After successful `link_prepared`, the isolated fault must leave that control's
final reread successful. They then put the checked projection inside capture,
repeat the same post-link fault, and require the exact physical rejection. After
restoration, the actual link entry point must succeed on the same view, proving
that refusal did not leak capture or reuse state.

A separate test compares public linking with immutable compilation and checks
both the old pinned view and a new current view after public HEAD advancement.
Existing finite-allowance, cancellation, dependency and cache controls remain.

## Measurement contract

The ignored `link_projection_measurement` probe uses a retained public disk
fixture and its exact compilation manifest. It performs no build fallback,
fixture authoring, source export, compiler generation or runtime execution.
The external driver binds the complete source, actual Cargo-selected image,
complete fixture files and terminal process outcome for each arm. Both arms use
the same probe and fixture; complete artifact bytes must compare equal.

Timers distinguish repository opening, fresh view validation, actual linking,
retained-admission rebinding and owned cleanup. Link time includes canonical
re-lowering, encoding, strict artifact loading and final rereading. The later
`into_admitted` step is not another strict load. External inventory/hash checks
and artifact-file publication are outside those phase timers. Input hashing
warms file contents; the comparison makes no cold-cache claim.

Linux `VmPeak` and `VmHWM` describe process-lifetime virtual and resident peaks;
`VmRSS` is current at each observation. Checked CPU/I/O deltas and modeled linker
work have different scopes. The previous linker omitted its discarded first
projection from reported work, so those modeled counters alone cannot quantify
the removed work. A single measured pair is an observation, not a statistical
speedup guarantee.

## Acceptance requirements

Focused controls precede meaningful changed selection and all 29 fresh full
gates on the frozen staged tree. The candidate-built verifier owns its target
lease; original workspace stdout/stderr are retained through joined cleanup.
`release target` checks target policy and is not archive publication or deployment.

The maintained raw fixed-point control must also run anew. Its host oracle links
a program used to emit C1, so this host-link change reaches both C1 and Ahost.
The control must genuinely compute C2 and C3, A1/A2/A3, compare them with fresh
Ahost, strictly admit every artifact, exercise independent behavior and source
mutation, and finish the final original-repository integrity check. Historical
output digests are comparisons; they cannot replace those computations.

## Measured pair

The retained disk fixture has 1,000,007 live owners, three compiler units and no
semantic dependencies. Both fresh links produced exactly 267,505,246 identical
artifact bytes. Complete physical input inventories remained identical across
both arms, and the same probe ran in the original Cargo-selected executables.

| Observation | Before | After |
| --- | ---: | ---: |
| Link, including strict admission and final rereading | 379.310 s | 341.528 s |
| Process read operations during link (`syscr`) | 26622072 | 22365783 |
| Aggregate process characters read during link (`rchar`) | 170306376549 | 154617539668 |
| Process lifetime resident high-water mark at link completion (`VmHWM`) | 6679532 KiB | 6655040 KiB |
| Recorded final object rereads | 1065805 | 1065814 |

This pair observed 9.96% less link wall time and 15.99% fewer read
operations. Peak resident memory was essentially unchanged. The process reported
zero physical read bytes during both link intervals. The after invocation reused
the executable just built for focused controls; preparatory Cargo time is outside
the comparison. Twelve focused projection, read-reuse and artifact controls passed.

The complete machine record is
`.artifacts/capacity-unbounding/link-projection-reuse-20261108/measurement-comparison-1.json`
(SHA-256 `a17f2fab180a4e0e283486aa86de7194d2ced219d1d2915628088778357b5523`).
The artifact SHA-256 is
`c730de801b29b1040d46d51e1cbcd0cca113c8aaa42dce929e5f09e539eb9e9c`.
These measurements precede the required staged-tree acceptance gates.
