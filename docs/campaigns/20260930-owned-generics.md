# First-order owned generics and explicit implementation witnesses

## Initial mandate (unchanged)

Implement the first-order owned-generic/explicit implementation-witness increment in lkjscript. Your ONLY editable repository is /home/coder/workspace/lkjscript-owned-generics-20260930, a clean detached worktree at 40e098d9. Confirm pwd/status. Read AGENTS.md and /tmp/lkjscript-owned-generics-design-20260930.txt (the completed independent read-only review), actual owners and selected direction. This is actual implementation, not another plan. User delegates long-term design, breaking source contracts allowed when explicit, graph-first meaning mandatory. You are implementation owner; main assistant independently reviews/tests/integrates. Do not commit/push, change other worktrees/repos, services, releases/tags/selectors, permissions, credentials, or use MCP/external services. No dangerous sandbox bypass. Use PATH=/home/coder/.cargo/bin:$PATH, CARGO_BUILD_JOBS=4, own worktree target only; avoid broad full/release suite, main will run dependency-complete acceptance after source stabilizes. No Python dependency or generated semantic side-registry.

Selected coherent contract: explicit Owned constraint (closed distinct from CaptureSafe/None) for rank-one direct affine memory parameters/results/locals. Add independently sealed OwnedI64Cell with fixed scalar storage and pure creation/read/consuming replacement/extraction/discard, not encoded bytes or renamed ByteBuffer. Generic body must be checked symbolically independent of instantiation, instantiated exact type/modes checked again. Final memory suffix, exact local args, synchronous read reborrow, any consuming alias rejection, lexical/failure cleanup, pure only signatures and no capability mix remain. Ordinary generic/CaptureSafe contracts unchanged, unsupported combinations reject. No borrowed results, owned containers, escaping captures, mutable borrows, asynchronous transfer, task memory signatures or generic implementation schemes.

Add minimal nominal first-order method contract with one owned Self and explicit implementation owners mapping every method to an exact visible monomorphic pure graph function, with exact parameter types/use modes/result/kind. Explicit witness parameters/call operands can supply an implementation or forward an exact in-scope witness; no implicit search. Static witnesses are canonical semantic operands, not ordinary runtime functions/dictionaries or grants. Multiple same-signature implementations may exist but exact selection must survive package transport/preparation. Methods can create owned Self, consume-return Self, or borrow-read ordinary result. Ordinary graph wrappers call closed externals. All methods checked even unused; tasks with empty effect rows are not pure methods. Do not reuse capability Interface or validation-witness namespace as trait authority. Keep implementation modular where feasible; avoid giant monoliths or unchecked fallthroughs. Complete canonical codecs/generations, relations/change invalidation, native authoring/query/draft, package contracts/export/import, inference, affine kernel and independently derived oracle, compiler/lowering/canonical artifact verification, bounded closure and recursion including witness edges and exact witness keys, VM and independently source-derived reference dispatch, generic result/loan cleanup, raw/persistence/capture rejection coherently. Do not let exact ByteBuffer absence fast paths bypass Owned-only or cell-only programs; preserve elided ordinary annotation acceptance while requiring owned type metadata. Preserve exact origin/type distinctions in raw tokens; Clone inert, shared metadata not ownership; cleanup cannot depend on user method calls. Reserve before storage growth, metering exhaustion distinct invalid meaning, no raised proof/execution limits. Update version selectors when meaning changes, no old proof silently valid.

End-to-end evidence: one generic producer/transformer/consumer graph across packages, instantiated with ByteBuffer append/length and I64 cell replace/read. Same inputs 0,255,128 produce independent results 3 and 128; additionally signed extremes on cell. Generic-only package must have no concrete ByteBuffer type. Include borrowed helper and recursive forwarding where supported. Public native literal fixtures, unchanged draft, exact source-free detached execution with empty grants. Negative and hostile artifact cases: duplicate consume/use after move, consume+borrow alias both orders, escaping loan/results/capture/container, unused/phantom invalid args/untaken branch, wrong contract/Self/method/mode/result/scope/missing/stale witness, task-as-pure, same signature different implementation selection, forged consistently rehashed source+code+metadata, cleanup/traps/cancel/quota both representations, raw/adapter ingress. Extend independent oracle without just calling production resolver. Use tests with independent expected outcomes and predecessor failure where feasible. Regenerate maintained standard/native guide/policy/app bundles only through supported public owners, retaining accepted meaning HEADs unless authoring new standard operations explicitly. Discover generated reference owner; no hand-editing accepted graph/binary packs. Add concise normative spec, native guide and campaign initial mandate/reconciliation/results preserving failed attempts. Do focused builds/tests and clippy, correct regressions; main handles full frozen-source acceptance and integration.

