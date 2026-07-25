# Semantic Source And Agent Protocol: First Current-Candidate Operations

[Authority](../semantic-source-and-agent-protocol.md)

## Status

**Historical.** This records the formerly Current bounded one-shot
`lkjscript.agent-foundation/1` slice. Complete Semantic Source Schema V1
superseded its public identity without retaining an accepted alias. Its exact
operations and evidence remain historical; sessions, typed holes, and wider
operations remain Accepted Targets.

## Envelope

Every request and response carries its exact registered schema name and integer
version. Requests select a resource profile and name the complete loaded source
closure. Success identifies compiler build, profile, exact source revision, and
aggregate charges plus profile schema/version, implementation-maxima version,
effective protocol ceilings, and core-ceiling SHA-256. The closed profile names
are exactly `sandbox`, `default`,
`build`, `trusted-local`, and `deterministic` from Resource Profile V1;
`standard` and every other name fail. Protocol request/response bytes, source
bytes/units/schema nodes, and validation work use the selected core ceilings
intersected with stricter immutable protocol/foundation maxima. Transactions
add the complete staged closure to source byte/unit/node charges before any
publication. The protocol ledger remains request-local rather than pretending to be the compiler ledger.
Failure is a structured protocol diagnostic and grants no partial authority.
Unknown fields, variants, duplicate fields, trailing values, and unknown
versions fail.

## `snapshot`

Input is the canonical root and optional exact expected repository identity.
The operation loads and validates the complete bounded closure once. Output
contains revision counter/fingerprint, source units in canonical path order,
exact input fingerprints, declarations in stable-key order, stable declaration
keys, revision-scoped dense node IDs, spans, compact structural summaries, and
all limits/charges. It returns no inferred compiler fact as source authority.

The same closure, compiler build, profile, and bytes produce byte-identical
canonical response content apart from a daemon's separately identified session
counter. Enumeration order, process identity, and wall clock cannot affect it.

## `read_entity`

Input names exact revision plus one complete stable declaration key and optional
entity precondition fingerprint. Output contains that declaration's full source
schema subtree, descendant node IDs/spans, exact source unit, entity
fingerprint, and explicitly separate available derived-summary references. A
missing, duplicate, stale, or wrong-kind key fails; prefix and text-name search
are not accepted identity.

## `query_node`

Input names exact revision and one revision-scoped node ID. Output repeats the
node identity, stable containing entity, source kind/span, and only compiler-
derived facts available after the requested analysis level: resolved binding,
static type, effects, ownership/place/loan state, control-flow relation,
layout, or proof relation. Every fact names its producer/schema version and
certainty. Unavailable analysis is an explicit `unavailable` value, not an
omitted success or guessed fact. A node from another revision is stale.

## `diagnostics`

Input names exact revision and requested completed analysis level. Output uses
`lkjscript.diagnostic` version `1`, sorted by severity, stable code, canonical
source path, primary span, and deterministic tie-break identity. It includes
zero diagnostics explicitly. Truncation is a resource-limit failure for this
slice; it cannot silently present an incomplete clean result.

## Atomic `rename`

Input names base revision, exact declaration key, declaration precondition,
new valid source identifier, and expected preconditions for every resolved
reference owner. The engine resolves the declaration and complete loaded-
closure reference set before staging. It rejects ambiguity, collision,
unsupported external reference domains, stale reads, and invalid spelling.

Success changes the declaration and every resolved reference as one
transaction, reruns structural/name/type checks, formats affected units, and
returns old/new stable keys, exact changed entities, semantic diff, diagnostics,
and one new revision. Before any source rename, the exact success response is
encoded within its output bound. Repository publication holds one local
repository exclusion lock and writes an atomically replaced, synchronized
recovery journal below `target/lkjscript/semantic-staging/`. Every later request
recovers an uncommitted journal before loading source. On Current Linux, each
source parent is opened and verified as its exact canonical repository
directory, and leaf operations remain anchored through that descriptor.
Installation uses no-replace linking, so an external leaf created after backup
is preserved rather than overwritten. Final backup and installed bytes are
rechecked before commit. Any ordinary failed precondition/check/publication
restores complete old files; a process crash is rolled back before the next
protocol read. Comments and string contents are not renamed by text search.

## Atomic `replace_expression`

Input names base revision, exact containing declaration key/fingerprint,
target node ID/fingerprint, and one complete replacement expression subtree.
The target must be an expression position in the first schema vocabulary. The
replacement is validated under the exact expected type, visible bindings,
allowed effects, ownership constraints, and resource profile before staging.

Success replaces exactly that subtree, rebuilds node IDs deterministically,
rechecks the complete affected declaration and dependants required by analysis,
formats affected units, and returns old/new node relationships, semantic diff,
diagnostics, and one new revision. Text-range replacement, implicit coercion,
partial publication, and best-effort repair are rejected.

## Serialization Selection

The accepted boundary pins direct dependencies exactly to `serde 1.0.229` and
`serde_json 1.0.151`, with derive-enabled typed request/response structs and
lockfile retention. Serde is confined to the protocol/serialization crate
boundary; validated in-memory authorities are not publicly constructible by
deserialization. Boundary types deny unknown fields and use closed tagged enums
with explicit schema/version checks. Untyped `serde_json::Value` authority is
rejected.

Before parsing, the endpoint checks exact request bytes. During/after parsing it
checks nesting, strings, collection entries, schema nodes, operation count, and
work categories. Duplicate fields, malformed Unicode/escapes, trailing data,
unknown variants, non-integer identity/counter encodings, and overflow fail.
The locked dependency selection is Current. Both crates are MIT OR Apache-2.0;
the reviewed OSV queries reported no advisory, and strict malformed-boundary
tests pass. Dedicated fuzzing and `cargo-audit` remain explicitly untested; the
latter tool is not installed. Those omissions are not presented as evidence.

## Acceptance

Focused tests cover deterministic success, every stale identity/precondition,
unknown versions/fields/variants/profiles, duplicate/trailing JSON, malformed
Unicode, all five profile selections and aggregate boundaries, rename
collisions/cross-unit references, replacement
type/effect/ownership failures, rollback after each staged phase, and
same-directory atomic publication failure. Canonical compile/evaluator/VM/JIT
semantics must remain unchanged.
