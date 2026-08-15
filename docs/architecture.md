# Architecture

## Current flow and authority

```text
strict JSON CLI projection (optional)
    -> typed protocol-v3 frame over private Unix socket
    -> synchronous lkjscriptd
    -> one DurableWorkspace writer per workspace
    -> staged typed transaction over immutable Snapshot
    -> full deterministic validation and derived diff
    -> preflighted compact receipt + artifact + LKJHEAD3
    -> durable immutable revision, then in-memory publication
    -> revision-bound scan query or direct SPG-to-Core-IR lowering
    -> Core IR verifier -> interpreter -> typed response
```

The daemon is the only live graph writer. `graph.rs` owns immutable snapshots and retained history;
`schema.rs` owns closed node contracts and static operation descriptors; `transaction.rs` owns
staging and compact receipts; `validate.rs` owns graph acceptance; `diff.rs` owns deterministic
change facts/digests; `artifact.rs` owns canonical semantic bytes; `persistence.rs` owns durable
publication; `protocol.rs` owns version-3 IPC types/framing; `query.rs` owns derived scan queries and
repair-context composition; `machine.rs` owns strict bounded JSON projection and executable schema
description; `compile.rs` iteratively discovers direct-call closures and lowers structured regions;
private `core_ir.rs` owns dense multi-function CFG contracts and independent verification;
`interpret.rs` owns the one explicit-frame runtime route. Generated CFG blocks thread the complete
visible semantic environment in `ValueRef` derived order: reference variant first, then canonical
workspace/node identity, then operation output index. This is a private deterministic lowering
choice, not a public identity or serialized contract.

Static operation descriptors are the shared fact owner for arity, operand/result rules, use modes,
literal fields, completeness, and termination. Validators, queries, codecs, lowering, and machine
description consume those facts without runtime registration or heap allocation for lookup. There
is no second graph, type authority, function body, evaluator, mutable client workspace, or persisted
compiler representation.

Queries are pure over one retained immutable revision (or two exact revisions for diff). Incoming
uses/references, dependencies, visible values, legal constructors, blockers, and repair contexts use
full deterministic scans. Legal-constructor and owner-chain pagination streams and counts candidates
while retaining only the requested page; repair context retains at most its per-category budget while
counting the exact total. Repair context is a bounded composition of typed facts, not prose or model
ranking. There is deliberately no reverse-reference index, query engine, or cache; full scans
remain both implementation and oracle until representative repeated cost justifies a narrow index.

The generic CLI is not another service. It strictly decodes one bounded JSON envelope, converts to
the same closed Rust request, sends one private binary IPC request, and strictly encodes one typed
response. JSON never becomes semantic state. Local `schema` and daemon `DescribeSchema` derive from
executable descriptors and stable enums rather than a separately maintained schema file.

## Durability and bounded acknowledgement

A workspace retains immutable `revisions/REVISION.lkjscript` files and one compact non-semantic
`LKJHEAD3`. HEAD is independently capped at 16 KiB and stores head revision/hash plus at most one
compact keyed fingerprint/receipt. It stores no full semantic diff or full allocation map; unkeyed
commits preserve an existing keyed replay record. Any non-`LKJHEAD3` bytes, including prior
`LKJHEAD1` and `LKJHEAD2` formats, reject without a compatibility reader.

Commit preflights exact artifact, HEAD, and protocol response bytes before publication. It writes
and flushes a revision temporary file, renames and syncs it, writes/flushes a HEAD temporary file,
atomically replaces and syncs HEAD, then publishes the in-memory `Arc<Snapshot>` and acknowledges.
Failures before authoritative HEAD leave the prior state authoritative. If publication and rollback
make outcome unknowable, the daemon reports `CommitOutcomeUnknown` and stops.

Validate-only follows the same semantic preparation and byte preflight but writes and publishes
nothing. Restart bounds files before reading, decodes every contiguous retained artifact, validates
history, checks HEAD checksum/hash, and recomputes compact receipt facts from retained snapshots.
Corrupt or ambiguous durable state is rejected, not repaired heuristically.

## Trust boundaries

- **JSON stdin/stdout:** 8 MiB input, bounded nesting, strict fields/variants/canonical IDs/trailing
  policy; streaming 32 MiB output cap and bounded boundary errors;
- **binary IPC:** private socket permissions/peer filesystem boundary, checked frame/counts/tags,
  request correlation, request-side EOF before dispatch, and response-side EOF before acceptance;
- **artifact and HEAD bytes:** separate bounded canonical decoders, hashes/checksums, graph/history
  validation, strict trailing-byte policy;
- **AI proposals:** only closed typed requests reach deterministic validators;
- **runtime:** verified Core IR only, bounded invocation arguments, positive bounded fuel and frame
  policy, and one non-recursive interpreter loop; fuel is charged once per executed instruction and
  once per terminator transfer, while both frame count and aggregate live value-slot capacity are
  checked before entry/call allocation and released on return; no native
  code, ambient host capability, or foreign boundary.

User-scalable graph traversals use loops and explicit work collections rather than unbounded native
recursion. Operational page/frame/context limits protect boundaries and do not constrain semantic
program size.

## Deliberate restraint

The measured bootstrap retains `BTreeMap`, vectors, full snapshot clones, full semantic
recomputation/diff materialization, and full artifact rewrites. It has no database, journal, async
runtime, generic graph framework, runtime schema registry, reverse index, cache, source projection,
native backend, runtime cell, plugin mechanism, or remote service. A replacement requires a current
consumer, measurements, one preserved authority path, and evidence that supports its added cost.