If the complete contract reveals a genuine blocking flaw, preserve exact work and report the flaw rather than weakening checks or claiming partial implementation is complete. Otherwise finish the full stated slice; no arbitrary deadline. Return exact changed scope, test results/failures and remaining unsupported boundaries when done.

## Startup reconciliation

The authorized checkout was clean and detached at
`40e098d9fde50dc25f6018808d3871ad545b7b2a`; cached `origin/main` was identical.
The other worktrees and the unrelated recent-history stash are preserved.
The root guidance already matches the supplied guidance. The selected direction
and independent review agree on the explicit owned/witness boundary. Concrete
ByteBuffer implementation and its annotation/inventory corrections are completed
predecessors; owned generic constraints and static method witnesses are actionable.
Integration, dependency-complete acceptance and publication are delegated to the
main assistant and are outside this implementation checkout's authority.

Pinned Rust is 1.98.0. Startup observed 34 GiB available memory and 563 GiB free
disk. No Cargo, rustc or lkjscript process was observed in the process namespace.
Builds use this checkout's target directory and four Cargo jobs. No external
services, source-control writes or other checkout edits are authorized.

## Resumption and independent source review — 2026-09-30

The integrator paused the original implementation to deliver two independent source-inspection reports (`/tmp/lkjscript-owned-generics-early-audit-20260930.txt`, rechecked 08:49 UTC, and `/tmp/lkjscript-owned-contract-audit-20260930.txt`, rechecked 09:00 UTC). Their suggested regressions were not executed evidence. The same detached worktree and original full scope remain in force; no product edits were supplied by the integrator.

Confirmed findings being corrected: ordinary structural raw-entry and effect/requirement callback admission; phantom owned external arguments in the reference evaluator; unsupported nominal Owned constraints; unmetered absence scans and witness inventory reads; declaration identity-domain admission; the package-interface wire-layout collision; contract Self checks independent of type inventory; nominal wrappers hiding callable method types. No finding has been rejected. The initial interim interface-11 compatibility cut was superseded by a frozen interface-11 representation plus distinct interface generation 12. Canonical Graph 14–17 owner layouts remain separately readable. These are implementation decisions, not yet acceptance results.

The integrator's independent native tests were incorporated as `tests/public_cli/native_owned_parameters.rs`, with the abstract library literal separated into `tests/fixtures/owned-parameters-abstract.lkjc`. The supplied predecessor evidence and project files remain untouched. Their historical successful ordinary duplicate/draft and rejected Owned-only probe are integrator evidence, not candidate observations.

Failed attempts retained: a JavaScript replacement-string expansion corrupted the native declaration parser; its exact failed content is `/tmp/lkjscript-owned-declarations-failed-edit.rs`, and the intended changes were reapplied to the original file with literal replacement callbacks. Initial builds exposed incomplete exhaustive matches. Library check 13 passed with one subsequently removed dead-code warning. The first executed scalar test (`/tmp/lkjscript-owned-test-3.log`) failed on a missing fixture parenthesis, which was corrected. Public native run 1 failed compilation in the new reference dispatch module. Runs 2 and 3 executed all three independent tests and failed at the setter's compact-label parsing; assertions were retained. Run 3 showed that changing only the decoder did not fix the earlier inventory parser: the native setter must use the existing `%` fragment convention throughout. None of those failures establishes semantic coverage or completion.

### Continued focused execution

Native parameter runs 4–6 exposed the missing compilation-manifest generation and
unsupported existing-parameter use-mode edit (including a missing import); run 7
passed all three independent native assertions. Scalar/witness run 4 exposed a
misnested literal module and missing compilation units for the new declaration
kinds; run 5 passed scalar extremes, ordinary aggregate raw entry and two exact
cell implementations. The existing effect/requirement callback regression passed
in both evaluators (`/tmp/lkjscript-owned-callback-1.log`).

Contract run 1 passed identity-domain and ordinary-only Self tests, but demonstrated
that incremental publication still accepted nominal Owned parameters despite full
validation rejecting them. The shared affine owner path now rejects this case;
contract run 2 passed all three tests, including nominal callable hiding.

Package run 1 failed a literal structural-record marker; run 2 reached execution
and failed because its test reused a create-new output path. Run 3 passed native
three-package authoring, unchanged drafts, transport, both scalar extremes, exact
same-Self alternate selection, and detached execution after all temporary projects
and producer transports were removed, with empty grants and PATH.

