# Owned byte buffer implementation

## Initial mandate (unchanged)

Implement the concrete ByteBuffer semantic slice recommended in your completed review. Your ONLY editable repository is the already-created detached worktree /home/coder/workspace/lkjscript-owned-buffer-20260930, currently at 88609553. Confirm pwd and git status there before edits. The original /home/coder/workspace/lkjscript and all other projects must remain untouched. Main assistant preserved and pushed the three inherited release-report documents; do not touch releases/tags/configuration/permissions, and do not commit or push. No external services or MCP are needed. This is actual implementation, not another plan. Read your review and current owners then finish a coherent end-to-end feature with tests, native discovery and spec/guide. Keep safe Rust. Use a concrete ByteBuffer distinct from CapabilityResource and no requirement/deployment grant for pure memory. Start with pure empty, push, get, length, freeze and discard closed operations with exact modes; allow direct pure helpers with final borrow/consume memory suffix and direct owned results, ordinary generics over resource-free data/callbacks, synchronous package calls. Buffers may be locals in task bodies but reject task helper borrowed signatures and mixed capability/buffer function signatures if unsupported; describe scope honestly. Borrow parameters cannot mutate/consume/return/capture/escape. Reject buffers in unrestricted generic substitutions (even unused/phantom), containers, constants, indirect descriptors and raw/adapter/persistence boundaries. Preserve all existing resource contracts. Extend canonical/type/witness generations, independent kernel and reference admission, exact compiled loader expectations, VM/reference checked value rules, parameter modes, result transfers and cleanup coherently; do not just whitelist types. Runtime ownership duplication must be impossible through the Clone raw value surface: sealed tokens or checked one-owner storage with stale/foreign/borrow rejection. No fake effect authority. Ensure lexical discarded-owner cleanup and borrowed-tail lifetime; conservatively suppress borrowed-memory tail replacement if necessary. Reserve before allocations, cleanup after cancellation/quota failure; creation/read/update/freeze must not alias mutable memory with ordinary immutable Bytes. Add an explicit Vec-adopting BytePayload freeze constructor; current From<Vec> converts to Arc slice and is not evidence of allocation transfer. Keep existing immutable Bytes semantics and performance paths. Add focused success/negative/forged-artifact/raw tests and a public CLI test that creates and imports a generic producer-transformer-consumer from native requests. Match exact binary expectations and test source-reference independently. Use the maintained standard authoring/generation path, not manual storage edits. Do focused tests/builds and fix them; no broad full/release suite yet (main assistant will run stable acceptance). Record commands and failures in your final report. If the complete slice reveals a blocking design flaw, stop with exact preserved state and a concrete alternative rather than weakening invariants or claiming an incomplete feature works. Return a clear change/test/remaining-boundary report when finished. Keep implementation focused; no unrelated cleanup or opportunistic carrier changes. Use CARGO_BUILD_JOBS=4 and the worktree own target directory; do not share original main build outputs.

## Reconciliation

The detached worktree was clean at `88609553ae97d4c3ecf71736cd8be31ef5f769a6`.
Existing AGENTS.md matches the supplied replacement. No commits, integration,
publication, release operations, or external services are selected. Original
checkout, inherited reporting documents, stash, and other worktrees are preserved.
Use this worktree's target directory and four Cargo jobs. Full acceptance is
explicitly deferred to the responsible main assistant after inputs stabilize.

## Implementation and review continuation

The review checkpoint preserved uncommitted work. On resumption, cwd and detached
HEAD were reconfirmed; the prior ByteBuffer test command had completed and no
unjoined predecessor build remained. All 13 findings in
`/tmp/lkjscript-owned-adversarial-notes-20260930.md` were addressed in maintained
implementation and tests.

The concrete memory owner is separate from capability-resource provenance.
`kernel/memory.rs` admits every executable root and final pure memory suffix;
`kernel/memory_reference.rs` is an implementation-disjoint test oracle. Branch
joins intersect liveness, metadata failures preserve exact metering errors, and
complete canonical inventories can certify absence without rescanning ordinary
resource graphs. No proof or execution budget was raised.

