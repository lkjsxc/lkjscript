# Independently checked dependency plans

Build a native result checker independently of the native planner. The checker
accepts an original proposal and an untrusted claimed plan, not the planner's code
or a purported producer certificate. Checked flat and chunked entrypoints then use
that checker through an exact exported package before returning a valid plan.
[The decision](../../../docs/decisions/native-plan-result-checking.md) owns the
predicates and trust boundary; [status](../../../docs/status.md) owns acceptance.

## Author the checker before the producer

Use the existing [worklist sequence](../../owned-worklists/README.md) to export the
standard, worklist contract and independent carrier packages. In a fresh minimal
checker project, stage all three and bind the exact package IDs, semantic revisions
and package revisions. Use aliases `owned-worklists` and `worklist-carriers` for the
latter two. Author `../../owned-worklists/reachability.lkjc`, followed by these
literal requests in order: `data.lkjc`, `members.lkjc`, `edges.lkjc`, `walk.lkjc`,
`stages.lkjc`, `verify.lkjc`.

Each file is an ordinary request with the project's current `request base=REVISION`.
Use `change plan`, then `change apply` with its returned token. The initial request
adds the exact supplier bindings; subsequent requests retain the explicit aliases.
Do not concatenate repeated builtin alias clauses into one units collection.
Run `check`, verify unchanged canonical drafts, and build `dependency-plan-verifier.lkja`.
Export this checker with `package current export --kind transport --output verifier.lkjp`.
It is executable before the producer project or producer artifact exists.

This checker project contains the shared proposal admission/reachability library,
but no component-order, component-groups, component-data, dependency-components,
plan-edges, plan-stages, plan-presentation or dependency-plan modules. Checking
strong connectivity and stage equations does not call those producer algorithms.