The first `owned_` library filter executed 50 tests: 44 passed, five failed and one
preexisting scale test was ignored. New cell, raw aggregate, phantom external,
witness selection, independently source-dispatched unused-map negatives and
8,192-deep consume/read tail forwarding tests passed. Failures exposed a source
container magic guard omitting Graph 17 and a deletion-owner coverage fixture
missing the new kinds; the other failures reached stale maintained artifact
encodings. These are intermediate source results, not final acceptance. No limits
were raised and no assertions removed. All logs named here remain in `/tmp`.

### Review fixes and additional focused results

The Graph-17 source guard and deletion-owner coverage failures were fixed without
weakening predecessor admission. A retained predecessor transport now verifies
interface-11 function/constant/component layouts against exact frozen bytes. Owned
focused runs 2–5 progressed to 52 passed, four stale-asset failures and one existing
ignored scale test (57 selected in run 5). The coherently rehashed source/code/metadata
attack initially failed its test packer's missing unit-15 generation, then passed
for both carriers. Cleanup, quotas, cancellation with live loans, adapter-result
rejection, cell raw/persistence rejection and template raw-entry rejection passed.

Native run 8 passed four of five tests, including all independent symbolic native
cases and the full three-package path. Inspection failed because the executable
function-definition field inventory omitted method/witness fields. The inventory
was corrected; this failure was infrastructure, not accepted semantic rejection.

An independent run of the existing affine-budget filter found six failures in nine
tests. The new memory traversal changed phase counts, and the first implementation
also charged cancellation-only checkpoints twice. That implementation is superseded:
`validation_work` now admits traversal separately from cancellation-only checkpoints.
The tests retain every predecessor boundary, add independently enumerated memory
phases, and keep production proof limits unchanged. Run 2 passed all nine tests.

Workspace/all-target Clippy run 1 failed with 22 warnings promoted to errors,
primarily needless mutable reborrows, plus formatting and an overlong argument list.
Run 2 passed after fixes. Later metadata/token refinements still require another
final run. Source inspection also corrected scalar reservation to include Mutex
storage and made artifact implementation-inventory work observable and bounded.
Neither source inspection nor these intermediate passes constitutes full acceptance.

### Closure audit and preserved pause

The integrator deliberately paused this implementation at 10:35:42 UTC to deliver
`/tmp/lkjscript-owned-generics-closure-audit-20260930.txt`. This did not change the
mandate, worktree or acceptance boundary. The report contains three source findings:
bundle-wide rather than source-declared dependency authority in artifact admission;
missing ImplementationCall edges in independent reference type closure; and recursive
nominal actual-binding overwrite in three first-order method-type walkers. None has
been dismissed. The integrator separately executed a valid recursive Node<I64>
control and observed kernel_owned_contract rejection after adding the method
contract; its copied product, project, literals and logs remain untouched.

Native runs 10 and 11 failed compilation in the new oracle/test helpers (a record
kind accessor and treating parsed CLI records as text); run 12 passed all six native
tests, including exact private-signature export rejection with unchanged HEAD.
Standard build attempt 1 then failed the independent oracle's retained interface
commitment check. Its interface reconstruction still selected generation 12 for
old owners; it now explicitly selects 10 for owner 14 and 11 for owners 15–17.
The frozen interface-11 regression now also invokes that independent reconstruction.
No accepted package HEAD was changed. `/tmp/lkjscript-owned-build-assets-1.log`
is preserved; no asset regeneration result or final acceptance is inferred from it.

Three focused closure regressions were authored before correcting the audited
product paths. The first compile attempt exposed missing test imports and one
DependencyRecord generation field; those scaffolding failures are retained in
`/tmp/lkjscript-owned-closure-red-1.log` and are not semantic red evidence.

Closure red run 2 executed the composite and recursive regressions: production
returned the independently expected 7, while source reference rejected the missing
composite application; recursive method admission rejected valid closed data.
The artifact case initially stopped at an invalid imported-module qualifier, which
was fixed without changing its semantic assertion. Red run 3 then accepted the
fully rehashed hostile artifact after A→B removal while root still retained A and B;
the regression correctly failed. Its package revisions, semantic states, compilation
bindings and container hashes were coherently rebuilt. These are executed failures,
not source-inspection claims. The additional nested-callable negative in that run
stopped at the still-invalid recursive positive fixture and was not negative evidence.

The correction gives artifact body readers the exact source revision's dependencies,
adds ImplementationCall to independent reference application discovery, and replaces
concrete nominal-environment overwrites with structural ordinary-property proofs.
Every nominal actual argument remains checked in its enclosing scope, including
phantom arguments; each nominal body is checked under only its own ordinary formal
assumptions. Recursive cycles cannot introduce free parameters, callables, resources
or owned containers. Production, source reference and the independent oracle retain
separate walkers. No limits were raised.

### Closure correction and asset regeneration