Sealed invocation-scoped runtime tokens own one Vec. Raw Clone creates an inert
identity marker, while raw identity equality ignores mode to preserve Clone/Eq.
Debug is opaque. VM and source-reference admission independently classify values,
reject memory at raw/capture/persistence boundaries and check direct result
transfer. Freeze uses an explicit Vec-adopting BytePayload constructor; existing
From<Vec> and ordinary immutable Bytes paths remain intact.

Canonical BUF type generation 1, authored intent 20, affine witness feature 10,
compiler unit 14 and bytecode 10 bind the new semantics. Artifact 21 independently
checks exact memory declaration/external/parameter/result metadata and canonical
compiled cleanup. A caller remains live only when it owns storage with live loans;
borrowed reader reborrows can tail-transfer. Unit/StoreLocal cleanup continuations
retain bounded consuming tail loops. General owned generics, mutable borrowing,
task memory signatures, mixed capability/memory signatures and indirect memory
descriptors remain outside this increment, as specified in
[the normative scope](../spec/owned-byte-buffers.md).

The literal [standard request](../../packages/standard/requests/20260930-owned-buffer.lkjc)
was planned and applied using a copied executable under `/tmp`. It adds six pure
externals and four graph tests. Accepted standard revision is
`rev_85b2be44a8deca911fc6bdf4efdd4fb7b510f53e4a39723fbee263dff4b3a9b2`.
Standard artifact and transport were generated by public build/export. The
embedding owner's exact identities came from those outputs. Native guide and
retained policy bundles were rebuilt through public build from disposable copied
accepted projects, preserving their original HEADs and exact suppliers. Generated
reference pages came from capabilities generation and verification, without manual
graph/asset edits. No release, tag, configuration, permission, commit, push or
original-checkout change was made.

## Focused verification and preserved failures

Cargo commands use `CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=$PWD/target`, pinned Rust
1.98.0 and `--locked`. Source is the dirty detached worktree on `88609553`; these
results are not a committed-source or published-binary claim.

| Command / owner | Fresh result |
| --- | --- |
| `cargo test --lib byte_buffer -- --nocapture` | 19 passed; exact octets, all roots, independent oracle/source-reference, raw tokens/adapters, Clone/Eq, opaque Debug, Vec transfer, both owner choices, 16,384-step builder/reader, task-local execution with task kind preserved, traps, quotas, cancellation, lexical drop, forged parameter/result/move/cleanup/generation |
| `cargo test --lib affine_ -- --nocapture` | 22 passed, including unchanged exact proof-budget boundaries and old independent resource oracle |
| `cargo test --lib resource -- --nocapture --skip normalized_http_dispatch_uses_exact_body_resources_and_resident_admission` | 33 passed; exact grants, resource modes, generics, recursion, failure and joined cleanup |
| `cargo test --lib bytes -- --nocapture` | 55 passed, including existing allocation, alias, map projection, conversion, range, cancellation and immutable reuse paths |
| `cargo test --lib platform::compiler::tests::f64_tests -- --nocapture` | 7 passed; current and authentic predecessor generations and rehashed malformed payloads |
| `cargo clippy --lib --tests -- -D warnings` | Passed |
| Copied executable `--project packages/standard check` | 89 passed; VM/reference differential equal |
| `cargo test --test public_cli native_byte_buffer -- --nocapture` with copied `/tmp/lkjscript-owned-buffer-final` | 2 passed; native primitive/intrinsic discovery, I64 and Bool generic package callbacks, exact transport imports, unchanged draft, detached `00 ff 80` with empty grants, invalid modes/phantoms preserve HEAD |
| `cargo test --test public_cli maintained_native_guides_regenerate_the_exact_embedded_artifact -- --nocapture` | Passed; 79 graph tests, exact derived bundle and all eight generated pages, unchanged copied HEAD |
| `cargo test --test public_cli maintained_native_policy_rebuilds_exactly_from_copied_accepted_meaning -- --nocapture` | Passed; 62 graph tests and exact derived bundle, unchanged copied HEAD |
| Copied executable `capabilities --verify-generated docs/generated` | All eight pages current after rebuilding native renderer |

