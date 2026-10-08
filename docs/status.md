# Status

Current snapshot: 2026-10-08 (Asia/Tokyo). This page owns availability and unfinished
acceptance. [Direction](direction.md), [specifications](spec/) and [roadmap](roadmap.md)
own contracts and future work. External states below are observations, not promises.

## Accepted mainline: v0.1.88

Selected checkout: `/home/coder/workspace/lkjscript`, branch `main`.
The last independently confirmed remote main is
`6a15fdab20cdbf89ebc8683e8aa1d8b0f64b7142`, tree
`864623d89c619bbc56744314db24293ebea606fa`.
[Operation-local incoming call inputs](decisions/operation-local-callable-inputs.md)
follow the accepted worker-custody/local-dispatch work. Their original acceptance
is indexed at `.artifacts/20261007-call-transfer-inputs/source-acceptance.json`:
26 fresh gates, stable inputs, no reuse, 2,205 workspace successes, zero failures
and 29 existing ignored cases. This receipt does not cover the new files below.

## Native dependency components: source acceptance in progress

[Native dependency components](../examples/dependency-components/README.md) extend
the provisional graph validator with complete strongly connected components,
cyclic/acyclic classification and the retained reachability partition. The
[decision](decisions/native-dependency-components.md) owns the scope. All five
implementation modules are literal lkjscript inputs, authored through ordinary
public operations; the host supplies inputs and an independent expected-result
oracle, not program output. There is no new intrinsic, graph writer, runtime
feature or production compiler substitution.

The inherited diagnostic test error is corrected: the test now checks exact class
`resource` and code `normalized_instruction_steps`, not a substring of explanatory
text. The complete corrected public regression passed. Empty-batch native checks
and detached public executions have since been added; their focused verification
and fresh full source acceptance are pending. Current logs are under
`.artifacts/20261008-native-components-acceptance/`.

The public regression compares 532 proposals, including all 512 directed
three-vertex graphs, under flat and chunked storage both before and after deleting
its disposable authoring projects and transports. These are 2,128 complete graph
comparisons, plus single-example and empty-batch runs. It checks exact typed
invalid/capacity outcomes, wrong-type and explicit runtime refusals without result
files, unchanged artifact/executable bytes, zero remaining owners and joined
cleanup. The first traversal needs LIFO behavior; a matching Worklist type alone
does not prove that algebraic law. Helper preconditions are established by the
complete application validator.

Original failures and evidence remain unchanged at
`.artifacts/20261008-native-components/handoff.json` and its linked originals.
The earlier full receipt
`.artifacts/lkjscript-dev/check/1791399198827777771-1806530-0/receipt.json`
failed one of 26 gates because of the diagnostic assertion; it is not relabeled
as successful. Additional raw-admission investigation was blocked before execution
by tool preflight in this continuation; no alternate route was used for that
request. Runtime optimization is not part of this change.

Next: finish the focused regression, commit intended source, run dependency-complete
`check full --fresh --machine` with stable inputs, then integrate normally and
independently confirm remote main. Record the exact accepted source and receipt
before replacing this pending handoff.

## Distribution remains separate

A fresh authenticated API read still identifies immutable
[v0.1.83](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.83), release
`404980756`, as latest. Its accepted source is
`65b3d00428d36827f9d65bf92f7900cf52719d21`.

The existing v0.1.88 producer
[37643217328/1](https://github.com/lkjsxc/lkjscript/actions/runs/37643217328)
previously reached `candidate_accepted` for source
`6a15fdab20cdbf89ebc8683e8aa1d8b0f64b7142`; publication and public verification
were skipped. Original authenticated terminal/acceptance ZIPs and jobs metadata
remain in `.artifacts/20261008-native-components/prior-candidate-*`.
Archive SHA-256: `4d19bb0c33625fd806ba91a643ac172104819f56a19d57d739091972b6a197ba`.
Executable SHA-256: `b25a42987e2bbf4e5e57bcb28fe41e26010171160b11e3b073357d9430320a8a`.
Service asset retention begins expiring at `2026-10-21T17:30:55Z`.
These are original candidate observations, not a new finalized-byte acceptance
for the native witness. Do not rebuild or duplicate a healthy accepted producer.
Promotion requires the maintained publication/public-verification boundary with
fresh occupancy and authority checks. No release operation is part of this
source-only continuation; no product encoding or compatibility cut is introduced.

Existing stashes, unrelated worktrees, immutable/failed history, services and
operational data remain preserved. Source acceptance, finalized distribution
acceptance and public availability remain separate claims.