The four new closure tests passed after correction (`closure-fixed-1`), followed by
seven public native Owned tests (`native-13`). The latter includes independently
expected results 7 and 128 for witness-only composite derivation and recursive
ordinary method data. No independent inspection report is counted as execution.

The related focused run (`related-1`) had 118 passes, seven failures and one
explicit predecessor-inventory acquisition ignored. Four failures used stale
maintained assets; three used predecessor derived-unit/artifact assumptions.
The subsequent compiler-focused run (`compiler-1`) had 44 passes and 15 failures:
stale maintained assets and binary64 generation assertions accounted for the
failures. The frozen direct and named expanding-call attacks passed after neutral
re-encoding of their exact canonical source and instructions into current derived
envelopes. Their original bytes remain untouched and rejected at the rebuild cut.
The Graph 14 iteration fixture now rebuilds the exact retained source before
running its original 33 behavioral cases. The unit decoder explicitly rejects
predecessor layouts before decoding; this supersedes its intermediate attempt to
parse old function layouts before failing generation admission.

The first standard build failed with `package_closure_oracle` because independent
interface projection used generation 12 for a retained generation-17 source.
The oracle now independently selects interface 10 for graph 14, 11 for graphs
15–17, and 12 for graph 18. The frozen interface-11 regression also exercises
independent package reconstruction. The second public standard build and export
succeeded, as did the first public guide and policy builds. The first application
build failed with `builtin_standard_source_artifact` before the host's embedded
standard was refreshed; its log remains retained. No accepted HEAD was changed.

The fresh standard export has package revision
`package_revision_573523e947ac4b65eee9de6b7184361b3a2acf8be52d06db6491ae054821b932`
and transport
`package_transport_5b9f9cb3cd2dc756e0cf98dcadd1e6b48d16044bf4e01753e4240cfc1b8ba0ea`.
This is derived source-container/interface generation, not a new semantic revision
or a replacement of exact retained dependency selections.

## Integrator continuation and source freeze preparation

The implementation owner was stopped and joined before the integrator resumed
writes. Its read-only handoff is `/tmp/lkjscript-owned-generics-handoff-20260930.txt`.
No source commit, full-profile acceptance or push was claimed at that handoff.
Its final focused combined run had 166 passes, four failures and two existing
ignored cases; all four failures concerned predecessor numeric fixture handling.
Final Clippy was still unrun. These results remain failures, not retrospectively
accepted evidence.

The integrator finished the test-only frozen unit reader and wired genuine older
standard and transaction fixtures through neutral derived-envelope re-encoding.
Canonical source and original instructions remain unchanged, and production still
rejects the old derived units before decoding. The standard fixture is compared
against a fresh compile of its retained source; transaction completion retains
its exact original bundle identity assertion and independent Committed(Unit),
zero-write, zero-live-transaction and joined-cleanup assertions. The old-outer/new-
compilation forgery rejects at its exact package binding, separately from the
current-outer/old-unit rejection. `cargo test --locked --lib f64 -- --nocapture`
then passed all 29 tests, zero failed/ignored, in 10.29 seconds after compilation.
The complete original output is `/tmp/lkjscript-owned-integrator-f64-1.log`.

The added native implementation-remapping regression passed one test with zero
failures/ignored cases in 1.04 seconds. Its log is
`/tmp/lkjscript-owned-integrator-mutation-2.log`. The initial launcher invocation
failed before Cargo because a background wrapper treated `export` as an executable;
that operator failure is not a product test result. The corrected invocation uses
an explicit shell and retains its own terminal status.

The explicit-owned-implementation validator feature is now version 2, invalidating
preliminary version-1 proof reuse after the dependency and recursive-type fixes.
The global validator remains generation 18. No admission budget, cancellation
contract, accepted project HEAD, deployment grant or running service was changed.
Dependency-complete acceptance remains separate until the source is frozen below.
### Integrator's independent native observations

The copied debug product `/tmp/lkjscript-owned-generics-independent-probe-1032`
was preserved from the implementation tree's product modified at 10:29:45 UTC.
It is a dirty-source observation, not frozen-source or release acceptance.

The literal recursive-data control at
`/tmp/lkjscript-owned-independent-recursive-base-20260930.lkjc` planned successfully.
Adding a Visitor method with ordinary `Node<I64>` before borrowed Self in the adjacent
`/tmp/lkjscript-owned-independent-recursive-contract-20260930.lkjc` input failed
with `kernel_owned_contract`.
The separate project `/tmp/lkjscript-owned-independent-recursive-20260930` remained
at its initial revision. Both original logs and the copied product remain untouched.
The implementation continuation received this executed reproduction, separately
from the read-only closure audit and its two other source findings.

