# Evidence-gated roadmap

[Current status](status.md) describes available behavior and release state.
[Specifications](spec/) own contracts; [campaigns](campaigns/) retain decisions,
failed experiments and exact evidence. This page selects direction, not history.

## Selected delivery

Finish [v0.1.45](campaigns/202609250650.md): native guide/policy tooling, reusable
text/byte operations, explicit optional resident quotas and contributor process
reliability. The combined implementation is mainline and fully verified; the
versioned producer must still accept its finalized distribution, then pass immutable
publication and anonymous installed use. Do not rebuild or mutate an accepted
candidate merely to update documentation. Public v0.1.44 remains unchanged.

This milestone should make the implemented language easier to acquire and use.
Native application development and native contributor-tool adoption are distinct
from a self-hosted compiler. Raising a language-percentage metric is not a goal.

## Next workload and decision criteria

Prefer the shortest complete route from an ordinary native program to useful
behavior. Choose the next change by observing a real user's public authoring,
editing, testing, composition or execution workload, not by expanding a feature list.

The next investigation should exercise a library-backed web application through
native proposals, exact packages and a standalone runtime. Existing
[HTTP](guides/native-http.md), [typed HTML](guides/native-html.md) and
[HTML-over-HTTP](guides/native-html-http.md) programs provide a starting point,
not a requirement to finish a particular experimental application. Measure the
first real impediment: repeated authoring/preparation, error diagnosis, library
composition or runtime work. Improve the common owner, retaining the public input
and an independent expected result. A browser/Wasm backend is not a prerequisite
for server-side usefulness.

Keep the public path small: discover, author, review, check, build and run.
Retire a replaced implementation when a native library/tool actually assumes its
responsibility with equivalent correctness, failure recovery and maintainability.
A host wrapper or generated string emitter does not remove an external dependency.
The supported Rust kernel/platform boundary may remain where it is the better owner.

## Conditions for larger changes

| Direction | Evidence required before selecting it |
| --- | --- |
| Native tooling and libraries | A concrete maintained consumer or blocked public witness, a real removed duplicate, public reproducibility and honest measured costs. Keep unaffected exact suppliers. |
| Faster preparation and execution | Matched behavior/workloads, source/reference correctness, clean/incremental equality and separate preparation, I/O, allocation and execution measurements. Retain regressions. |
| Additional authoring operations | A demonstrated edit workflow, typed intent, identity continuity, review binding, complete discovery, independent negative cases and a complete consumer transition. |
| Richer abstraction/resource ownership | A public composition need fixing the lifetime/failure protocol, type/effect/capture safety, producer/consumer admission and retirement of a superseded path. |
| Worker recipes and network capabilities | A standalone consumer fixing topology, exact endpoint/grant authority, cancellation, owned resources and an implementation-disjoint live oracle. |
| Browser/Wasm or another binary target | A useful program and precise host/ABI boundary, compatible transport, effect/grant policy, hosted execution oracle, distribution identity and measured costs. Do not create a speculative target matrix. |
| External package distribution | Named consumer, exact publication/resolution authority, immutable content, recovery/revocation and explicit mutable-name policy. No ambient network resolver. |
| Million-owner compilation or long history | Select compilation, graph history and operational-data scale separately. Require independent correctness/reachability, interruption recovery and exact resource observations before deletion or compaction. |
| Broader CI, signing or distribution integrations | A distinct operating need, source/trust ownership, retention, revocation/recovery and maintenance responsibility. Existing release automation is not a blanket mandate. |

None of these rows is an automatic implementation queue. A large architectural
revision is appropriate when evidence favors it; continuity with an old campaign
is not a reason to retain a bad design. Conversely, a proposed improvement must not
weaken current authority, admission, atomic publication or verification to appear
simpler. Keep semantics, operational policy, derived caches and deployment data as
different responsibilities.

## Reversal and completion

For each selected increment, retain a literal public workload, an exact intended
contract, independent failure/success expectations and a concrete delivery point.
Measure competing implementations where performance is the claim. Finish affected
consumers, generated assets, documentation and migration or explicit rejection.
A partial prototype, local commit or open PR is not mainline completion.

Preserve accepted history and old runtime/artifact pairs when compatibility requires
it. Use digests only where an identity, integrity or compatibility contract needs
them; do not add redundant inventories or receipt systems. Failed experiments
remain failed even when a later design succeeds. The [release owner](release.md)
separates source acceptance, exact distributable bytes and public acquisition.

Do not make a particular experimental application the language's purpose.
Ordinary native users and their reusable mechanisms are the long-term product.
Inbound TLS, encrypted local storage and hostile multi-tenant sandboxing remain
separate unselected systems; current limits do not imply those guarantees.
