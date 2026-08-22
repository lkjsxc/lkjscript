# Semantic authority

Status: normative. Contract: `lkjscript-meaning-graph-1`.

## Authority

An accepted lkjscript program is exactly one validated typed semantic graph revision. The graph
owns repository and package metadata, modules, declarations, types, expressions, components,
ports, capability requirements, tests, semantic relations, retained documentation, annotations,
targets, exact dependencies, and deleted-identity tombstones.

Names are mutable presentation and lookup data. Stable IDs express continuity. Content digests
identify exact encoded content. Revision IDs identify exact accepted history nodes. Physical keys,
compiler indexes, runtime handles, rendered coordinates, caches, deployment grants, secrets, and
live resources are not program authority.

Maintained `.lkj` modules and `lkjscript.package.json` descriptors are forbidden. A text file may
be a non-authoritative review projection. A graph-native artifact may initialize a new authority,
and a verified backup may restore the same authority. Neither mechanism creates a second writer.

## Logical model

The canonical root owns one repository identity, package identity and name, an ordered set of
module references, exact dependency bindings, targets, and tombstones. Each module reference binds
a module ID and mutable name to one integrity-protected immutable module object.

A module object owns:

- module identity, namespace name, imports, exports, documentation, and annotations;
- declarations and their distinct stable identities;
- stable member identities for fields, cases, operations, parameters, requirements, and ports;
- stable binding and expression-site identities where public operations or relations need them;
- typed semantic operation trees; and
- sorted reference, call, type, field, variant, capability, port, target, and test relations.

Package, module, declaration, record, variant, interface, constant, pure function, task function,
component, port, capability requirement, test, expression, binding, documentation, annotation,
target, dependency, revision, receipt, draft, and conflict are logical semantic kinds. They need
not be separate physical records. Derived call/type/capability indexes are disposable acceleration;
the canonical relations in module objects and full validator remain their oracle.

Lexical comments, whitespace, source paths, source positions, and formatting preferences are not
meaning. The current shared Rust operation structs contain coordinate-shaped padding used by the
test-only text oracle; canonicalization sets it to one constant value, packed decoding rejects any
other value, and public semantic projections remove it.

## Identity domains

Stable semantic IDs contain 128 opaque bits, a textual domain prefix, and a packed domain tag.
Production allocation uses operating-system randomness. Independent branches therefore allocate
without coordination; collision remains an error, never an implicit merge. Test and one-time
migration allocation use a domain-separated deterministic seed and canonical ordinal. The seed or
ordinal is allocation input, not the resulting identity's meaning.

| Domain | Text prefix | Continuity |
|---|---|---|
| repository | `repo_` | backup and exact restore |
| module | `mod_` | rename and namespace change |
| declaration | `decl_` | rename and move |
| record field | `field_` | field rename |
| variant case | `case_` | case rename |
| interface operation | `op_` | operation change |
| parameter | `param_` | signature member continuity |
| binding | `bind_` | binding rename and selected rewrites |
| expression site | `expr_` | exact semantic replacement/rebinding |
| requirement | `req_` | task/component requirement continuity |
| component port | `port_` | port continuity |
| target | `target_` | runner-target continuity |
| draft | `draft_` | one non-executable work authority |
| conflict | `conflict_` | one closed conflict report |
| documentation | `doc_` | retained prose continuity |
| annotation | `annotation_` | retained metadata continuity |

Revision IDs use the distinct `rev_` domain and 256 content-derived bits. Package IDs remain the
language package domain and do not substitute for repository IDs. Every textual and packed decoder
rejects a foreign domain even when raw bytes coincide. Deleted IDs enter canonical tombstones and
may not be reused. Clone creates new IDs; restore applies explicit historical identity rules.

## Accepted revisions

A revision core contains contract versions, repository ID, zero, one, or two exact parent
revision/record pairs, canonical root digest, semantic diff digest, and transaction digest. Its
domain-separated digest is the revision ID. A revision record binds that core to one receipt. HEAD
binds repository ID, revision ID, and revision-record digest.

