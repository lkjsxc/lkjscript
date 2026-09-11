# Explicit effects and reusable task libraries

Status: implemented in the working source; campaign acceptance and selected publication are tracked
in [202609111843](../campaigns/202609111843.md). This decision is a contract owner, not a release receipt.

## Problem and replacement

Closed task effects and pure-only callable types prevented an offline library from invoking a
consumer callback that reads configuration or writes data. Function-owned rank-one effect parameters
now describe explicit finite sets of exact requirements. Pure factories can return configured task
descriptors; a generic task can invoke an equally typed callback without owning its concrete
requirements. Invocation still needs both the current activation allowance and actual checked grants.

The standard owns sequential range `task-fold-left` and a `task-map` built from that fold, a bound
mapper step and persistent append. Splitting the original list's index range gives logarithmic
additional traversal frames without task TCO, slices, host traversal loops or scheduling policy.
Indexed reads retain their existing logarithmic work. Earlier effects survive failures except where
the applicable lexical transaction rolls back staged writes. Ordinary returned error cases are data.

## Contract change table

| Problem | Replacement and affected domains | Preserved identities | Rejected predecessors and recovery | Retired paths |
|---|---|---|---|---|
| Tasks had closed rows and could not declare type parameters | Graph 13 → 14: ordered function-owned effect parameters, canonical exact rows, generic tasks; validator 13 → 14, witness/owner-summary 8 → 9 | Existing owner IDs, names and ordinary generic recursion contract; unchanged type bytes | Graph 13 owner/repository inputs reject without mutation. Retain predecessor exports and matching v0.1.31 executable | Generic-task rejection in the current canonical owner |
| Pure callable types could not describe tasks | Task-function envelope `LKJTFN01`, version 1, with exact row and signature | Pure TypeObject 10 and nominal-application envelope 1 bytes/digests; unchanged typed-data layouts | Effect-erased, wrong-kind or noncanonical task types reject | Task-as-pure canonical/compiled/prepared port path and its purity exceptions |
| Authoring and inspection omitted effect scope/application | Authored change 14 → 15, compact change 17 → 18, definition projection 5 → 6, registry 16 → 17, CLI 30 → 31 | Zero-effect-arity request meaning and existing declaration identities | Foreign/missing/excess effect arguments, stale reviews and incompatible contracts reject. Reauthor through discovered current operations | Body-only certification of effect/signature edits is ineligible |
| Transported executable metadata could erase effects | Interface-owner 9 → 10, compilation-unit 9 → 10, bytecode 5 → 6, Artifact 17 → 18; strict canonical cross-checks and complete prepared application keys | Package identity, transport framing, deterministic exact dependency selection; self-consistent current older bundles remain executable | Old Artifact 17 rejects before preparation. Rebuild a supported current project or use the matching predecessor executable | Trusted-producer assumptions and old task port representation |
| Grant availability alone could widen indirect calls | Independent VM/reference activation allowances plus canonical grant identity; exact package/name/interface, operation containment and compatible limits | Adapter instances, counters, resource provenance, lexical transaction ownership, runtime/fuel policies | Forged descriptors, foreign preparation, undeclared or wider shared-grant calls reject before body effects | Interface/name-only coverage on the changed invocation boundary |
| Existing receipt semantics did not prove task composition | Offline-package receipt 5 → 6, within the existing five-owner aggregate; stateful HTTP lifecycle reuse | Public-pair independent acquisition/admission and within-pair binding policy; other receipt domains | Missing effect workload, source binding, stopping, resource or transaction observations reject transfer | No added aggregate or parallel verification framework |

The package version is selected independently as 0.1.32 after an actual remote read found it unused.
No operational data, deployment descriptor, grant placement or immutable published identity changes.
Supported execution still uses explicit component targets; pure `run` remains pure-only.

## Preserved meaning and admission

The retained [Graph 13 transition inventory](../../tests/fixtures/graph13-effect-cutover.json) binds
the actual startup observation at `f5bc5db6485affe92daf4ced2c026027121d5bec`, all predecessor owner
identities and meaning hashes, explicit additions/port replacements, retirements and local type bytes.
The immutable Graph 10 typed-data and Graph 12 nominal fixtures remain independent predecessor
oracles; their tests account only for the explicitly enumerated successor changes. Actual predecessor
standard transport/artifact fixtures are retained for strict rejection, with no automatic converter.
All 2,842 predecessor owners remain; the standard adds 86 graph owners and lkjournal keeps all 2,040.

Production and reference derive their own effect closure, signatures and allowances. They share
neutral identities and value carriers, not a production authority oracle. Raw descriptors carry an
exact prepared origin/target, resolved ordered arguments and a flat capture-safe prefix; no grants,
task scope, transaction, credential or live resource can be retained. Type traversal visits task
signatures for finite nominal analysis while treating signatures as leaves for capture safety.
JSON, durable typed data and session retention reject callable-containing types even for empty or
inactive values. This also closes the JSON empty-container bypass found by the new boundary test.

The independently retained legacy semantic-model tests still describe their predecessor model.
Their task-value flag has no caller on the canonical executable/package preparation path. Current
kernel, compiler, normalized execution, HTTP, worker and session entries have one explicit task
callable representation; no executable compatibility branch reinstates task-as-pure ports.

## Proof and limits

The campaign archive distinguishes fresh development runs, retained predecessor observations and
pending source/target/publication gates. The new public workload is separate from maintained adoption:
it creates a producer and consumer outside the checkout, transports a generic library, removes the
producer, and exercises a pure bound factory through configuration/data callbacks over parametric and
recursive nominal values. Reviewed exact dependency replacement changes complete predicted outputs;
the old self-consistent artifact remains available for standalone recovery.

The neutral probe compares map/fold with an equivalent concrete task traversal at 0, 1, 31, 32, 33,
4,097 and 8,192 elements. Measurements separate preparation, execution, allocation, indexed reads,
frame high-water, disabled observation cost and cleanup. The claim is logarithmic additional
traversal control space, not faster IO, linear indexed access, task TCO, hostile-code isolation or
a throughput threshold. Product resource defaults are unchanged. Separate controlled stopping probes
select smaller fuel/allocation budgets; live transactional cancellation is executed only once.