In a separate fresh producer project, stage the standard, worklist and carrier
packages and this checker transport. Bind its exact observed identity under alias
`dependency-verifier`. Follow the [planner's authoring sequence](../README.md),
then author this directory's `application.lkjc` and `batch.lkjc`. The consumer names
`dependency-verifier::verify-dependency-plan` and the nominal
`dependency-verifier::Verification` exported by that independently built package.
Run `check`, verify unchanged drafts, and build `verified-dependency-plan.lkja`.
No external semantic generator or privileged graph writer participates.

## Public commands

Copy the selected executable and both artifacts outside the source checkout. Place
the supplied descriptors beside the corresponding artifacts. No configuration,
secret or effect grant is required. Ordinary foreground execution is retained.

```sh
./lkjscript run --deployment verify.deployment.json \
  --arguments-file claim.json --result-file verification.json
./lkjscript run --deployment flat.deployment.json \
  --arguments-file proposal.json --result-file checked-plan.json
```

Use absent result paths. `verify.deployment.json` selects the standalone verifier;
`flat.deployment.json` and `chunked.deployment.json` select checked planning. Their
valid-plan output is unchanged from the original planner. A valid result cannot
escape those checked entrypoints after a failed result check.

A standalone input is `[PROPOSAL, PLAN]`. For example:

```json
[
  {"roots":[10],"nodes":[
    {"id":10,"successors":[20]}, {"id":20,"successors":[]}
  ]},
  {"components":[
    {"members":[10],"cyclic":false,"dependencies":[20]},
    {"members":[20],"cyclic":false,"dependencies":[]}
  ],"stages":[[20],[10]],"reachable":[10,20],"unreachable":[]}
]
```

The result is `{"case":"verified","value":null}`. Claiming one cyclic component
`[10,20]` with no external dependencies and stage `[[10]]` instead is rejected with
`component-connectivity`: reaching 20 from 10 does not establish the reverse path.
A claimed partition splitting a real cycle cannot satisfy the cross-component
stage equations. A topological but unnecessarily delayed stage assignment also
rejects; the contract requires the earliest stages, not just any valid ordering.

`verify-batch.deployment.json` takes `[[{"proposal":PROPOSAL,"plan":PLAN}, ...]]`
and returns one verification outcome per sample. Checked planning batches use
`batch-flat.deployment.json` or `batch-chunked.deployment.json`, with the original
`[[PROPOSAL, ...]]` input and one planner outcome per proposal. Empty batches return
`[]`. Every sample has independent graph/claim state; batching shares preparation.

After building, source projects and package transports can be deleted from the
owned disposable test root. Detached execution uses neither those files nor the
host oracle's expected results. Retain literal requests and inputs as evidence.

## What rejects

Complete source validation and its capacity precedence run before claim checking.
Malformed raw types in any argument still fail before native execution, including
a malformed claimed field paired with an already invalid source. The checker
inherits `invalid` and `capacity` source outcomes rather than claiming verification
of such a plan. Runtime refusal or interruption cannot produce a successful partial
verification result.

For a valid source, the checker returns `rejected` with one of these codes:

| Code | Failed condition |
| --- | --- |
| `component-members` | Nonempty, exact once-only coverage and authored identity order. |
| `reachability` | Complete ordered reachable/unreachable lists for these exact roots. |
| `component-dependencies` | Exact distinct cross-component dependencies in first-reference order. |
| `component-cyclic` | Multi-member cyclic status or the actual singleton self-loop flag. |
| `component-stages` | Exact representative coverage, peer order or earliest-stage equations. |
| `component-connectivity` | Forward and reverse reachability within a claimed component. |

Input identities include the full signed I64 range. -1 is not a special node ID.
Representatives must be the first authored member, not another member with the
same component meaning. Claimed arrays cannot raise the admitted graph capacities.
Source invalidity and malformed claims are distinct from runtime resource refusal.

## Independent acceptance evidence

The maintained regression first builds and runs the standalone checker before
creating any producer project, then imports its exact transport into both planners.
It compares complete checked plans and standalone reports for 4,635 source cases,
including all three-vertex directed graphs and all loop-free four-vertex graphs,
before and after deleting authoring projects and transports.

A separate adversarial family enumerates all 512 three-vertex directed graphs and,
for each, every canonical set partition and every gap-free assignment of its
components to ordered stages: 11,776 complete claimed plans. Exactly 512 claims are
correct; the other 11,264 must reject. These are not 11,776 different input graphs
or all possible malformed serialized claims. Forty targeted corruptions/source
changes plus one recovery cover identity/order/cyclic/reachability boundaries,
one-way connectivity, delayed stages and source-before-claim precedence.

The test-only oracle uses pairwise reachability and synchronous level relaxation.
It does not run the native finish-order SCC algorithm, producer readiness queue or
checker traversal. Eight new fixed native tests exercise both evaluators, including
both one-way false merges and propagation of checker failure by the consumer.
The public matrix uses production execution, not differential evaluator pairs.

Every successful public run checks joined cleanup and no remaining owned handles,
locals, operands, frames, transactions or type bindings. Test batches fit the
unchanged 1 MiB argument limit; large inputs are separated, not admitted by raising
that limit. Artifact/executable bytes are checked unchanged across detachment.
Counts and original receipts describe successful acceptance only where status says
so; development failures remain retained separately.

## Limits and next connection

This is result checking, not a privileged certificate type. The returned Unit is
ordinary data and must not be attached to another proposal or used as publication
or execution authority. The accepted meaning graph remains the sole program-meaning
authority. Generic method types still do not prove the producer's algorithmic laws.
The checker shares complete source admission/reachability, standard collections and
runtime machinery; it is not independent of every layer of the implementation.

The checked interfaces are a candidate boundary for later compiler tooling. No
production compiler pass or scheduler is replaced here. Their exact selected checker,
source-scoped identities and complete current input still need to be bound at that
host boundary. Cyclic components do not acquire a sequential schedule, and equal
stages do not prove disjoint effects or safe parallel execution.

There is additional checking work. Bounded graph visits do not imply linear wall
time, zero copying or bounded RSS. No runtime, encoding, dependency version, existing
planner API or product identifier changes are required by this native addition.
