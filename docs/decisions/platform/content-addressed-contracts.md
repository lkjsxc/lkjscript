# Content-Addressed Current Contracts

## Purpose

Define one continuously canonical lkjscript language and platform and the exact
identity used wherever independently produced or retained bytes cross an
authority boundary.

## Status

**Accepted contract.** The one-language rule is binding before implementation
migration. Contract-registry, protocol, package, artifact, and cache behavior
becomes Current only with its named implementation evidence.

## One Living Language

lkjscript has one Current language, source grammar, Semantic Source, agent
protocol, resource contract, runtime-call surface, native artifact contract,
and package contract. Source never selects a generation. Current compiler and
protocol implementations do not retain old parsers, semantic branches,
translation tables, accepted-old-digest lists, or compatibility modes.

Language evolution is atomic canonical replacement:

1. update this authority and the affected semantic authority;
2. update complete canonical descriptors and all producers and consumers;
3. rewrite the complete canonical corpus and accepted fixtures;
4. differentially verify behavior where preservation is required;
5. remove temporary old parsing, lowering, rewriting, and schema code; and
6. merge only one canonical state.

Git commits and immutable evidence preserve history. An old program uses the
exact historical compiler that accepted it; the Current compiler is not a
museum of former grammars.

## Contract Identity

A `ContractDigest` is the full SHA-256 digest of canonical descriptor bytes.
It answers whether exact structures, operations, semantics, ownership,
capabilities, resource units, and ABI facts agree. It is not a semantic version
and never selects among coexisting implementations.

Every descriptor contains:

- a stable contract name and hashing-algorithm identity;
- stable item IDs and item kinds;
- field, variant, and operation facts;
- required/optional and closed/open facts;
- types, effects, ownership, capabilities, and failure channels;
- resource categories and units where relevant;
- serialized section or runtime-call layout where relevant;
- canonical ordering rules; and
- exact dependency contract digests.

Encoding uses explicit domain tags, big-endian checked lengths, and complete
framing. Unordered descriptor collections sort by stable identity before
encoding. Semantically ordered collections preserve order. Rust source order,
hash-map order, clocks, process IDs, absolute paths, and memory addresses never
participate.

## Registered Domains

The registry is closed for each build and includes at least:

- language semantics and canonical source grammar;
- Semantic Source nodes and agent requests, responses, and diagnostics;
- resource categories and profile ceiling sets;
- repository graph, capsule manifests, and agent work state;
- serialized HIR, verified SSA, bytecode, runtime calls, native layouts, and
  metrics where those bytes cross an independent boundary;
- package manifests, lock graphs, module interfaces, and future components.

Pure in-process Rust types need no serialized contract ceremony. A serialized
or cached consumer accepts exactly the current full digest. Mismatch reports
contract name, expected and actual digests, producer and consumer identities,
and update or rebuild guidance.

## Discovery

`lkjscript describe`, `lkjscript describe --json`, and
`lkjscript semantic describe` are bootstrap operations. They report compiler
build identity, full Current contract digests, language and Semantic Source
operations, engines and targets, resource profiles, package capabilities, and
unsupported surfaces. Other agent requests require the exact digest returned
by discovery. Discovery does not negotiate historical contracts.

## Content Identities

Distinct hashes have distinct purposes:

- source entity identity preserves editing relationships and presentation;
- definition hash covers normalized resolved semantics and dependency hashes;
- interface hash covers exported names, types, effects, capabilities,
  ownership, errors, and deliberate layout promises;
- package content hash covers canonical manifest, module graph, definitions,
  interface, dependencies, capabilities, and declared build inputs; and
- artifact hash covers verified program form, all relevant contract digests,
  target and CPU facts, backend/configuration, resource facts, and build
  identity.

Names are excluded only where a stable item identity deliberately separates a
presentation rename from semantic identity. Full digests are authoritative;
display prefixes never authorize bytes.

## Research Disposition

Adopt from Unison normalized definition hashing, hashed dependencies,
canonical recursive groups, and names as separate metadata. Reject replacing
Git projection and Semantic Source with a codebase-manager database.

Adopt from Nix immutable content-addressed outputs, complete input graphs,
deterministic build descriptions, deduplication, and early cutoff. Reject
historical compatibility formats, ambient build inputs, and network resolution
inside locked builds.

Adopt from Cap'n Proto stable item/member identity and symbolic rename separate
from structural identity. Reject backward-compatible wire evolution in Current
protocols; lkjscript requires one exact descriptor.

Adopt from the WebAssembly Component Model explicit packages, interfaces,
world-like import/export boundaries, typed resources, and interface-derived
ABI facts. WIT does not define lkjscript semantics or package resolution.

Adopt language-based agent control: generated and hand-authored actions pass the
same type, ownership, capability, policy, and resource checks. Reject runtime
prompt interpretation as authority.

Adopt affine values, inferred exact borrows, uniqueness/locality facts,
deterministic drop, and selective immutable-reference fallback as staged memory
mechanisms. Defer the complete future mode and region system; never use
reference counting for external affine resources.

Adopt typed query identity, dependency recording, structural invalidation, and
private-body early cutoff from incremental-computation work. Defer a full query
engine until definition, interface, package, and artifact identities are
Current.

## Change Gate

A contract-changing commit updates descriptor authority, producers, consumers,
corpus, packages, locks, artifact invalidation, Semantic Source legal actions,
documentation, and status together; removes obsolete Current implementation;
preserves historical evidence; and passes focused plus canonical gates. It
never creates a numbered schema.

## Rejected

Numbered lkjscript editions or semantic generations, numeric protocol selectors,
old-digest acceptance, aliases, fallback conversion, truncated-digest authority,
source compatibility modes, and generation labels disguised as dates, “next”,
or “modern” are rejected.
