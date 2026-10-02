# Structured parallel tasks with exact owned ingress

Entry: `d9a6ba5e7a8ab6de94a3ee5f5b54e526c42e2ab3` on main. This reporting-only
descendant contains accepted runtime source `7ea18ac9239ae99ddac892d44ad576b32b1afcdb`.
The [mailbox campaign](20261002-custody-mailbox.md#accepted-source-and-copied-executable-continuation)
retains its 26 fresh gates, 1,538 passing workspace tests and four copied-product
cases. That proof does not certify the new language work described below.

## Owner instructions, unchanged

> lkjscriptについて進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。ちなみに、今回のあなたは過去最高のリーゾニングエフォートで動作しています。

Environment clarification:

> そういえば、この環境はcoderのtomatoそのものです

Scope clarification:

> 根本的な課題があれば、今回のうちに挑んでおきたい。今回は特別なので

## Selected implementation boundary

The continuation tackles actual structured language execution, not only an
internal transfer helper or a future design. The selected first expression is a
pair of direct calls to named monomorphic graph tasks with closed empty effect
rows. It moves disjoint consuming owned inputs, permits closed ordinary data
arguments, and joins both children before returning an ordinary structural
record with `left` and `right` fields. Parent argument evaluation remains authored
left-to-right. Empty task effects do not make the operation callable from pure
code. Generic libraries and exact implementations remain usable inside each
named child. General endpoints, detached tasks, borrowed child arguments, owned
child results and capability transfer are separate boundaries.

The implementation must address three related obligations together: sealed
destination admission for every nested owned token, bounded execution without a
parent/child worker deadlock, and shared resource accounting without child quota
replenishment. Ordinary nominal metadata keeps the exact prepared-program identity;
memory custody crosses fresh invocation origins. Raw ingress remains closed.
All child work must be joined through failure and cancellation. Source, artifact,
independent reference and public authoring checks remain distinct obligations.

Work continues in the existing structured-handoff checkout and lineage. Front-end
meaning/admission, physical custody, execution/accounting and compilation/loading
have separate code owners with one integrator. This selection is not yet accepted
language support, a performance result or binary publication.

## Additional session regression

Before the parallel-language edits, the actual session driver regression passed
in 3.89 seconds after a 3m57s test compilation. The controlled clock adapter closes
the writer inside a successful graph callback, after observing its reserved slot.
The test requires immediate failed continuation, exactly one callback, an untouched
second event, no accepted output batch and joined task/stream cleanup. It uses the
normal VM and exact grants; it does not instrument the driver's private state or
replace actual WebSocket tests. Original output is retained in
`.artifacts/20261002-custody-mailbox/driver-regression-01.log`.

The predecessor reporting correction passed two fresh changed-profile gates in
the main checkout using its unchanged checker/policy owners. An earlier command
without Cargo's PATH failed before selecting gates. A second invocation used a
checker compiled in the implementation checkout: its compile-time repository root
selected that checkout, not the shell working directory. That unintended broad
run was cancelled, and its remaining owned Cargo process group was terminated.
It supplies no acceptance. Future verification uses the checker built for its
actual owning checkout.

## Implementation findings and focused feedback

The implemented graph expression reaches native/flat authoring, canonical drafts,
definition projections, exact package closure, compilation, strict loading and both
evaluators. Admission keeps empty-effect task kind distinct from purity and limits
children to exact monomorphic graph tasks with closed ordinary results. Fresh memory
identities cross a private nonduplicable capsule; physical token adoption preserves
nested allocations and immutable metadata. A bounded scoped worker shares one
invocation ledger with its parent and joins on every exit, including host unwind.

Independent review found and corrected three execution seams before acceptance:
the capsule must enforce the same ordinary-prefix/owned-suffix order as the graph;
argument scratch and result names must be charged before allocation; and a worker
permit must remain outside the scope closure so caller unwind cannot release it
before automatic join. Controlled overlap uses an isolated test worker lane while
executing the actual child VM, avoiding interference from other tests' global lanes.

The first focused attempt exposed malformed test literals and a real authoring
inventory omission: flat `expression.parallel` left/right edges were absent from
preflight dependencies. The maintained preflight owner was repaired. The second
attempt reached compilation and caught the omitted current generation tuple in
CompilationManifest admission. These failures remain in `parallel-01.log`,
`parallel-02.log` and related original logs; they are not accepted executions.

The third Cargo-selected library harness passes 14 parallel cases, seven sealed
transfer cases, one shared-ledger case and one separately run nested-depth case.
It demonstrates actual child overlap with different thread and memory identities,
aggregate exact/below-limit instruction/allocation/item quotas in both evaluators,
depth-33 refusal, original-trap precedence, cancellation cleanup, caught caller
unwind/join ordering, rehashed code rejection and unchanged historical graph reads.
The additional graph-19/20 package-source preservation case passed in the preceding
harness. Full-source and final copied-product acceptance are still separate work.

Strict Clippy first rejected an eight-argument child helper; immutable child context
now groups its execution policy and inherited depth without suppressing the lint.
The five current adversarial derived fixtures were regenerated by their test-only
owners while preserving every original predecessor. Both generation invocations
failed only their expected old-golden comparisons; the live-reading predecessor
control then passed. Embedded finite controls require the later rebuilt harness.
Original outputs and pre-replacement copies are retained under
`.artifacts/20261002-structured-parallel/derived-regeneration-01/`.