After the corrected debug producer completed at 10:56:50 UTC, its independent copy
`/tmp/lkjscript-owned-generics-independent-probe-1101` successfully planned the exact
same previously rejected contract input. The original project's initial HEAD remained
unchanged. The new output is
`/tmp/lkjscript-owned-independent-recursive-contract-green-20260930.log`;
the original failure output and predecessor executable were not replaced.

A second independent CLI exercise at
`/tmp/lkjscript-owned-independent-mapping-20260930/` created and checked one native
Owned generic library, a scalar implementation and a test expecting 128. Its initial
accepted revision was `rev_0f4344779911904836a3f1a139de6ac585767d4602fbdfa711b29b18746f6fa8`.
An unchanged cell draft replanned unchanged. A native edit then changed only the
selected Scalar implementation's read-method mapping to the existing alternative
read function, preserving declaration identity and all generic bodies. Planning
reported one updated owner, one selected test and zero executed/passed tests;
planning did not execute the selected test. Applying produced
`rev_844400b6abe5edb3df075135c84d044e99acab19407e5ca54d3f92cf7d7e885e`.
The following `check` correctly failed the old expected-128 test. A second native
edit changed only its expected result to 99. Final revision
`rev_646183e0e11125126b37fed38aa47ade090c26027f4cd5b7edf3f0dd9a7cf0fb`
passed one test with zero failures and equal production/reference results.
All literal requests, draft outputs, operation logs and the external Node launch
harness remain in that dedicated temporary directory. The application semantics
were authored and edited through public native requests, not by the launch harness.
The equivalent maintained Rust test is now included as
`tests/public_cli/native_owned_mutation.rs` and passed its focused execution.

This remapping check is successful evidence of dependency reselection and actual
new dispatch, not a product defect: graph admission and test execution remain
separate operations. None of these observations restamps another source receipt.

### Maintained-source finalization

The copied post-proof-correction debug product rebuilt standard, guides, policy
and lkjournal artifacts through public `build`. All four outputs matched their
maintained generated artifacts exactly and all four accepted HEAD files remained
byte-identical. The supported `package current export --kind transport` reproduced
the exact maintained standard transport. Generated reference pages were produced
and verified through `capabilities`, and `git diff --check` passed. Completed
outputs and unchanged-HEAD controls are in `.artifacts/20260930-owned-finalize/`.
Public builds materialized their derived pack objects before source freeze.

The initial export invocation omitted the `current` selector and correctly failed
with `cli_usage`; `standard-export.log` remains unchanged and the corrected result
has its own `standard-export-corrected.log`. An earlier background finalization
observation ended after the two successful initial builds; their outputs were
independently compared before continuing only the remaining stages. Neither
partial observation is claimed as the complete successful finalization. The final
`finalize.exit` is zero. No broad source acceptance is implied by these asset checks.

## First frozen-source full profile and contributor alignment

Source `caeb92299f3b5b86042ee8ad083427f673aeb362`, tree
`b1b951dad72e17d52d1e08931d24a682203e98b0`, completed a full-profile attempt
with 21 fresh passes, four failures, one skipped gate and zero reuse in
1,044.676478306 seconds. Inputs remained stable. The original receipt is
`.artifacts/lkjscript-dev/check/1790767674291317093-3006681-0/receipt.json`;
its terminal output and source identities remain in `.artifacts/20260930-owned-full1/`.
This is failed acceptance, not a mainline-delivered source.

Clippy found one avoidable clone in the new source-dependency regression;
`std::slice::from_ref` preserves its assertion without copying the loaded artifact.
The failed prerequisite left `workspace_tests` unrun. Distributed HTTP reached
successful function inspection, then rejected projection generation 9 because
its exact expectation still named generation 8. Service acceptance stopped before
starting a service because its retained SHA-256 expectation predated the already
rebuilt lkjournal artifact. Both consumers now bind the actual current generation
and the exact artifact already reproduced by public build and the full profile's
successful artifact comparison. No runtime behavior or service fixture meaning
was changed by these two expectation corrections.

The offline workflow reached its frozen expanding-callable artifact cases.
Those historical compiler units now reject at the explicit rebuild boundary,
not at the later semantic stage expected by the old verifier. The correction
keeps both original artifacts and tests their exact `source/compiler_unit_contract`
rejection. It also adds two separately retained current-envelope fixtures, generated
by the existing test-only converter without changing canonical source or original
instructions. They must reject as `semantic/kernel_callable_expansion` before any
execution. The offline runner and receipt reader now require all four exact cases;
a format rejection cannot stand in for semantic admission. The compiler test
reconstructs and byte-compares the current fixtures. Their provenance and the
create-new-only regeneration command are recorded in
[the fixture owner](../../tests/fixtures/finite-callable-current/README.md).