An accepted revision has no hole or conflict. All references resolve in its exact dependency
closure; identities, names, scopes, types, effects, capabilities, components, targets, relations,
and tests validate. Publication writes immutable module, root, receipt, revision, and dependency
objects durably before replacing HEAD. Readers therefore observe the old complete revision or the
new complete revision.

History is a DAG. Ordinary publication has one parent. An accepted semantic merge has two unique,
canonically ordered parents. Initial import has no parent. Nonsemantic intent is bounded receipt
metadata and does not enter revision identity.

## Drafts

A draft is separate packed non-executable authority. It binds one repository, exact accepted base,
generation, ordered operations and preconditions, typed holes, closed conflicts, and optional
bounded intent. Draft mutation never changes HEAD. Draft validation uses the transaction validator
but publishes nothing. A draft with holes or conflicts cannot publish. Rebase is explicit and
updates the draft only after validation against the named base. Drop cannot affect accepted
authority.

Draft files are local operational state by default and are excluded from Git. Verified backups
include retained drafts. Draft IDs never parse as revision IDs.

## Semantic transactions

The public transaction contract is the sole normal writer. A request contains:

- transaction and graph contract identities;
- repository identity and exact base revision;
- optional idempotency key;
- ordered high-level operations and exact preconditions;
- operation, work, and affected-owner budgets; and
- optional nonsemantic intent.

The implemented operation set covers package metadata and exact dependencies; module create,
rename, and delete; declaration create, replace, rename, move, clone, restore, and delete; record
field and variant-case evolution; interface-operation evolution; signature, body, expression,
reference, binding, and test-expectation changes; and target create/delete. `CreateDeclaration`
constructs records, variants, interfaces, constants, pure/task functions, components, and tests
without product-specific native policy. `apply` executes an ordered batch atomically.

Plan calculates the exact candidate and impact without validation publication. Validate runs full
canonicalization and the independent semantic validator but publishes nothing. Apply repeats the
same deterministic operation semantics and then performs exact compare-and-publish. No raw table,
arena, or byte-offset edit is public.

Preconditions currently cover root digest, owner existence/absence, and owner name. An idempotency
key is scoped to repository history: exact replay returns the retained receipt; reuse for different
transaction bytes is a precondition failure.

## Publication outcomes

The closed transaction outcomes are:

- `accepted_change`: one revision and receipt become visible;
- `semantic_no_change`: nothing is published;
- `replayed`: an exact idempotent receipt is returned without publication;
- `stale_base`: the requested base is not current;
- `precondition_failed`: an exact predicate or idempotency condition failed;
- `foreign_identity`: repository or owner domain does not match;
- `invalid_graph`: the candidate is not accepted meaning; and
- `resource_exhausted`: a declared or hard budget was exhausted.

Malformed protocol, corrupt authority, and infrastructure failures are diagnostics outside the
semantic result. Validation, plan, query, stale input, rejection, and no-change publish nothing.
Publication visibility that cannot be reconciled is an infrastructure failure; the caller must
read HEAD and retained receipts before retrying.

## Bounds

Current hard maxima include 16 MiB root payload, 64 MiB module payload, 100,000 modules per root,
100,000 declarations per module, 2,000,000 retained identities or tombstones in the relevant
container, expression depth 256, 4,096 dependencies, 65,536 targets, 10,000 operations per
transaction, 10,000,000 transaction work, and 100,000 affected owners. Smaller request budgets are
mandatory and are part of deterministic behavior. Checked arithmetic and pre-allocation bounds
apply before decoding growth.

## Change of graph contract

There is no compatibility edition or fallback reader. A future graph contract change requires an
explicit new-authority reconstruction or exact one-time cutover, complete consumer migration,
predecessor rejection, and deletion of the prior current reader. Git history may retain old bytes;
the executable current tree does not interpret them.