Original failure evidence is retained, including `/tmp/lkj-buffer-tests4.log`,
`tests5.log`, `tests6.log` and subsequent numbered test logs. Early failures exposed
visibility/manifest-generation fixtures, missing exact memory external owner
metadata, borrow-check errors and a reference tail step escaping its retained
owner. Those were corrected without weakening intrinsic/artifact admission.
Later raw-test compile failures came from mixing distinct VM/reference observation
types. Exact forged-artifact expectations were corrected to distinguish owner-set,
canonical-result, compilation-binding and compiled-control failures; all seven
fault classes must execute. The original runs are not relabeled as passes.
The final task-local fixture initially named the wrong module and was placed
before its declaration; `/tmp/lkj-buffer-tests26.log` preserves that selector
failure. Correct declaration order and the explicit task/pure context distinction
passed all 19 tests in `/tmp/lkj-buffer-tests27.log`.

`/tmp/lkj-buffer-affine1.log` failed two reproduction checks against the stale
embedded standard; the regenerated standard passed the subsequent run. Initial
capability generation and public package execution failed against old embedding
identity constants; exact export bindings fixed them. `/tmp/lkj-buffer-bytes1.log`
exposed a test helper adding types after preparation without extending the new
buffer-free fact. Its checked-child metadata was updated, retaining all allocation
assertions. The first F64 compiler run rehashed a current unit with the old envelope
domain; its maintained forgery now uses the current domain and still requires
malformed NaN decoding to reject. Initial Clippy rejected two collapsible conditionals,
which were corrected without changing admission.

`/tmp/lkj-buffer-resource1.log` passed 33 cases and failed the existing HTTP case
when the sandbox denied its local listener bind (`EPERM`). No permissions were
changed, and that test was not weakened. Its live socket segment remains unverified
here. `/tmp/lkj-buffer-capabilities0.log` also retains an invalid discovery-section
probe; the actual `type` section discovers `byte-buffer`.

Full/release acceptance and mainline integration remain assigned to the main
assistant. Detailed logs and copied executables remain under `/tmp`; this campaign
does not convert a failed, blocked or unrun check into acceptance.

## Main-assistant continuation: acceptance defects and correction

The continuation recovered this same detached lineage at `88609553`, preserving
all original-checkout work, stashes, other worktrees and release selection. An
independent read-only review was completed, followed by executed adversarial tests;
the review alone was not treated as proof. Its report and original events remain at
`/tmp/lkjscript-buffer-continuation-review.txt` and `.jsonl`.

The first full profile selected all 26 gates. It completed with six fresh passes,
two failures and 18 skipped gates in 607.301890723 seconds, with no result reuse.
Workspace library tests reported 963 passed, five failed and eight ignored;
the release command lifecycle failed separately. The original receipt is
`.artifacts/lkjscript-dev/check/1790741776698818823-2233165-0/receipt.json` and the
terminal record is `/tmp/lkjscript-buffer-continuation-full1.log`. Input stability
was false: a maintained application clean build materialized its newly derived
compiler-generation pack. This run is diagnostic evidence, not source acceptance.

Four failures were exact standard/witness expectations: 1,626 owners, 89 standard
tests, 90 tests in the command fixture, and affine feature 10 with owned-memory
feature 1. An internal depth-admission fixture inserted Option types directly after
preparation without extending the buffer-absence proof; it now derives each new
fact only from its already-proved child. No production quota or rejection was
relaxed. The maintained lkjournal artifact was rebuilt through public `build`,
retaining its exact older standard pin and semantic HEAD. Two native builds produced
equal artifacts; the second complete observation is recorded in
`/tmp/lkjscript-buffer-lkjournal-build.log`. The first tool observation timed out
after producing bytes and is not independently claimed as a completed operation.