The first fixture-generation launcher combined an unqualified filter with `--exact`
and selected zero tests; its successful process exit is not a test pass.
The corrected launcher selected and passed the one compiler test, covering both
original and both current artifacts. Original logs and generated outputs remain
in `.artifacts/20260930-owned-corrections/`. Subsequent retained-byte assertions,
contributor changes and full acceptance still require their own completed runs.
No failed receipt, predecessor artifact, original project HEAD, deployment grant,
release selector, stash or unrelated worktree was rewritten.

## Second frozen profile: ordinary intent and historical observers

Source `37d18728c44e4f506e6ca4e5cda9bcb4702150bc`, tree
`4deafa9623522391d1b402af2ec63d244854b8fd`, completed all 26 selected gates
with 24 fresh passes, two failures, zero reuse and no unrun gates in
1,198.105419898 seconds. Inputs remained stable. The original receipt is
`.artifacts/lkjscript-dev/check/1790769770249759687-3071144-0/receipt.json`;
the terminal record remains in `.artifacts/20260930-owned-full2/`.
Clippy, the corrected distributed HTTP workflow (46 commands, two runners),
service acceptance and the other 21 gates passed. This is still failed source
acceptance: neither source commit has reached remote main.

Offline packages passed the four old/current finite-artifact cases, then exposed
a real native-authoring regression. Every native function emitted an empty
`SetImplementationParameters`, including ordinary declarations with no witnesses.
That extra request operation changed canonical intent and derived identities
relative to independently authored flat/structural forms. The failed workflow
retains all three inputs and plans under `.artifacts/offline-packages/run-3BCHKw/`.
Flat and structural plans are byte-identical; native is not. Exact comparison is
retained, not relaxed. Native lowering now emits a setter only when clauses exist
or an existing function actually has witnesses to clear. Ordinary create/edit
requests retain their prior generations; removing all witness clauses still clears
the accepted signature. A new public create/draft/edit/check/build/draft regression
covers that latter obligation.

The workspace library run reported 984 passed, 12 failed and eight existing ignored
cases. Three independent flat/native intent tests and the literal-edit generation
test expose the same extra setter. Three historical/generation-neutral observers
mistook the newly explicit empty function witness vector for changed meaning.
Their comparison now elides only that empty function field; historical fixtures,
expected historical hashes, owner identities, canonical type bytes and retirement
checks remain unchanged. Two additional tests require nonempty witness selections,
changed contracts and unrelated data fields to remain distinguishable. Current
projection/graph/validator expectations are aligned with their explicit new
versions, and the old graph-17 codec manifest is retained as a separate exact
predecessor assertion. Constraint tag 2 is tested positively as Owned, while
unknown tags, duplicate constraints and both orders of unsupported combinations
still reject. The root-library failure prevented subsequent workspace test
executables from running; an empty unrun-gate list is not a claim that those tests
executed.

At the same source, the copied optimized executable passed all eight `native_owned`
public cases from `/tmp` with an empty environment and PATH in 1.43 seconds;
173 unrelated tests were filtered, zero failed or ignored. This includes source-free
three-package execution and exact witness remapping. Both executables came from
the completed Cargo-reported release-command producer and were byte-compared.
They remain in `.artifacts/20260930-owned-final-native/`, with original log/exit.
Those successful cases did not establish ordinary intent equality and are not
restamped as evidence for the subsequent parser correction. The correction's
library/public runs and next frozen-source acceptance remain distinct.

## Focused correction and exact predecessor controls

The parser and historical-observer correction passed all 998 root-library tests,
with zero failures and eight existing ignored cases, in 72.99 seconds. The two
new observer controls preserve nonempty witness meaning. The following dev-library
run passed 199 tests, failed one and retained 19 ignored cases. Its historical
preflight helper incorrectly required a generation-21 artifact to pass current
execution admission. The complete failed workspace-library command remains in
`.artifacts/20260930-owned-corrections/library-correction.log` with exit 101;
its root-library success does not relabel the command as successful.

The independent native filter then passed all nine cases, including removal of an
existing witness signature, in 3.77 seconds, with zero failures or ignored cases.
`native-correction.log` and its exit 0 are retained separately. The subsequent
workspace optimized build completed successfully in `release-correction.log`.

A read-only source review in `/tmp/lkjscript-owned-legacy-cutover-review-20260930.txt`
identified three offline predecessor execution workflows that also needed the
explicit derived-format cut. It performed no tests. A scoped implementation
continuation changed only those three contributors and performed no builds/tests.
The main integrator reviewed their exact refusal, runtime, source, descriptor and
store bindings and added separate omission/substitution controls before execution.

