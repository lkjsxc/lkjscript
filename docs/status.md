# Current status

Snapshot: 2026-10-08. Source acceptance, distribution acceptance and running
applications remain separate boundaries. Product identifier components are opaque.

## Accepted source and retained evidence

Accepted source: `403683601f5adfed4e6cd0750224dc2f48fe3dc4`, tree
`269df09ff7ea4d11a070cef3e4905568b9c8a143`. Its fresh full source verification
passed 26/26 gates, with stable inputs, zero reused evidence and zero unrun gates.
Workspace tests passed 2,236, failed 0 and retained 29 existing ignored tests.
The 234 public CLI tests are included; two successful filtered child probes are
separate. The source was normally pushed to main and independently observed there
through Git and the GitHub ref API. A status-only reporting descendant is not the
source to which this full receipt belongs.

Verification/evidence checkout:
`/home/coder/workspace/lkjscript-native-plan-verification-20261008`, branch
`work/native-plan-verification-20261008`. The following receipt and evidence paths
are relative to that checkout, not to the original main checkout.

Receipt:
`.artifacts/lkjscript-dev/check/1791458711573539146-37009-0/receipt.json`, digest
`verification_01dd79d3c40deec4f1edffb8872e380d38371dba23953b26072f08781d8bdbff`.
The complete run is retained outside checker rotation in
`.artifacts/native-plan-verification/full-source-verification/`.
The evidence index is `.artifacts/native-plan-verification/evidence-index.json`.

## Independently checked native plans

A separately authored native checker now verifies a complete claimed dependency
plan against its original proposal. It is built and exported before the producer
project exists. Checked flat and chunked planner entrypoints import that exact
checker package, verify every generated valid plan and return the existing
`inconsistent` outcome on a failed check instead of exposing a valid result.
See the [decision](decisions/native-plan-result-checking.md) and
[public authoring guide](../examples/dependency-plan/verification/README.md).

The standalone artifact contains the existing complete proposal validator and
reachability implementation, but none of the producer's SCC, condensation or
readiness-queue planner modules. It checks exact ordered member coverage,
reachability lists, distinct cross-component dependencies, cyclic flags, stage
coverage/order, earliest-level equations and two-way connectivity inside each
component. Internal connectivity prevents false merges; strict cross-component
level descent prevents splitting a true SCC. This is not a second execution of
the producer algorithm, nor a formal verification of the checker implementation.

The final copied release-build executable passes all five tests in the new public
integration target. Its 317 batch executions check 39,627 conditions: 18,540
complete checked plans, 9,270 standalone reports for the existing 4,635-case graph
family, 11,776 exhaustively enumerated three-vertex candidate plans and 41 targeted
corruptions/source changes/recovery cases. Exactly 512 exhaustive claims are valid;
the other 11,264 reject. These are candidate claims, not 11,776 different graphs.
The largest argument file is 877,673 bytes; the 1 MiB runner limit is unchanged.

Three single commands, three empty batches, two raw-type/resource refusals and two
subsequent recoveries also pass. A separate standalone command executes before any
producer project/artifact exists. Both carriers and ordinary standalone reports
match independent results with sources present and deleted; adversarial claims run
after deleting all four source projects and transports. Successful runs observe no
remaining owned handles, locals, operands, frames, transactions, type bindings,
tasks or workers. Eight new fixed native tests run through both evaluators; the
complete checker and consumer projects report 106 and 139 passing tests respectively.
The public matrix uses production execution, not differential evaluator pairs.

Final public originals are retained in
`.artifacts/native-plan-verification/final-public-proof/` (original temporary root
`/tmp/lkjscript-components-A5umOe`). The accepted-source test executable was selected
from the full workspace Cargo output and copied before use. Both it and the selected
product were executed without inherited publishing credentials. The product's
SHA-256 is `c6af3e5a3f6a40a87ec27b3af55f0535647c339020d041b46c06115b530aa89c`;
the test executable's is
`6f3809340b024bc6b2b073d736d68922c4031b1d9a7eaa175f41267679e3d483`.

The final standalone artifact has SHA-256
`6ce27ca5083e76808171a247dcc36d2ef73d60a661e3c9fe03ba76c6dc5f1d27`; the checked
producer artifact has
`d2991ac50c4d9d51382f2b4bbde6861494e4d0bd75f34ed9990af4e726d9b22f`.
Artifact and executable bytes stay unchanged during their detached proof. An
earlier focused executable had different bytes, so its evidence was not silently
assigned to this producer. Two corrected authoring failures, the earlier successful
proof, full acceptance and the final executable's fresh proof remain distinguished.

No Rust runtime, intrinsic, dependency, encoding, existing planner API or product
identifier changes are introduced; development remains `0.1.88`. Additional result
checking is real work. No speedup, copying, bounded-RSS or API-cost claim is made.

## Trust and next compiler boundary

A `verified` report is ordinary data, not an unforgeable certificate, accepted
meaning, an execution grant or publication authority. Reuse must bind the complete
proposal/claim and exact trusted checker. The checker shares source admission,
reachability, standard collections and runtime machinery with the producer.
Checked wrappers guard valid outputs; they do not independently certify arbitrary
negative producer outcomes. Cyclic groups have no implied sequential member order,
and stages do not prove disjoint effects or authorize parallel execution.

The next concrete connection is a real compiler consumer binding exact source-scoped
identities and complete dependency extraction to this checked interface. No compiler
pass or scheduler is replaced here. Additional compiler inspection was refused by
the tool and was not retried or bypassed; it is not adoption evidence. Prefer this
connection over adding another disconnected graph-analysis example. Generic method
types alone still do not prove implementation laws such as LIFO behavior.

## Preserved unfinished runtime work

The original checkout `/home/coder/workspace/lkjscript` retains two unrelated,
pre-existing, unintegrated test changes: `src/platform/execution/normalized/vm_map_tests.rs`
and the untracked `src/platform/execution/normalized/vm_map_entry_scan_tests.rs`.
They are not part of the accepted source above. Their last separately recorded run
passed 59 tests and failed the new entry-scan complexity law; no lookup-free map
projection is implemented by this native checker. Preserve those files, the existing
stash and the original failed evidence. Their record remains at
`.artifacts/20261008-map-entry-scan/resume-evidence-index.json` in the original checkout.

The existing [borrowed immutable metadata](decisions/borrowed-immutable-metadata.md)
implementation remains intact. Consuming extraction still needs an inseparable
exact-parent/field proof before checked construction may reuse it. Free-value
classification, apparent shape or an address cache is not that proof. Neither this
work nor its full source acceptance completes the unfinished runtime optimizations.

## Distribution and running applications

The independently observed public release remains immutable `v0.1.83`. The previously
observed `v0.1.88` producer `37643217328/1` belongs to source
`6a15fdab20cdbf89ebc8683e8aa1d8b0f64b7142`, not this accepted source. This step did
not dispatch a producer, replace artifacts, promote a release or deploy applications.
Publication is deferred for this native-only addition. After the next selected
runtime/distribution milestone, bind its exact final source and transferred
executable through the maintained release procedure; do not relabel the old candidate.
