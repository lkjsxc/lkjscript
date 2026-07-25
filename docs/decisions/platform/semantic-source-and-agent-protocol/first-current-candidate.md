# Semantic Source And Agent Protocol: First Current-Candidate Operations

[Authority](../semantic-source-and-agent-protocol.md)

## Status

**Accepted Implementation Contract.** These are the complete operations for the
first Current-candidate slice. They are not Current endpoints. Foundation V1 is
Current; complete Schema/Protocol V1 remains an Accepted Target.

## Envelope

Every request and response carries its exact registered schema name and integer
version. Requests select a resource profile and name the complete loaded source
closure. Success identifies compiler build, profile, exact source revision, and
aggregate charges. Failure is a structured protocol diagnostic and grants no
partial authority. Unknown fields, variants, duplicate fields, trailing values,
and unknown versions fail.

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
and one new revision. Any failed precondition/check/publication leaves memory
and files byte-identical to the base revision. Comments and string contents are
not renamed by text search.

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
The dependency addition remains not Current until license/notices, advisory,
fuzzing, malformed-boundary, and locked-version gates pass.

## Acceptance

Focused tests cover deterministic success, every stale identity/precondition,
unknown versions/fields/variants, duplicate/trailing JSON, malformed Unicode,
all aggregate boundaries, rename collisions/cross-unit references, replacement
type/effect/ownership failures, rollback after each staged phase, and
same-directory atomic publication failure. Canonical compile/evaluator/VM/JIT
semantics must remain unchanged.