The original preflight now binds exact retained official artifact/provenance/output
and exit bytes rather than claiming current executable permission for old evidence.
Original artifacts still require exact format refusal before execution or store
mutation. Separate current controls retain canonical source and instruction forms,
then require strict artifact admission and exact original-transport binding before
running the existing scalar/transaction/participation workloads. Public rebuild and
import paths remain mandatory. The scalar workflow records seven target calls;
transactions record twenty across original-refusal/current/rebuilt/imported groups;
participation records its additional original refusal plus all five prior cases.
All legacy outcome, store, suppression and cleanup expectations remain in place.
Normative verification and fixture continuation notes distinguish these claims.

The first current-control generation failed at `artifact_runtime_owner_count`.
The next attempt failed compilation because two test-helper imports were omitted;
the third reached a missing original runtime owner. All failed logs remain under
`.artifacts/20260930-owned-corrections/predecessor-controls-generate*.log`.
Generation four read the exact original source transports independently and selected
required metadata from their unchanged canonical records. The scalar predecessor
needed 394 runtime-owner bindings rather than the historical 383. No canonical
owner or instruction operand/order was edited. Reference maps include the necessary
original external declarations; unreachable old derived map pages are retired.
The converter remains test-only, and the existing source-less adversarial converter
path retains its old behavior.

Generation four passed the one table-driven test covering all three original/current
pairs in 0.28 seconds. Current control identities are
`artifact_bundle_1d8e03a37d339bc897500686e528ec47e49f73784cdfaa7e8201b1c7126e152e`
(requirements),
`artifact_bundle_5942bdd66c4c594547fdba1b232ce29e2ee2fdf51268e6a97087545a5d21b10e`
(transactions), and
`artifact_bundle_02c6be1f48c3ff43b2527e6440f13fd7c1bf3d1b03617fdb1723d04b2de10041`
(participation). The retained fixture owner documents regeneration and source
binding. Serialized instruction tags change with format; these are preserved
instruction forms, not a claim of identical serialized instruction bytes.
Original historical artifacts/provenance remain byte-for-byte unchanged. These
fixture observations do not yet establish a completed offline or full-profile run.

The first focused predecessor-reader run passed six tests with zero failures and
retained two existing evidence-dependent ignored tests. It includes all three
current-control omission/substitution/source checks and the historical preflight
faults. Clippy passed all workspace targets/features in 9.71 seconds, and the
optimized workspace rebuild completed. The subsequent focused offline execution
failed with `contributor_artifact_source`; its original receipt is
`.artifacts/offline-packages/run-cnITHb/receipt.json`, with wrapper log/exit retained
as `legacy-focused.log` and exit 1. An additional receipt-inspection tool call was
blocked; no permission or protection was changed to bypass it.

Source inspection identified an overconstraint introduced in the contributor
correction: it applied the original transport/package-revision equality not only
to the mechanical current control but also to a public rebuild. Public rebuilding
can project the new interface generation from unchanged accepted meaning and thus
has a different derived package revision. The exact original transport binding
remains mandatory for the mechanical current control. Public rebuilt executions
now additionally require the original package and semantic revision, while retaining
strict artifact admission, exact artifact identity, original source-pack/HEAD checks
and every independent value/store outcome. Imported wrapper roots intentionally
have their own source identities. New reader tests require wrong rebuilt package,
semantic revision or artifact identity to reject. The strict product source probe
is unchanged; this correction does not relax its comparison or old-format admission.
These newest corrections require their subsequent test and execution results.

## Third frozen profile and ordinary-program preparation correction

Source `9fb5e7308896bdc80d457e0e186c011223d10ec6`, tree
`bbf6253cd276dcc5edb961260266e2196150db4d`, completed all 26 selected gates with
25 fresh passes, one failed workspace gate, zero reuse and no unrun gates in
1,288.698800748 seconds. Inputs remained stable. The original receipt is
`.artifacts/lkjscript-dev/check/1790774145239279833-3209689-0/receipt.json`.
The previously failing offline package workflows passed, including the separate
original-format refusals and current/rebuilt/imported behavior. The root library
passed 999 tests with eight existing ignored cases. The public CLI executable
passed 178 tests, failed three and retained one ignored case; later workspace
test executables did not run after that failure. This source was not accepted or
pushed, and the failed receipt remains unchanged.

The discovery test's exact expression inventory omitted `implementation-call`
and `method-call`. Its two inventory comparisons now include both actual forms.
The other two failures occurred in existing resident/foreground quota fixtures:
their unchanged 1,000,000-byte allocation policy no longer admitted the small
successful control. The measured resident program's derived type-metadata charge
was 1,132,226 bytes before any substantial request work.

