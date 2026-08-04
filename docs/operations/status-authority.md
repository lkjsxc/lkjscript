# Public Fact And Documentation Authority

## Purpose

Define one checked authority for public fact identity, lifecycle status, interface
scope, exclusions, evidence, and documentation projections.

## Status

**Accepted Contract.** The existing capability-status registry and exact status
marker check remain Current only for their narrow legacy behavior until the
atomic migration below passes. They do not provide fact, exclusion, projection,
or impact closure and cannot certify documentation coherence.

## Problem And Threat Model

Current prose can outlive a removed command, broaden a narrow capability, import
Historical wording, or copy a crate or engine list that already has another
machine authority. A passing link and marker scan does not detect those faults.
Bounded agent context can then rank stale prose above implementation authority.

The hard boundary is deterministic tracked input, exact content identity, and
closed typed records. Natural-language equivalence and LLM review are untrusted.
Malformed, excessive, escaping, cyclic, conflicting, or stale input publishes no
partial registry, graph, report, or projection result.

## Authority Order

1. Executable registries, Cargo, capsule manifests, structure policy, contract
   descriptors, compiler vocabulary, platform revision, and unsafe registry own
   their machine facts.
2. The public-fact authority owns only stable public fact identity, status,
   positive interface, explicit exclusions, and projection closure.
3. Accepted decisions own intended semantics for their exact scope.
4. Implementation anchors and retained evidence support a fact but do not set
   its status.
5. Current State and entry documents are checked projections, never competing
   registries.
6. Historical, Superseded, Rejected, Deferred, and Experimental material grants
   no Current behavior or compatibility fallback.

## Closed Status Vocabulary

- `current`: complete for the exact positive interface and exclusions, with
  applicable implementation and evidence closure.
- `accepted-contract`: accepted interface awaiting complete implementation.
- `accepted-target`: binding destination that is not implemented as a whole.
- `accepted-selection`: selected measured candidate awaiting integration.
- `experimental`: bounded implementation or investigation without adoption.
- `deferred`: explicitly outside the active sequence.
- `rejected`: evaluated and not accepted.
- `superseded`: retained decision history only.
- `historical`: immutable or recorded-baseline evidence only.

Accepted Contract, Accepted Target, and Accepted Selection are distinct. No
other status spelling, capitalization, or implied fallback is accepted.

## Fact Identity And Record

A fact ID is stable lowercase ASCII kebab-case and carries no platform revision,
edition, schema generation, ABI generation, or implementation strategy. The
strict record can represent:

- fact ID, closed kind, status, and semantic scope;
- one canonical positive interface and sorted explicit exclusions;
- one authority path or named machine source;
- sorted implementation anchors and typed evidence records;
- sorted projection targets, dependency facts, and invalidating facts;
- the platform revision at the cut; and
- relevant contract digests plus derived content and fact-closure digests.

Machine-derived crate, command, contract, suffix, vocabulary, unsafe, topology,
and platform facts remain derived from their existing authorities. They are not
copied into hand-authored fact fields. Capability records may point at those
sources without becoming a second Cargo, CLI, or contract registry.

## Positive Interface And Exclusions

The positive interface is the narrowest supported public behavior. Exclusions
are separate machine fields and are part of the fact digest. They distinguish,
where relevant, persistent from invocation-local, borrowed from owned, native
from VM, forced from automatic, Linux from non-Linux, process from in-process,
static from dynamic, and an exact type matrix from universal support.

A projection marker binds the fact ID, exact status, and complete fact-closure
digest. The marker claims the registered interface and all exclusions as one
bounded block. Missing, extra, stale, duplicate, wrong-status, or unregistered
markers fail. The checker does not pretend to prove arbitrary surrounding prose.
Focused deterministic retired-concept and migration lints cover registered
high-risk contradictions.

## Evidence And Implementation Anchors

Current facts name applicable implementation anchors and evidence. The checker
validates normalized contained paths, exact content digests, declared evidence
class, and any recorded commit, contract digest, platform revision, result, and
untested fields. A recorded commit must be exact and reachable unless the record
explicitly identifies external immutable evidence.