Five new regression cases then failed for four specific defects, while all 19
previous ByteBuffer cases passed. `/tmp/lkjscript-buffer-regressions-red2.log`
records 19 passed, five failed, zero ignored in 6.28 seconds. The preceding red1
attempt had a test-only observation-sink type error and was corrected before any
semantic result was claimed.

* Strict loading admitted consistently rehashed canonical **and** compiled double
  consumption in untaken function and constant branches. Both neutral controls
  loaded; each forged source type-checked, matched canonical lowering and failed
  the ordinary affine checker. The loader now also performs bounded affine
  admission over the whole retained owner inventory, including signature shapes.
  Semantic failure and exhausted proof work have distinct diagnostics. No checksum,
  code-correspondence, source-identity or resource check was removed.
* Reference non-tail sequences retained a discarded buffer while their next
  expression executed. A storage-observer callback in a binding initializer saw
  one live owner in reference execution and zero in the VM. Reference evaluation
  now releases each preceding result before starting its successor.
* The independent test oracle forgot an owner's type identity after consumption,
  permitting a subsequent direct discarded read to look like ordinary data. It
  now keeps a separate set of known memory identities until lexical exit, alongside
  its independent owned/borrowed sets. Parameter and lexical owners are covered.
* An accepted pure buffer producer with an explicit requirement parameter failed
  artifact linking because runtime metadata required an empty parameter vector.
  Metadata now preserves the exact declared vector instead. The regression also
  removes that vector from rehashed runtime metadata and requires rejection.
  The copied public executable reproduced prepared, accepted, then failed-build
  behavior in `/tmp/lkjscript-buffer-requirement-probe-20260930`, with literal input
  `/tmp/lkjscript-buffer-requirement-probe-20260930.lkjc`. Requirements do not grant
  a pure producer effects or capability operations.

The loader regressions live at the existing compiler test owner, not in an external
untrusted executable. Original rejected and accepted test controls, source-side
reasoning and exact envelope reconstruction remain reproducible in source. A new
successful acceptance record, rather than relabeling this failed first run, owns
subsequent integration.

After those corrections, the same focused ByteBuffer suite passed all 24 cases,
zero failed and zero ignored, in 6.20 seconds. The complete result is retained in
`/tmp/lkjscript-buffer-regressions-green1.log`; the other two workspace libraries
matched no cases for that filter. A subsequent ordinary workspace release build
completed in 4m 04s. The copied executable `/tmp/lkjscript-owned-buffer-continuation`
generated and verified all eight public reference pages, then successfully built
the same already-accepted pure requirement-parameter producer that previously
failed. `/tmp/lkjscript-buffer-source-finalize.log` records these completed commands;
its exit status is zero. The new artifact is
`/tmp/lkjscript-buffer-requirement-probe-corrected-20260930.lkja`. This corrects the
accept-then-unbuildable path without replacing the accepted source or adding a grant.

The implementation, derived assets and regression inputs are frozen together for
a new dependency-complete full-profile run. Local commit and source acceptance
remain separate from remote-main delivery and from distribution publication.

## Whole-artifact follow-up and maintained consumers

Implementation source `f9c2630b653ee991beef1f0209b4d018e0c9e002`, tree
`afbe204ce5fa4216439faf2c7ee6621d9a39d618`, received the next complete full-profile
attempt. All 26 gates were selected: 19 fresh passes, six failures, one skipped,
zero reuse, stable inputs, in 554.711650633 seconds. Its original receipt is
`.artifacts/lkjscript-dev/check/1790744163824494141-2310936-0/receipt.json`
(`verification_e0af3b64713adcc1989a8c406693854687805d2b51f9836df7834c33e393db6f`).
Library tests reported 963 passed, ten failed, eight ignored. This committed source
was not accepted or pushed; the failures remain historical evidence.

