# Current architecture

This document maps the current implementation. Normative behavior is owned by `docs/spec/` and the
executable validators; application policy is owned by maintained `.lkj` modules.

```text
canonical source + exact package descriptor
        │
        ▼
bounded parser ──► typed semantic model ──► deterministic component artifact
                                                 │
                                                 ▼
                                      prepared bytecode + AST oracle
                                                 │
                               ┌─────────────────┴─────────────────┐
                               ▼                                   ▼
                    in-memory test dispatch             resident HTTP/worker
                               │                                   │
                               └──────── typed requirements ───────┘
                                                 │
                                                 ▼
                                  deployment grants and adapters
```

| Layer | Current owner | Owns | Does not own |
|---|---|---|---|
| Authored authority | `.lkj`, `lkjscript.package.json`, `.lkjscript/source-v1` | module bytes, exact package metadata, immutable accepted revisions | derived semantic facts, compiled code, grants |
| Parser and semantics | `syntax.rs`, `language.rs`, `semantic.rs`, `intrinsic_contract.rs` | spans, declarations, types, effects, owner identity, closed extern validation | source publication, paths as identity, host handles |
| Artifact and preparation | `artifact.rs`, `execution/` | exact package closure, component/port preparation, bytecode, independent AST oracle | credentials, listeners, domain persistence |
| Component model | authored `component`, requirement, port, and target declarations | typed entry ports and required operations/limits | runner mechanics and deployment bindings |
| Runtime | `runtime.rs`, `http.rs`, `worker.rs`, `stream.rs` | bounded admission, task identity, execution, cancellation, shutdown, transport adaptation | route policy, SQL choice, object keys, worker meaning |
| Capabilities | `execution/capability.rs` plus generic adapters | requirement/grant equality, operation accounting, task resources, generic external mechanics | application authorization or cross-authority atomicity |
| Deployment | `deployment.rs` and deployment JSON | concrete adapters, environment secret bindings, limits, listener and topology | program semantic identity or application policy |
| Application | `applications/lkjournal/src/*.lkj` | routes, schemas, SQL, auth policy, rendering, object and job transitions | sockets, database driver, OS randomness, secret acquisition |
| Development tools | `cli.rs`, `workspace.rs`, `tools/check` | discovery, exact apply, build/test/inspect, compact verification evidence | a second editable program truth |

## Authority and identity

Canonical textual modules won the authored-authority comparison. They give local diffs, comments,
source spans, task-sized loading, ordinary merge behavior, and independent reconstruction without a
private builder. The typed meaning graph remains a derived semantic representation. A canonical
structured AST was rejected as a maintained form because it retained graph-sized edit payloads;
dual source/graph authority was rejected outright.

Package identity is an explicit nonzero 32-character lowercase hexadecimal value. A declaration
owner is `(package_id, module_name, declaration_name)`. File paths are locators only. Renaming or
moving a module or declaration replaces that semantic owner; the current language has no declared
continuity operation. A package dependency binds exact package identity, semantic revision digest,
and artifact digest. Builds never resolve ambient directories or mutable versions.

The source store deduplicates immutable source and dependency objects and commits a revision by one
atomic `HEAD.json` replacement after all objects, manifest, record, and revision index are durable.
Unreachable objects from an interrupted pre-HEAD publication are harmless and are not current
authority. Validation and no-change do not publish.

## Language and execution

The core is expression-oriented and deterministic. Pure functions cannot perform capabilities.
Task functions declare capability aliases, and semantic closure analysis ensures every direct and
indirect operation is present in the component requirement. Transactions are lexical task scopes;
their native handles are never language values. Streams, secrets, functions, and all runtime
resource values are non-durable.

Preparation lowers functions to compact bytecode. The production VM and the separate AST evaluator
share types and values but do not share instruction dispatch; package tests compare their results,
traps, and instruction observations. Compiler indexes and bytecode offsets are disposable and never
semantic identity.

One `ResidentDeployment` is used by live HTTP, workers, and in-memory component tests. A bounded
admission semaphore rejects overload, a worker semaphore bounds active execution, every task has a
runtime identity and cooperative control, and shutdown stops admission before drain/cancel/adapter
cleanup. The language cannot observe worker count or scheduler order.

## Capability and deployment boundary

An artifact contains the exact interface owner, operation set, and semantic limits required by each
component. A deployment grant adds adapter kind, sharing domain, descriptor digest, authority
revision, concrete limits, and an opaque adapter. Preparation rejects absent, duplicate, foreign,
under-scoped, or over-limit grants before work is admitted.

Expected domain results are values. Execution failures are closed as trap, capability,
possible-visibility, resource, cancelled, or infrastructure. Possibly visible operations are never
silently made retryable. Cancellation and deadlines are runtime observations, not deterministic
instruction fuel.

The maintained service binds configuration, secret verification, wall clock, secure randomness,
UUID generation, Argon2 password hashing, PostgreSQL, byte streams, named object storage, and a
durable queue. Test adapters are deterministic and disjoint from live PostgreSQL, local object, and
OS entropy implementations.

## Product boundary

`lkjournal` is a consumer, not architecture. A test scans generic native sources for its product
vocabulary. Its route strings, tables, SQL statements, actor ownership, session expiry, HTML, object
key policy, and queue behavior occur only in authored application modules. The generic binary
locates an artifact and deployment descriptor, validates them, and invokes the selected port.

The old typed/bytes/stateful/interactive profile worlds, graph projects, application contract 8,
runtime contract 2, and product-specific native binaries were removed. No current decoder or runner
falls back to them.