Source inspection found that witness specialization cloned the entire function,
test and port instruction inventory even when the program had no witness parameter
or instruction. A new physical-sharing/allocation regression ran against that
implementation and failed as intended: the otherwise unnecessary pass reserved
386,512 bytes for the maintained standard program. Its original output and exit
101 remain in `.artifacts/20260930-owned-ordinary-pass/red.log` and `red.exit`.
That is an exact observation of this pass, not a total-RSS or allocator measurement.

A bounded, allocation-free scan now checks every function signature and every
instruction, including unreachable instructions and both test/port expression
roots, before choosing whether specialization is needed. No-witness programs
retain their original function/port and instruction carriers. A generic-only
witness template, an already selected witness, an empty-operand implementation
call, or a concrete method call still requires the full specialization path.
Whole-artifact type, affine, implementation and source-dependency admission remain
mandatory before this derived optimization. Cancellation and proof-work exhaustion
stop the scan before it mutates code. No production or fixture quota was raised;
no type/ownership validator generation changed for this derived-only optimization.

Detector regressions cover both instruction forms in each of five root positions,
both signature states, cancellation, the exact work boundary, zero allocation and
shared carrier identity. The first post-change test invocation failed compilation
because the test used `Name::from_str` and a two-field ComponentIndex constructor;
`green-library.log` retains that failure. The corrected tests use the existing
Name constructor and index representation. A preceding background launcher rejected
`export` before starting Cargo; its corrected invocation uses an explicit shell.
Neither launcher or compile failure is semantic test evidence. The corrected root
library and selected public cases have separate output/status files and are not
claimed successful until their complete terminal results are observed.

The corrected all-feature root-library run passed 1,002 tests with zero failures
and eight existing ignored cases in 70.74 seconds (`green-library2.log`, exit 0).
Its three new absence-pass regressions all executed. The selected public run
passed 12 of 13 cases in 13.60 seconds, including all eight Owned CLI cases and
all four resident-policy cases. The unchanged 1,000,000-byte allocation policy
again admitted both small controls while refusing the deliberately excessive
workload, with joined cleanup and subsequent healthy requests. The same resident
program now reported 709,906 type-metadata bytes instead of 1,132,226; this is
422,320 fewer charged preparation bytes for that exact workload, not a general
allocator/RSS or timing claim. `green-public2.log` and its exit 101 remain failed
because the discovery test then reached another stale operation-list expectation.

The complete literal discovery inventory now includes the four owned operations
and their 15 required fields, both new owner kinds and all three witness relations.
Inspection also found real omissions in the compact field descriptions: `owned`
and `owned-i64-cell` were accepted by the parser but absent from their advertised
constraint/type alternatives. Those descriptions and their independent expectations
are corrected. The discovery test then passed in 0.61 seconds, exit 0, in
`green-discovery3.log`. All eight reference pages were regenerated and verified
through public `capabilities` (`references.log`), retaining the normal renderer.
Only the field descriptions and the common capabilities digest changed there.

These focused observations select the corrected source for a new frozen full
profile; they do not combine earlier failed receipts into acceptance. Use two
checker workers and four Cargo build jobs on the same isolated checkout, respecting
the maintained producer dependency graph. Main, all other worktrees, immutable
releases, running services and accepted application/standard HEADs are unchanged.

## Observed completed acceptance — 2026-10-01

The final fourth full-profile receipt was inspected directly on resumption. It binds
source `9afa799794ac26fa90bd4b2413e6e4d26886ccb6`, tree
`3f190685967112d9df230fff5872ebf9e4a6e228`, with all 26 gates passed freshly,
zero reuse, stable inputs and no unrun gates. Elapsed time was
1,054.047499672 seconds with two checker workers. The original receipt is
`.artifacts/lkjscript-dev/check/1790777372130651479-3312948-0/receipt.json` in the
existing owned-generics checkout; the terminal record is in
`.artifacts/20260930-owned-full4/`. This observation completes the previously
pending boundary; it does not rewrite the first three failed full receipts.

The workspace gate passed 1,435 top-level tests, zero failures and 29 existing
ignored cases. Two nested one-test subprocess controls are not counted again in
that total. Root-library coverage was 1,002 passes and eight ignored; public CLI
coverage was 181 passes and one ignored. All other selected workspace executables
completed. Clippy, ordinary resident quota controls, source-free package workflows,
generated discovery and maintained artifact comparisons also passed in the full
profile. GitHub's main ref was independently fetched and matched this exact source;
both main and the isolated checkout had this HEAD at the new continuation's entry.

No binary release or running service was changed by that integration. GitHub still
reported immutable v0.1.55 as latest on 2026-10-01. Subsequent storage-admission work
has its own [campaign](20261001-owned-storage-admission.md), source and validation;
this predecessor receipt is not restamped for those changes.