Freshness validation does not rerun a command. Parse, type, compile, build, link,
VM execution, native execution, process execution, measurement, and acceptance
remain distinct. Missing proof stays explicit rather than inferred from age.

## Document Classes And Context

- Current-facing documents project registered Current facts and exact gaps.
- Accepted documents describe destinations without projecting them as available.
- Historical and Superseded documents state a recorded baseline in historical
  tense or retain an explicitly delimited immutable quotation.
- Immutable evidence preserves exact bytes and is excluded explicitly from prose
  migration, while default context does not rank it above Current facts.
- Mixed-status documents delimit every status scope.

Default fact context includes interface, exclusions, authority, evidence
freshness, projections, dependencies, tests, and nearest accepted next work.
Historical records appear only through explicit edges or explicit requests.

## Projection And Impact Closure

The repository graph derives these explainable edges:

```text
fact -> authority and machine source
fact -> implementation anchor and evidence
fact -> interface, exclusion, status, dependency, and invalidating fact
projection -> fact
fact -> verification gate
command -> classified example
crate or capsule -> architecture projection
```

Impact from a changed fact, authority, anchor, command, contract, or crate reaches
every affected projection. Output is sorted, bounded, content-addressed, and
serialized only under `target/`. Unsupported edge classes and truncation remain
explicit. A globally truncated graph must still traverse retained focused edges.

## Commands And Generated Output

The canonical interfaces remain:

```text
cargo run --locked -p lkjscript-xtask -- check-docs
cargo run --locked -p lkjscript-xtask -- structure explain fact:<id>
cargo run --locked -p lkjscript-xtask -- structure context fact:<id> --profile strong
cargo run --locked -p lkjscript-xtask -- structure impact fact:<id>
```

`check-docs` writes canonical inventories, expected projection markers, impact
summaries, and diagnostics under `target/lkjscript/documentation/`. Checks never
rewrite tracked prose. Any later mutation command must be separate, atomic,
previewable, bounded, and leave no compatibility alias.

Shell and lkjscript blocks are classified as executable verification, safe
demonstration, illustrative syntax, privileged or external, or historical.
Only an explicitly local deterministic class may execute. CLI shape derives
from canonical command authority. Natural-language generated tests and LLM prose
review remain Experimental and cannot certify Current behavior.

## Resource Contract And Baseline

The cold starting baseline at commit `5edd9ed8` was 213 documentation inputs,
1,106,863 bytes, `check-docs` 1.615 seconds, graph 4,096 nodes and 9,344 edges in
1.165 seconds, and `quiet verify` 108.220 seconds. Peak RSS was unavailable.

The public-fact gate rejects before publication above 2,048 files, 16 MiB input,
16 shards, 256 facts, 32 members in any per-fact collection, 8,192 aggregate fact
members, 4,096 claims or examples, 16,384 fact edges, 262,144 work units, or 256
diagnostics. Arithmetic is checked and traversal is iterative.

Acceptance requires cold and warm byte-identical verdicts and generated output,
`check-docs` no slower than 3.25 seconds on the baseline environment, graph no
slower than 2.35 seconds, and `quiet verify` no slower than 130 seconds. Exceeding
a threshold falsifies promotion and records evidence; thresholds do not move
after results.

## Migration, Rollback, And Acceptance

Migration atomically replaces `meta/config/capability-status.json`, its contract
name, decoder, marker grammar, and consumers. There is no dual read, dual write,
old marker, alias, or compatibility decoder. The platform revision and exact
contract digests advance in the same cut.

The authority is not Current until strict decoding, projection staleness,
exclusion binding, graph impact, focused contradiction fixtures, corpus repair,
cold and warm determinism, structure checks, and canonical local verification
pass together. Failure leaves the prior Git commit as rollback authority; no
partially generated output is tracked.

## Deferred And Rejected

Complete natural-language semantic equivalence, automatic prose rewriting,
model-backed hard gates, network-dependent checking, and generated tracked prose
are Deferred. A second fact registry, copied Cargo or CLI inventories, universal
token bans, blind corpus replacement, unqualified Historical Current prose,
marker-only interface authority, and stale command aliases are Rejected.
