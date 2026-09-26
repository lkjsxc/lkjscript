# Evidence-gated roadmap

[Current status](status.md) describes available behavior and release state.
[Specifications](spec/) own contracts; [campaigns](campaigns/) retain decisions,
failed experiments and exact evidence. This page selects direction, not history.

## Selected product direction

Make an ordinary native application easy to create, edit, check, compose, build,
run and recover with one compatible executable. The [web starter](guides/native-web.md),
[strict form library](guides/native-forms.md) and [durable editor](guides/native-editor.md)
are concrete consumers, not a mandate to make every web concern a compiler feature.
The next coherent boundary is the recurring edit/build/run loop: retain accepted
program identity, separate deployment choices from meaning, preserve saved data and
make failure recovery explicit.

[Current status](status.md) owns which increments are integrated and published.
[Release procedures](release.md) and the linked campaigns own exact source,
candidate, publication and acquisition evidence. Do not duplicate moving workflow
states here or turn a historical pending run into a perpetual development task.
Published assets and their original evidence remain immutable.

Native application development and native contributor-tool adoption are distinct
from a self-hosted compiler. Raising a language-percentage metric is not a goal.
Keep the Rust kernel where it is the better owner; add ordinary native libraries
where a complete consumer demonstrates the need. A broad rewrite is appropriate
only when its complete transition is better than improving that boundary.

## Next workload and decision criteria

Prefer the shortest complete route from an ordinary native program to useful
behavior. Choose the next change by observing a real user's public authoring,
editing, testing, composition or execution workload, not by expanding a feature list.

The [native editor](guides/native-editor.md) demonstrates ordinary typed page
composition, explicit POST actions, configured Host/Origin admission, independent
shared authentication, visible conflicts and durable storage without application
HTML/CSS/JavaScript. Existing transactions were sufficient; no framework intrinsic
or browser backend was needed. Its browser-discovered referrer-policy defect shows
why raw HTTP observations and actual user-agent behavior are distinct obligations.

The [web starter](guides/native-web.md) addresses the first setup barrier:
`new --template web`, check, build and serve require no manual UI imports or source
downloads. Native UI code is vendored as editable local modules, not moved into a
privileged renderer or a hidden registry. The application can be edited through the
same identity-preserving native draft workflow and deployed without its authoring
graph. Its focused public and script-free browser results belong to the
[campaign](campaigns/202609251450.md); exact public acquisition is tracked separately
from source tests rather than inferred from a development executable.

The [form library](guides/native-forms.md) now closes a concrete input/output gap:
ordinary native code can both parse form data and construct bounded canonical
UTF-8 bodies or queries. Ordered pairs and duplicate fields remain explicit;
transport, escaping for markup and authorization are separate owners. A structural
consumer, independent byte expectations and a detached POST receiver test this
composition without creating a privileged framework serializer. Aggregate runtime
quotas still bound batches even when every individual form satisfies its own limit.
Keep exact dependency updates explicit rather than silently replacing old suppliers.

The next useful question is the recurring development loop, rather than another
large closed starter. Use a second small real application or an identity-preserving
editor change to identify genuinely reusable admission/form/persistence helpers and
measure discovery, review and restart friction. Prefer ordinary packages or small
public authoring mechanisms over a special-purpose compiler subsystem. Preserve
explicit dependency updates: vendoring is convenient creation, not a silent upgrade
policy. Do not hide grants, transaction completion or recovery behind a convenient
API. The observed manual rebuild-name/descriptor friction is addressed by
`build --deployment`: derive an unselected content-addressed
pair, preserve the operator template and local data roots, and reuse only exact
bytes. The [campaign](campaigns/202609260831.md) retains copied-binary command,
concurrent build, native edit, persistent-data and old/new HTTP witnesses.
Source acceptance and distribution are recorded separately there. The
[relocation continuation](campaigns/202609261015.md) adds complete-directory moves,
source-free execution from an unrelated working directory and refusal to initialize
missing transferred data. This is not a live-backup protocol. The next question is
review/discovery and explicit restart ergonomics, not another renaming wrapper.
Measure those costs on actual edits before choosing a development server or reload
protocol. A future switch must own readiness, failure and joined
shutdown without conflating accepted meaning, built bundles and running processes.
Multiple notes or richer actions should justify their shared mechanism;
completing this particular note app is not the language's purpose. A form is not
authorization, and a failed response does not establish rollback.

Do not migrate native consumers solely to remove a small helper. The measured UI
text-join consolidation increased identical-test-suite instruction counts despite
smaller transports. It remains unselected with its originals retained, and the
existing public-v0.1.44 UI path remains intact. Require complete affected-consumer
proof and an explicit maintenance/performance decision before replacing it.

The [paged-list workload](guides/native-list.md) separately exercises fresh library-backed
HTTP composition, strict native query-number parsing, detached use and a reviewed
edit. No additional kernel or framework primitive was needed. It leaves a concrete
next question about edit/review scale: at the same revision and intended small edit,
function-only drafting reduces the proposal from 18,732 to 3,066 bytes, but both
complete logical plans remain 155,246 bytes with 34 recreated internal owners.
These are one workload's byte/owner counts, not model-token, monetary or latency
claims. Prefer the existing targeted draft immediately; investigate finer edits or
review presentation only against a demonstrated workflow, preserving full validation,
identity continuity and review binding. Do not mistake a large optional proof file
for compulsory model input or remove correctness evidence merely to shrink it.

Continue selecting common improvements from ordinary native applications, not a
particular experimental game's completion. A browser/Wasm backend is not a
prerequisite for useful server-side development. The existing [HTTP](guides/native-http.md),
[typed HTML](guides/native-html.md) and [HTML-over-HTTP](guides/native-html-http.md)
programs remain useful separate composition witnesses with their own runtime bounds.

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