The new affine loader exposed one classification regression: ordinary lexical
annotation objects may be omitted from an execution artifact when canonical
expression inference reconstructs their types. Requiring every such object merely
to decide whether a binding is a ByteBuffer rejected valid maintained applications.
A fresh minimal public project with only an explicitly annotated Bool local, an if
and an I64 result reproduced acceptance followed by failed linking. It uses no
buffer or capability at all. Literal request and accepted project remain at
`/tmp/lkjscript-buffer-annotation-probe-20260930.lkjc` and the matching directory;
accepted revision is `rev_0a2f38beab75f96154f10fbfcc85f4eaa1c61576f20c9a6d0017b04f86248a73`.

Only lexical ownership selection now compares the annotation with the exact
canonical ByteBuffer identity; a matching buffer still requires its actual type
object. Complete expression type validation and initializer ownership validation
remain separate and mandatory. General signature, containment, resource and type
read errors remain errors. New regression coverage proves that Bool's ordinary
annotation object is absent from the valid loaded envelope both without and with
real buffer storage; removing a real buffer type still rejects. No extra type
inventory, hidden source dependency, new format or weakened affine gate is added.

The remaining independent gate failures were maintained-consumer expectations:
distributed HTTP now has 89 standard tests plus one HTTP test (90); the offline
diamond has 89 plus four (93), and the nominal consumer retains 89. Fresh native
policy, text-join and web-starter tests likewise count 100, 92 and 121 respectively.
Pinned historical native-guide/policy suppliers keep their old counts. The service
oracle's exact artifact binding is updated to the already native-rebuilt lkjournal
bundle, `590aac1f684791259b041f71bd4a2758d3f64db05595a7ddb7b1bc5d1240d932`;
its semantic HEAD, selected old standard, protocol, routes and workload are unchanged.
The service identity check is retained, not replaced by accepting arbitrary bytes.

Focused successor verification completed in
`/tmp/lkjscript-buffer-annotation-fix-tests1.log`: 25 ByteBuffer cases passed in
5.31 seconds; 48 artifact-filter cases passed in 31.06 seconds; the explicit
Graph 14 predecessor-type/nominal-data case passed in 1.74 seconds. Each filter had
zero failures and zero ignored cases; the filters overlap and are not claimed as
74 distinct tests. This includes the original maintained-application failures and
the new nonbuffer/mixed-buffer annotation controls. Integrity-repaired affine
counterexamples continue to reject. All test builds used the pinned locked toolchain.

## Final public-consumer inventory alignment

Source `551f6293e54c43b158c9cba53e69a7e495edbec7` received the next full-profile
attempt at `.artifacts/lkjscript-dev/check/1790745523205843775-2350173-0/receipt.json`.
It completed with 25 fresh passing gates, one failed gate, no skipped gates, no
reuse and stable inputs in 804.281943977 seconds. The library suite passed 974
cases with eight existing ignored cases. The public CLI suite passed 162 cases,
failed ten and retained one existing ignored case; this is not full acceptance.

All ten public failures were exact inventory expectations that had not yet been
updated: one missing `byte-buffer` discovery form and nine observations of the
four newly added standard tests. Eight test files now retain exact expectations
for the new inventories. Web snapshot and literal editing expect 32 local plus
89 standard tests (121); forms expect 65 plus 89 (154); the editor starter expects
121 plus 89 (210); the paged-list consumer expects 36 plus 89 (125); freshly
authored native guides expect 16 plus 89 (105). Historical pinned guide/policy
bundles, data values, execution semantics, source-free assertions and all failure
checks remain unchanged. No gate is waived and no test is removed.

The focused public run `/tmp/lkjscript-buffer-final-cli-alignment.log` selected
12 cases, including two additional existing form rejection controls. Eight passed;
four reached the next stale expectation only after their library checks succeeded.
The imported form consumer expects 66 local/dependency plus 89 standard tests (155),
and the downstream HTTP receiver expects 70 plus 89 (159). Those later expectations
are updated as well. This first focused run finished in 43.05 seconds with no
ignored cases and is retained as failed, not relabeled after correction.
