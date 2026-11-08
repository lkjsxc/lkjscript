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
The checked implementation preserves this pair's product, test and probe inputs;
only the decision prose changed before the frozen acceptance run.

## Acceptance evidence and delivery

Implementation `47e989412e40ad2fd6ab75240f5c6a31d922288c` commits the exact
tested tree `0a4ef322e43500ae071de8ef950df1172e35915d` with sole parent
`7668bb9d0d314fed7b3adcaf9767cb8e74bb2528`. Main was fast-forwarded and
pushed normally; a separate advertised-remote query matched that implementation
commit. Integration preserved every source file and mode input of the accepted
tree. This later two-file acceptance prose has its own documentation check.

The canonical machine acceptance is
`.artifacts/capacity-unbounding/link-projection-reuse-20261108/candidate-acceptance-1.json`,
SHA-256 `b95998c66b361bbe102514146fb5ced96689dfbbaa1ecc7e2b0be48caecf1ac5`.
It binds all component receipts, actual process terminals, retained failed
attempts and the complete frozen source snapshot, whose SHA-256 is
`9e23e0db23e49be32c26d6a3216514309caf7ee7083b53f68f76c6c36246fd05`.
Receipt paths below are relative to that experiment directory.

| Evidence | Accepted result | Record |
| --- | --- | --- |
| Focused controls | 3 projection regressions, 7 read-reuse controls and 2 artifact-link controls passed | `focused-controls-2/receipt.json` |
| Bootstrap / typed bridge | Original selected verifier and complete source bindings passed | `candidate-bootstrap-1/receipt.json`, `candidate-bridge-1/receipt.json` |
| Changed | 29 fresh passed; 0 reused, unrun or failed | `candidate-changed-1/changed-receipt.json` |
| Full, explicitly fresh | 29 fresh passed; 0 reused, unrun or failed | `candidate-full-1/full-receipt.json` |
| Maintained raw fixed point | Exact ignored control: 1 passed, 0 failed, 0 ignored | `raw-controls-1/receipt.json`, `raw-audit-2/receipt.json` |
| Release target | Static-musl target policy passed; no archive publication | `candidate-release-target-1/receipt.json` |
| Main delivery | Accepted tree and source modes preserved; actual push and advertised remote matched | `implementation-delivery-1.json` |

Both changed and full root-library runs finished with 3,085 passed, 0 failed and
21 ignored. The full run's eight terminal harness summaries total 3,783 passed,
0 failed, 38 ignored and one filtered test; the separately selected release
lifecycle control also passed. Two nested helper successes in root stdout are
not counted twice. All 16 original workspace streams were retained through
joined checker cleanup and independently rehashed. The full checker took
4,958.182 s; its root library took 4,945.91 s. Fresh gate execution reused valid
Cargo outputs where reported.

The raw run genuinely computed its new generations from the complete source:

| Equation | Bytes | SHA-256 |
| --- | ---: | --- |
| C2 = C3 | 22,005,001 each | `3c1404bbfc6e0d15b7546d10fc42f9f9a22e4a9dc58b80acd412b2bee41d85fa` |
| Ahost = A1 = A2 = A3 | 22,908,368 each | `fc6a76ae349a049e69997c919620d3f425d85cfe46a9290ec69bc4160db9b0d3` |

Every artifact underwent strict admission. Independent behavior distinguished
42 from 43, and a changed compiler source produced diagnostic 77 where the
original produced 12. Native C1 and C2 phase controls checked survivor output,
scratch reclamation, cancellation/retry and callee-saved registers. The final
original-source observation and repository-integrity sentinel followed the
inner equations; the earlier inner record alone was not accepted. C1's direct
byte extent remains unspecified, and no separate C3 phase process is claimed.

The first post-run auditor incorrectly split the valid free-form telemetry label
`case=bulk binary execution` into fields. Its failed receipt remains in
`raw-audit-1/receipt.json`. The narrow correction follows the producer's leading
case-label delimiter and preserves strict remaining field checks. It passed on
the same retained successful raw run; no compiler computation was substituted.
The aggregate also retains the earlier failed preparation/control attempts.

The source, measurement, changed/full streams, actual raw records and release
proofs received independent review. The compact checker's typed receipt digest
is reported with its original attribution, not claimed as independently
recomputed; retained receipt SHA-256, publication identity and terminal bindings
were checked. Full P_self, general recursive lifecycle unbounding and heap-backed
native non-tail calls remain open.
