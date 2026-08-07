# Documentation authority

**Role: active authority policy.** This document assigns ownership by claim dimension. It does not
define language semantics, report implementation status, describe architecture, or schedule work.

## Authority by dimension

| Claim | Owning artifact |
| --- | --- |
| Current task and engineering procedure | Current task instructions, then root [`AGENTS.md`](../AGENTS.md) |
| Intended language semantics | Accepted normative [`spec/language.md`](spec/language.md) |
| Intended semantic-workspace behavior | Accepted normative/target [`spec/workspace.md`](spec/workspace.md) |
| Behavior in this checkout | Executable code, tests, CLI definitions, schemas, and manifests; [`status.md`](status.md) is the concise summary |
| Workspace membership and dependency edges | Cargo manifests and `cargo metadata`; [`architecture.md`](architecture.md) explains responsibilities only |
| Performance evidence | Reproducible harnesses, workload definitions, protocol, and compact recorded results in [`performance.md`](performance.md) |
| Durable rationale | A sparse accepted decision in `docs/decisions/`, when one exists |
| Ordering and intent | [`roadmap.md`](roadmap.md) |
| History | Git history |

A specification may intentionally lead the implementation. That difference is an implementation gap
to record in `status.md`; it does not silently amend the specification. An architecture decision may
constrain implementation but does not silently amend externally visible semantics.

## Conflict handling

When claims conflict:

1. identify the dimension of the claim;
2. inspect the artifact that owns that dimension;
3. inspect executable evidence for claims about the checkout;
4. update or delete stale material in the same change; and
5. record a decision only when a durable, non-obvious choice would otherwise be repeatedly
   rediscovered.

Do not introduce global revisions, prose digests, fact registries, closure graphs, or copied Cargo,
CLI, schema, operation, or diagnostic tables to manufacture a single authority order. Git owns
superseded prose.
