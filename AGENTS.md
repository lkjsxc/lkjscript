# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may add rules for a genuine ownership boundary, but it may not weaken any
applicable rule in this file.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, examples, benchmark labels, commit messages, revision metadata,
evidence, and handoffs.

## Mission

Build `lkjscript` into a general-purpose, meaning-oriented programming language and application
platform. It must support substantially different classes of useful software without requiring
application-specific Rust to own ordinary application policy.

The language, package model, compiler, runtime, capability system, application model, development
tools, and deployment boundaries must form one coherent system. No current application, including
`lkjedit`, `lkjwork`, or a future `kjxlkj`-like service, is the architecture.

Existing applications are forcing functions and regression consumers. They may reveal missing
general capabilities, but product vocabulary must not leak into universal contracts unless the
concept is independently general.

Preserve current mechanisms only when they remain the strongest long-term design after comparison.
The semantic graph, source-free authoring, immutable project history, closed application profiles,
foreground topology, one-command suspension, current artifact families, and current native shells
are all provisional.

Optimize for the best coherent system over a long horizon, not for compatibility, recent effort,
short-term diff size, or the survival of a successful past campaign.

## Primary objectives

1. Make ordinary application policy expressible in lkjscript across interactive, service, batch,
   worker, and durable workflow software.
2. Keep host authority explicit through typed capability requirements and deployment grants.
3. Make common application construction natural, modular, reusable, inspectable, testable, and
   efficient for both coding agents and humans.
4. Retain one exact semantic and publication owner for every authority domain.
5. Reach asymptotically sound execution, storage, compilation, and interaction performance while
   preserving independent correctness oracles.
6. Minimize repeated model context, command output, correction depth, and unnecessary verification
   work without weakening evidence.
7. Delete superseded formats, profiles, adapters, applications, builders, aliases, and documentation
   after direct cutover.

## General-purpose success criterion

A platform change is not general merely because it removes an editor-specific name. It is general
only when its semantics and ownership remain natural for materially different consumers.

At minimum, major application-model decisions must be evaluated against three distinct shapes: an
interactive foreground application, a long-running request-driven service, and a batch or durable
worker workflow.

- Interactive evidence may come from `lkjedit`, but editor layout, tabs, buffers, Vim modes, and
  terminal cells remain product concepts.
- Durable command and query evidence may come from `lkjwork`, but task, priority, label, and
  dependency vocabulary remains product meaning.
- Service evidence may be informed by `kjxlkj`, but users, notes, media, routes, PostgreSQL, S3, and
  transcription remain consumer requirements rather than universal language primitives.
- A capability may have one initial consumer only when its external contract is already
  independently general, its boundary is narrow, and no product-specific policy enters the adapter.
- A universal abstraction that cannot explain its second plausible consumer is presumptively
  premature.

## What lkjscript is not

- It is not an editor construction kit.
- It is not a work-ledger framework.
- It is not a `kjxlkj` implementation hidden in the runtime.
- It is not a conventional language recreated feature by feature without a coherent semantic model.
- It is not a graph database whose public purpose is to expose graph mechanics.
- It is not a Rust application with semantic policy sprinkled into artifacts.
- It is not a collection of unrelated invocation profiles and host adapters.
- It is not a compatibility museum.
- It is not a benchmark suite with no maintained ordinary applications.
- It is not a sandbox merely because application semantics lack raw pointers.

## Authority and precedence

1. The active user task.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative specifications under `docs/spec/`.
5. Executable validators, focused invariant tests, and public black-box contracts.
6. The selected maintained authored program authority and its immutable accepted history.
7. Generated descriptions mechanically derived from one executable owner.
8. `docs/status.md`.
9. `docs/architecture.md`.
10. Structured evidence and `docs/performance.md`.
11. `docs/roadmap.md`.
12. `README.md` and application READMEs.
13. Historical prompts, commits, branches, pull requests, issues, discussions, comments, and
    remembered plans.

A newer verified checkout outranks stale facts in a prompt. A campaign prompt defines objectives,
hypotheses, gates, and authorization for one campaign; it does not become permanent semantic
authority.

When behavior changes, update the owning specification, implementation, executable oracle, current
status, and affected user documentation in the same verified cutover.

## Autonomy and engineering responsibility

Resolve ordinary engineering choices from the checkout, current requirements, complete workflows,
bounded prototypes, and measured evidence. Do not ask the user to choose between implementation
details that the repository can decide.

Do not stop at a report when a safe dependency-closed implementation is authorized and feasible. Do
not scatter speculative partial architecture across the active tree.

Large changes are permitted. A large rewrite is justified only when it converges the repository on
one stronger model and carries its consumers, tests, documentation, and deletion work through
completion.

Preserve independently valuable invariants even when replacing their implementation. Explicit
authority, hostile decoding, deterministic semantics, bounded work, publication classification, and
independent reconstruction are examples of invariants rather than historical code.

## Repository safety

Before editing, inspect the actual checkout and every applicable instruction file.

```sh
git status --short
git status --branch --short
git branch --show-current
git rev-parse HEAD
git log -8 --oneline
git remote -v
git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || true
find .. -name AGENTS.md -print
```

- Preserve unrelated modified and untracked work.
- Do not reset, clean, overwrite, amend, rebase, merge, force-push, publish a release, delete remote
  state, or alter unrelated paths without exact authorization.
- Repository permissions are not authorization.
- When staging is authorized, stage only explicit in-scope paths. Never use `git add .`, `git add
  -A`, or `git add --all`.
- Inspect staged and unstaged diffs before every commit.
- Use coherent commits and verify local and remote refs after any authorized push.
- Never commit credentials, secrets, private transcripts, hidden model reasoning, raw provider
  events, personal data, unrelated user files, or unlicensed corpora.
- Keep scratch state, downloaded research, destructive experiments, and losing prototypes outside
  the repository unless a retained artifact has a named consumer.
- Report unavailable tools, failed verification, unknown publication, partial completion, and every
  requested action not performed.

## Backward compatibility

Backward compatibility is absent unless the active user explicitly requires it. Do not spend design
budget preserving old source, semantic repositories, artifacts, protocols, commands, profiles,
applications, instances, deployment layouts, or generated views.

- Prefer one direct current path over dual readers, dual writers, editions, aliases, migrations, or
  fallback.
- Reject superseded current formats exactly and test the rejection.
- Delete old normal paths after the replacement is verified.
- Historical immutable data may retain the minimum decoder needed to inspect its own history only
  when that route cannot create or execute a predecessor current artifact.
- History reconstruction is not compatibility.
- Incompatible-change freedom is not permission to leave the repository between architectures.

## Zero-based design

Treat every substantial mechanism as provisional. Re-evaluate assumptions when a new application
class, scale, performance target, authoring workflow, or security model invalidates the evidence
that selected the mechanism.

- Do not preserve a semantic graph as primary authority merely because the project is meaning-first.
- Do not replace the semantic graph merely because textual source is conventional.
- Do not preserve source-free authoring merely because it once reduced generator duplication.
- Do not introduce source files merely to obtain familiar syntax while retaining all old graph
  complexity underneath.
- Do not preserve closed application profiles if one component and capability model is clearer.
- Do not introduce a general effect system if explicit typed workflows remain simpler for all named
  consumers.
- Do not preserve a synchronous runtime when a service consumer requires bounded concurrency.
- Do not introduce an async runtime merely because a dependency uses futures internally.
- Do not preserve a pure interpreter as the only production tier when execution dominates complete
  workflows.
- Do not add a JIT before a simpler bytecode or specialized interpreter has been evaluated.

For every foundational choice, compare retention, replacement, simplification, and deletion. Record
the consumer, obligations, alternatives, evidence, selected design, direct-cutover deletions, and
reversal gate.

## Architecture layers

Keep ownership boundaries explicit. The exact layer names may change, but the final architecture
must make these responsibilities distinguishable and prevent circular authority.

| Layer | Owns | Must not own |
|---|---|---|
| Authored program authority | Modules, declarations, tests, package metadata, capability requirements, application composition | Formatting caches, compiled code, deployment grants |
| Semantic model | Types, scopes, identity, effect/capability typing, validated meaning | File paths, host handles, caches |
| Compiler and IR | Lowering, verification, specialization, diagnostics | Durable semantic identity unless explicitly specified |
| Runtime execution | Frames, values, tasks, resource accounting, scheduling policy | Application-specific business decisions |
| Application/component model | Typed ports, lifecycle, handlers, state transitions, declared requirements | OS authority and deployment secrets |
| Capability adapters | Generic external mechanics under exact grants | Hidden application policy |
| Deployment | Concrete endpoints, credentials, resource placement, process topology, quotas | Program semantic identity |
| Development tools | Discovery, context, change, validation, history, build, test, run, diagnosis | A second editable program authority |

Dependency direction must remain legible. A lower layer may expose a general mechanism; it must not
import a higher-level product concept to make one application pass.

## Authored representation and semantic authority

Each accepted project revision has exactly one maintained authored authority. It may be canonical
text modules, a structured syntax tree, a typed semantic graph, or another deliberately selected
form. The selection is architectural, not ideological.

- The authored form must support local edits, modular loading, exact diagnostics, deterministic
  builds, reusable packages, reviewable history, and bounded agent context.
- A derived typed graph or IR may remain central to validation and execution without being the only
  editable representation.
- A source-like document is not automatically authority. State whether it is authoritative, a
  proposal, or a rendering.
- Formatting-only changes must have an exact policy: accepted source history, semantic no-change, or
  derived-view no-change.
- Do not maintain source and graph as independently editable truths.
- Do not require a custom Python, shell, Rust, macro, build script, generated source, or opaque
  fixture to reconstruct maintained program meaning.
- Lossless round-trip claims require property tests over comments, names, modules, declarations, and
  every syntax form that is semantically retained.
- If semantic identity survives source movement or renaming, define the exact continuity rule rather
  than inferring identity from position.
- If semantic identity does not survive an edit, do not fabricate continuity merely to produce a
  pleasant diff.
- Keep the simplest complete import/export and backup oracle for the selected authority.

Evaluate authoring on complete tasks: orientation, locating owners, adding a type, changing a
handler, refactoring across modules, resolving an error, reviewing a diff, merging independent work,
and reproducing a build from a fresh checkout.

## Semantic identity

Assign durable identity only for a concrete continuity, reference, sharing, history, repair, import,
export, deployment, or operational consumer. Names, positions, paths, hashes, indexes, process IDs,
addresses, and rendered coordinates are not automatically identity.

- Every identity domain has one owner, canonical encoding, equality rule, allocation rule, retention
  rule, deletion rule, nonreuse policy, and diagnostic spelling.
- Reject foreign-domain values even when bytes and display names match.
- A digest establishes equality or integrity only under its exact domain. It does not imply
  provenance, authorization, freshness, or capability.
- Function-local and compiler-local identities must not leak into durable public contracts without a
  continuity consumer.
- Renaming metadata must not silently change identity when references are intended to survive.
- Structural equality must not silently become nominal equality.
- Deployment locators remain outside semantic identity.

## Modules, packages, and reuse

A general-purpose language needs reusable program structure. Packages and modules must be
first-class accepted meaning rather than conventions reconstructed by build scripts.

- Define module identity, package identity, namespace rules, visibility, imports, exports, cycles,
  initialization, and diagnostics.
- Dependencies bind exact immutable identities or content under one deterministic resolution
  contract.
- No accepted build resolves `latest`, mutable tags, ambient local directories, or undeclared
  network state.
- Package composition must not require application artifacts to embed unrelated development history.
- Separate semantic dependencies from deployment adapters and credentials.
- Support small task-scoped loading so an agent need not ingest a complete package graph.
- Refactoring across modules must preserve or deliberately replace semantic identities under exact
  rules.
- Generic reusable libraries must not require a native wrapper per application.
- Versioning is allowed only when it represents simultaneous exact packages, not compatibility
  editions inside one package.
- A package registry is optional infrastructure, not a prerequisite for local deterministic
  composition.

## Language design

Language features must make materially useful programs simpler, safer, and more reusable. A feature
must pay for its semantic, compiler, runtime, tooling, documentation, and agent-context cost.

- Prefer a small orthogonal core plus libraries over a large list of product-shaped primitives.
- Do not confuse minimality with forcing every common algorithm through verbose low-level graph
  construction.
- Provide exact data modeling for records, variants, sequences, optional values, errors, and the
  collection shapes required by ordinary applications.
- Evaluate maps, sets, iterators, folds, closures, type parameters, interfaces, traits, pattern
  matching, and builders by complete consumer workflows rather than convention.
- Avoid implicit coercion, ambient overload resolution, hidden allocation, and order-dependent
  inference.
- Make evaluation order, equality, ordering, overflow, indexing, Unicode, and serialization
  explicit.
- Keep representation unobservable unless a consumer requires a representation-level contract.
- User-controlled recursion and traversal must not consume unbounded native stack.
- Compilation errors must identify the authored owner and relevant exact context, not only dense IR
  indexes.
- Incomplete or hole-bearing programs may be accepted only under a clear build and execution rule.

## Types, errors, and effects

Separate expected domain outcomes, semantic traps, capability failures, resource exhaustion,
cancellation, corruption, and infrastructure failure. Do not flatten them into text or one generic
error.

- Expected application outcomes should be ordinary typed values.
- A trap represents a violated semantic operation contract and publishes no authority by itself.
- Capability failures use closed typed classes and preserve retryability and possible visibility.
- Cancellation is observable only when its boundary and state effects are specified.
- Timeout is an operational observation unless program semantics deliberately consume it.
- Effectful code must make required capabilities statically or structurally discoverable.
- A pure function must remain independent of deployment grants, time, randomness, scheduling, and
  external state.
- Do not encode every effect as an opaque byte command merely to preserve a pure core.
- Do not allow direct ambient host calls that bypass typed requirements, grants, resource
  accounting, and test adapters.
- If an effect system is introduced, define effect polymorphism, handling, sequencing, failure,
  cancellation, and serialization before broad use.

## Capability model

External authority is represented by application requirements and deployment grants. Requirements
describe what a component may ask for; grants bind that requirement to concrete authority and
limits.

- Applications never embed credentials, live handles, deployment paths, sockets, file descriptors,
  database pools, or cloud clients as semantic values.
- A requirement identifies one exact interface contract and operation set.
- A grant identifies adapter kind, sharing domain, descriptor, limits, authority revision, and
  lifecycle.
- Adapters expose generic mechanics and may not invent application state, policy, ordering, or
  responses.
- Capabilities are least-authority and operation-scoped where practical.
- Every non-idempotent operation distinguishes known failure, known success, possible visibility,
  and reconciliation.
- Capabilities are testable through deterministic fakes that are disjoint from production
  implementation.
- Capability composition must not create hidden cross-authority atomicity.
- A capability contract states whether values, handles, streams, transactions, and tasks may cross a
  call, suspension, thread, process, restart, or durable boundary.
- The runtime must reject an unsatisfied requirement before accepting work that depends on it.

## Live resources and lifetimes

Files, sockets, request bodies, response streams, database transactions, object reads, processes,
timers, terminal sessions, and similar live resources need explicit lifetime semantics.

- Define acquisition, ownership, borrowing or transfer, use, close, cancellation, timeout, panic,
  process exit, and cleanup.
- Do not expose raw native handles to application semantics.
- A live resource may not enter durable state unless represented by a separate serializable locator
  and revalidation contract.
- Finalization must not depend on nondeterministic garbage collection.
- Lexical scope, explicit region, linear capability, affine handle, or runtime-owned task scope are
  all candidates; select one coherent model.
- Failure during cleanup is reported under an exact precedence policy.
- Resource counts and retained bytes are bounded independently from instruction fuel.
- A leaked application reference must not silently keep unbounded host authority alive.

## Unified application and component model

Prefer one component model with typed entry ports and declared capabilities over a growing list of
mutually special application profiles. CLI, service, interactive, batch, test, and worker behavior
should be runners or compositions of shared semantics where possible.

- A component declares exports, imports, state ownership, capability requirements, lifecycle hooks,
  and resource policy.
- Typed handlers receive explicit inputs and return typed outputs or effectful tasks under one
  specified execution model.
- Pure, ephemeral stateful, durable stateful, request-driven, event-driven, streaming, and scheduled
  work must be expressible without product-shaped native code.
- A runner adapts external transport and lifecycle; it does not own hidden domain policy.
- Application composition resolves exact component interfaces before deployment.
- No runner may inspect private application state to decide business behavior.
- Profiles may remain as derived packaging conveniences only when they do not define competing
  semantics.
- Headless and live execution use the same component owners and differ only in adapters and
  observations.
- Tests invoke the same prepared component contract used by production.
- A component artifact contains requirements, not grants or secrets.

## Execution topology

One-shot processes, foreground sessions, resident services, worker pools, and embedded execution are
operational topologies. They must not create separate semantic implementations.

- One exact runtime kernel owns admission, preparation, execution, capability routing, and
  observations.
- Resident service topology is required when complete service workflows cannot be implemented
  efficiently through one-shot execution.
- Per-authority mutation order remains explicit even when independent requests execute concurrently.
- Queues are bounded and observable; hidden unbounded queues are forbidden.
- Restart reconstructs semantic authority from durable owners and may discard only documented
  disposable acceleration.
- Graceful shutdown defines admission stop, in-flight work, cancellation, non-cancellable
  publication, drain bounds, cleanup, and exit status.
- A daemon or supervisor authenticates and authorizes requests under its deployment model.
- A process boundary is neither identity nor sandbox.
- Operational reuse, caches, and pools never become semantic authority.

## Interactive applications

Interactive applications remain important but do not define the universal model. Terminal events,
logical frames, tabs, buffers, and editor modes belong to an interactive library or application.

- Application meaning owns interaction policy; adapters own input decoding, safe output, device
  lifecycle, and resource grants.
- Ephemeral foreground state is distinct from durable application state.
- Rendering is a projection of application state and cannot roll back external publication.
- Input, action, render, and worker queues are bounded.
- Large files, many views, resize, search, and split layout must be measured end to end when
  `lkjedit` remains a maintained consumer.
- Do not elevate one editor optimization into a universal value representation without another
  consumer or a clear language-level obligation.
- Product defects are not evidence that the general architecture is wrong, but recurring workarounds
  are evidence that a boundary may be wrong.

## Service applications

A service application is a long-running component graph that accepts concurrent external requests,
uses explicit capabilities, and produces bounded responses or streams.

- Routing, request decoding, authentication context, handler selection, response construction, and
  middleware policy belong to application meaning or reusable lkjscript libraries.
- Socket acceptance, TLS, HTTP parsing, connection limits, kernel I/O, and generic protocol
  adaptation belong to trusted runtime adapters.
- Request bodies and responses have exact byte, item, time, and concurrency bounds.
- Streaming uses backpressure and cancellation; buffering the entire stream is not an implicit
  fallback.
- Per-request state, shared immutable state, durable state, and deployment resources are distinct.
- Graceful shutdown and readiness are explicit.
- A service runner cannot embed product routes, database schema, auth roles, or HTML layout.
- HTTP is an initial important adapter, not the definition of service semantics.

## Durable state and workflows

Durable application state, database state, object storage, queues, and workflow continuations are
separate authorities. Cross-authority atomicity exists only when one owner proves it.

- Every durable mutation has an exact base, idempotency rule, publication boundary, record or
  transaction outcome, and restart behavior.
- Pure queries publish nothing.
- A durable workflow records only serializable semantic continuation facts, never live handles or
  native stack state.
- Retry policy is explicit and class-specific.
- Possible external visibility blocks blind retry.
- Checkpoints, journals, snapshots, compaction, retention, deletion, garbage collection, backup, and
  restore have one owner.
- A durable queue defines claim, lease, heartbeat, completion, retry, cancellation, dead-letter,
  ordering, and duplicate behavior.
- Application undo, semantic project history, database rollback, and workflow compensation are not
  interchangeable.

## Concurrency, asynchronous work, and scheduling

Concurrency is an operational and semantic design area, not a library checkbox. Introduce it where
complete workloads require overlap, isolation, or throughput.

- Define task identity, parent scope, admission, scheduling, ordering, wakeup, cancellation,
  deadline, panic containment, result delivery, and shutdown.
- Structured concurrency is preferred over detached tasks without an owner.
- Deterministic pure computation remains deterministic regardless of scheduler choice.
- Observable races require an explicit semantic model; otherwise isolate mutation authorities.
- Concurrent capability calls preserve per-resource rules and do not reorder visible effects
  silently.
- Backpressure propagates rather than forming unbounded memory.
- A worker count is a deployment policy, not application semantic identity, unless the program
  deliberately observes it.
- Do not use wall-clock timeout as deterministic instruction fuel.
- Test serial, maximally interleaved, cancellation, overload, shutdown, and restart routes.

## Relational data and transactions

Database support should be a generic capability and library surface, not application-specific Rust
or a database hidden inside the language core.

- Separate query text or query representation, typed parameters, typed rows, transaction scope,
  connection pool, and deployment credentials.
- Validate parameter and result schemas before invoking application handlers.
- Define nullability, numeric ranges, text encoding, timestamps, bytes, arrays, JSON, row counts,
  and truncation.
- A transaction handle cannot escape its task scope or enter durable state.
- Commit success, rollback success, serialization failure, constraint failure, connection loss,
  timeout, cancellation, and unknown commit visibility are distinct.
- Migrations are exact deployment inputs with ordering, checksums, atomicity, and rollback policy.
- Do not build a universal ORM before query and schema consumers justify it.
- Use prepared or otherwise safe parameter binding; application text must not be interpolated into
  SQL by convention.
- Pool exhaustion is bounded admission, not an unbounded wait.

## Filesystem and object storage

Local filesystem and object storage are different capability families. Both require exact locators,
least authority, bounded transfer, and publication classification.

- Filesystem paths are validated components under an exact root or selected authority.
- Object keys are validated opaque or structured names under a bucket/prefix grant.
- Reads support bounded whole values and streaming where complete consumers require it.
- Writes define no-replace, expected-base replace, multipart, checksum, metadata, range, and
  reconciliation behavior as applicable.
- Delete, copy, rename, list, and version operations are separate grants.
- Symlink, hard-link, mount, case, normalization, and traversal behavior are explicit for local
  filesystems.
- S3 compatibility is an adapter contract, not an object-storage semantic assumption.
- Application policies such as media visibility, note ownership, or editor dirty state remain
  outside adapters.

## Network and HTTP

Network access is ambient authority unless mediated by exact grants. Server and client interfaces
must be closed, bounded, testable, and separate from application policy.

- An outbound grant restricts schemes, hosts, ports, methods, redirects, DNS behavior, TLS policy,
  byte limits, and deadlines.
- An inbound service grant restricts listeners, protocols, connection limits, request limits, and
  shutdown.
- HTTP contracts distinguish method, URI components, headers, body stream, trailers, status, and
  response stream.
- Header names and values have canonical validation and size limits.
- Redirects, retries, proxy behavior, compression, decompression, range requests, and caching are
  explicit.
- WebSocket, server-sent events, raw TCP, UDP, and WebRTC are separate capability contracts.
- Do not expose arbitrary sockets when a narrower application contract suffices.
- Network failures never become application-specific strings at the adapter boundary.

## Time, randomness, secrets, cryptography, and identity

These mechanisms are explicit capabilities or standard libraries with precise trust and
nondeterminism boundaries.

- Wall-clock time, monotonic time, calendar conversion, and scheduling deadlines are distinct.
- Time zones, UTC, leap seconds, precision, truncation, and serialization are specified.
- Secure randomness and deterministic test randomness are separate.
- Identifiers such as UUID or ULID define generation, parsing, canonical spelling, ordering, and
  collision assumptions.
- Secrets enter through deployment grants and are never serialized into application artifacts,
  semantic history, logs, or diagnostics.
- Secret values use redacted diagnostics and restricted conversions.
- Password hashing is a generic security adapter or library with parameter policy, upgrade path,
  verification behavior, and side-channel awareness.
- Hashing, MAC, signatures, encryption, and password hashing are distinct primitives.
- Authentication establishes an actor context; authorization remains application policy over typed
  identities and grants.
- Sessions define token generation, storage, rotation, expiry, revocation, CSRF, cookie, and replay
  behavior.

## Serialization and markup

Serialization is a typed boundary. Textual convenience must not create ambiguous or partially
validated values.

- JSON, URL encoding, forms, multipart, headers, binary codecs, and application artifacts have
  independent closed contracts.
- Unknown fields, duplicate fields, invalid Unicode, noncanonical encodings, excessive depth,
  excessive items, truncation, and trailing bytes reject according to each contract.
- Encoding is deterministic where bytes participate in identity or caching.
- Schema derivation must not silently expose private fields or unstable names.
- HTML construction escapes by default and distinguishes text, attribute, URL, CSS, and script
  contexts.
- Markdown parsing and sanitization are reusable libraries or generic adapters, not hard-coded
  product behavior.
- Raw trusted HTML is an explicit narrow type or capability, never an ordinary text convention.
- Streaming decoders bound retained state and report exact offsets.

## Build, package, and deployment

Build and deployment meaning must be explicit, deterministic, and separate. A package artifact
describes runnable meaning and requirements; deployment binds concrete resources.

- Build targets are accepted program meaning, not shell scripts or private callbacks.
- Builds select exact revisions and exact dependency identities.
- Artifacts exclude secrets, mutable coordinates, ambient paths, and live handles.
- A product target may package generic native runners and adapters without adding product policy to
  Rust.
- Deployment configuration has a typed schema, source precedence, redaction, validation, and startup
  failure policy.
- Environment variables are deployment input, not semantic identity.
- Health and readiness report runtime state without claiming external dependency success unless
  checked.
- Container images, system services, and native binaries are deployment projections and remain
  reproducible.
- Cross-platform claims require execution and acceptance on each claimed platform.

## Compiler and runtime

Keep one independently checkable execution route. Faster representations and tiers are derived until
their equivalence, invalidation, failure behavior, and operational value are proved.

- The reference interpreter or evaluator remains the semantic oracle unless deliberately replaced by
  an even simpler formalized route.
- A bytecode or register VM is preferred before JIT complexity when it closes the dominant execution
  cost.
- A JIT or native tier requires stable IR, deoptimization or fallback policy, code memory
  accounting, cache identity, security analysis, and differential tests.
- Compilation selects only reachable exact meaning unless whole-program work is required by the
  optimization.
- Incremental compilation and caches bind every semantic input, toolchain identity, target policy,
  and dependency.
- Cache miss, eviction, corruption, and process restart remain correct.
- Prepared program dispatch must not repeat validation, lowering, or allocation that can safely be
  reused within one exact application.
- Diagnostics map optimized execution back to authored owners.
- Do not inflate fuel to mask an algorithmic or dispatch defect.

## Memory and value representation

Choose memory management from the value model and complete workloads. Ownership, reference counting,
arenas, tracing collection, regions, persistent structures, and copying are implementation choices
unless promoted by semantics.

- Accepted semantics expose no unchecked memory access or manual deallocation.
- Representation identity, addresses, capacities, reference counts, generations, and sharing remain
  unobservable by default.
- Live resources must not depend on tracing finalization.
- Persistent values require exact logical and retained accounting so sharing cannot evade limits.
- Cyclic values require an explicit semantic and collector design; do not accidentally introduce
  cycles through host handles.
- Large text, bytes, maps, sequences, and trees need asymptotically sound operations for named
  workloads.
- Every optimized representation retains a canonical materialization or independent model.
- Process RSS observations are not semantic memory accounting.

## Resource governance

Instruction work, value size, retained memory, live resources, I/O bytes, queue depth, concurrency,
wall-clock deadlines, and durable storage are separate resource classes.

- Each limit names its unit, owner, source, reservation point, release point, peak accounting,
  retained accounting, rejection class, retryability, observability, and restart behavior.
- Check bounds before corresponding allocation or irreversible work.
- One scalar fuel number may bound deterministic instruction work; it must not pretend to govern
  every resource.
- Shared backing, caches, compiled code, streams, transactions, workers, and undo roots remain
  accounted.
- Overload rejects, queues within an exact bound, or sheds work under an explicit policy.
- Resource exhaustion never becomes success, skip, or semantic no-change.
- OS limits may strengthen containment but do not replace semantic and runtime admission.

## Long-term performance

Correctness is necessary and insufficient. Measure complete optimized workflows, identify dominant
stages, and prefer asymptotic correction before micro-optimization.

- Separate authoring, project open, parsing, validation, type checking, lowering, compilation,
  execution, host I/O, serialization, rendering, publication, and cleanup.
- Measure startup, steady-state, p50, p95, worst retained cases, throughput, tail latency, memory,
  storage, output bytes, and correction depth where applicable.
- Do not call a stage optimization a product speedup until the complete equal workload improves.
- Do not call warm measurements cold.
- Do not infer token cost from bytes or monetary cost without exact provider telemetry.
- Set reversal gates for every cache, index, scheduler, worker pool, execution tier, persistent
  representation, and storage format.
- Keep slow independent oracles available to verify optimized paths even when they are not the
  production default.

## Security and trust

Write a threat model before expanding trust. Memory safety, authority, authentication,
authorization, path safety, protocol safety, resource exhaustion, supply chain, secret handling, and
hostile code isolation are separate contracts.

- Treat authored programs, semantic repositories, artifacts, records, packages, configuration,
  paths, database rows, network bytes, terminal input, object metadata, logs, caches, and adapter
  outcomes as hostile input at their boundaries.
- Decode closed formats completely and reject truncation, trailing input, noncanonical form, foreign
  identity, and excess before allocation.
- A process, container, user account, private directory, capability token, or semantic project is
  not automatically a sandbox.
- Application code must receive only granted authority.
- Native dependencies expand the trusted computing base and require a complete-workflow benefit.
- Secrets and sensitive content are absent from default logs, test fixtures, evidence, and
  diagnostics.
- Multi-user service support requires actor authentication, authorization, tenancy isolation, audit
  policy, session security, and denial tests.
- Hostile native plugins or arbitrary child processes require a separately proved isolation model.
- No local unsafe Rust is permitted unless the active user explicitly authorizes a narrowly
  justified replacement with an isolated safe contract and independent tests.

## Public development interface

The public `lkjscript` CLI and any accepted source/module tools are the normal authoring interface.
Private constructors and custom generators cannot remain the only practical route.

- An agent can discover the exact project and selected revision from an ordinary directory.
- Orientation is compact and identifies commands for bounded expansion.
- Context queries return only task-relevant modules, declarations, types, effects, capabilities,
  tests, targets, and diagnostics.
- Mutations or source changes bind an exact base and validate through the same owner that publishes.
- Accepted changes create exact immutable history under the selected authority model.
- Stale state rejects without silent retry.
- Build, test, run, package, diagnose, backup, and recover use public commands.
- Friendly names resolve only when unambiguous.
- Session-local handles expire with their exact revision and never become durable identity.
- The CLI must not require users to supply discoverable internal IDs, artifact paths, schema
  digests, or store paths for ordinary work.

## Agent context and provider economy

Context budget is a correctness, latency, and cost constraint. Optimize complete task success rather
than raw byte minimization.

- Keep durable global principles in this file and volatile campaign detail in active prompts,
  status, evidence, or generated orientation.
- Provide task-scoped source and semantic context, on-demand expansion, exact digests,
  continuations, and explicit omissions.
- Stop broad discovery after exact owners are identified unless new evidence invalidates the map.
- Support known-digest and unchanged responses for every expensive stable projection where
  correctness permits.
- Prefer local modules and stable interfaces that reduce unrelated source loading.
- Measure request count, response bytes, files opened, source bytes, schema bytes, process count,
  repeated discovery, correction depth, elapsed time, and test invocations.
- Record provider token classes, cache classes, model identity, dated prices, and money only when
  directly exposed.
- Never infer tokens from bytes.
- A larger exact response is justified when it avoids a more expensive correction or rediscovery
  cycle.

## Quiet and exact command output

Passing commands default to compact output. Complete logs are retained separately under an exact
locator when they have diagnostic value.

- Do not print one line per passing test, case, target, file, module, or benchmark by default.
- A success summary includes command/profile, selected project or target, aggregate counts, elapsed
  time, exact result identity, and a log or receipt locator when retained.
- Failure output includes stable class, failing identifiers, bounded actionable excerpts, truncation
  facts, and complete log locators.
- Warnings, skipped work, cancellation, exhaustion, unavailable tools, indeterminate outcomes, and
  unknown visibility remain visible.
- Machine mode emits one strict versioned response without progress contamination.
- Full detail requires an explicit flag or reading the retained log.
- Do not achieve compactness by discarding the only complete diagnostic copy.
- A coding agent must be able to determine success without ingesting all passing output.

## Change-aware verification

Verification should avoid repeated irrelevant work while retaining a full independent gate.
Selection and caching are derived optimizations, not excuses to skip required evidence.

- Provide focused, changed, product, and full profiles when their ownership can be made exact.
- A changed profile derives affected gates from accepted dependency and ownership facts, not
  filename heuristics alone.
- A cached pass binds command, source inputs, generated inputs, toolchain, environment facts, target
  policy, dependencies, and prior result digest.
- Cache miss, stale metadata, corruption, and uncertainty rerun rather than assume success.
- The final campaign still runs the required full and fresh-checkout gates.
- Retain complete stdout and stderr with exact framing and bounded retention.
- Do not transform unavailable, skipped, exhausted, cancelled, or indeterminate work into pass.

## Testing

Tests are executable contracts. Select unit, property, differential, integration, acceptance, fault,
performance, and fresh-deployment tests according to the risk and authority boundary.

- Use independent reference models where semantics are substantial.
- Test canonical success, repeated success, no-change, stale state, malformed input, one-over
  limits, foreign identity, corruption, restart, interruption, possible visibility, reconciliation,
  output failure, cleanup, overload, and cancellation.
- Test every optimized path against its simple oracle.
- Test service concurrency, transaction isolation, worker claims, stream backpressure, shutdown, and
  multi-user denial where introduced.
- Test both deterministic fakes and production adapters through shared conformance suites.
- A skipped, unavailable, exhausted, cancelled, or indeterminate test is not a pass.
- Do not weaken an invariant test to fit an implementation.
- Run narrow checks first and complete gates before handoff.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

## Evidence and claims

Evidence is not semantic authority. Claims must be no stronger than reproduced observations and the
current checkout.

- Record exact commit, environment, toolchain, command, corpus, configuration, sample selection,
  units, output, and unavailable data.
- Distinguish warm and cold, one-shot and resident, elapsed and summed child time, logical
  accounting and RSS, throughput and latency, median and tail.
- Retain serious losing alternatives and reversal gates.
- Do not call a digest provenance or a process a sandbox.
- Do not call compile success cross-platform support.
- Do not claim API-cost savings without exact provider telemetry.
- Measured reductions in bytes, requests, processes, files opened, correction depth, storage, CPU
  time, or elapsed time may be claimed exactly.

## First-party applications

First-party applications prove the platform but do not define it. Every maintained application must
use public authoring, build, package, run, diagnosis, and recovery paths.

- Application policy belongs in lkjscript unless a generic native boundary is independently
  justified.
- No maintained application may require a custom graph builder, generated semantic source, private
  Rust business logic, or opaque binding constants.
- An application-specific native shell must be reduced to generic packaging and adapters or deleted.
- A useful application has user-valued behavior, black-box acceptance, representative workloads,
  exact error and recovery behavior, and fresh-checkout reproduction.
- When a new capability is added for one application, identify the general semantic obligation and
  test a second distinct shape or a generic adapter conformance contract.
- Delete synthetic applications and fixtures that no longer provide independent coverage.

## Current product caution

`lkjedit` is a secondary interactive forcing function, not the north star. Its existing semantic
ownership may contain valuable general patterns, but editor-specific state and UI policy must not
occupy universal language or runtime contracts.

Known product-quality concerns include confusing explorer interaction, number-based selection,
missing automatic scrolling, severe split and large-file slowdown, unclear tabs, weak status
information, indistinct visual boundaries, and incorrect append-at-line-end behavior. Treat these as
real acceptance debt when touching the product. Do not mistake the existence of an artifact or
passing narrow cases for a satisfactory editor.

`lkjwork` is a secondary durable-state and query consumer. Preserve its user value only when it
remains useful; do not preserve its current application profile or native bindings for
compatibility.

## Native code policy

Rust is the bootstrap and trusted adaptation language. Ordinary application behavior should not
require application-specific Rust.

- Generic runtime adapters may use Rust for OS, protocol, database, cryptography, terminal, network,
  storage, and process mechanics.
- A generic adapter exposes a stable typed capability contract and reusable conformance tests.
- Application-specific route tables, SQL policy, schema decisions, access rules, rendering
  decisions, job state machines, and domain responses belong in lkjscript.
- Performance-critical generic primitives may be native when semantics remain exact, the simple
  oracle is retained, and multiple consumers benefit.
- Every native exception records the blocker preventing lkjscript ownership and the condition for
  removal.
- Do not increase the percentage of lkjscript code as a vanity metric; minimize duplicate authority
  and product-specific native policy.

## Code ownership and repository structure

Organize code around stable owners and changed-together behavior. Large files and directories are
signals to inspect, not automatic violations.

- `docs/spec/` owns accepted observable contracts.
- `docs/architecture.md` owns current components, dependency direction, and trust boundaries.
- `docs/status.md` owns implemented reality and active format identities.
- `docs/performance.md` and structured evidence own observations.
- `docs/roadmap.md` owns unresolved evidence-gated decisions.
- `README.md` owns concise user and contributor orientation.
- Application READMEs own product-specific behavior.
- Campaign prompts are historical execution artifacts after completion.
- Keep one executable owner for every type, operation, format, interface, grant, limit, failure
  class, resource, target, record, and protocol field.
- Split files when it improves ownership clarity, bounded review, compile locality, test isolation,
  or agent context without duplicating invariants.
- Prefer the standard library and existing dependencies; every dependency must repay supply-chain
  and maintenance cost on a complete workflow.
- Git history is the archive. Delete stale active-tree copies.

## Decision record

Before retaining a substantial design, record all of the following.

- Named current consumers.
- Semantic and operational obligations.
- Reproduced baseline.
- Retention, replacement, simplification, and deletion alternatives.
- Expected benefit.
- Measured result.
- Security and trust impact.
- Resource and performance impact.
- Authoring and agent-context impact.
- Independent oracle.
- Direct-cutover deletions.
- Stop rule.
- Reversal condition.

## Change workflow

1. Inspect checkout, instructions, branch, commit, upstream, unrelated work, and tool availability.
2. Map current authored authority, semantic owners, runtime owners, format identities, native
   policy, and complete consumers.
3. Freeze complete authoring, interactive, service, worker, and durability baselines relevant to the
   task.
4. Create a compact campaign ledger and stop rereading the complete campaign prompt once the needed
   facts are extracted.
5. Prototype the highest-risk architecture choices in dependency-closed form.
6. Select one coherent design from evidence.
7. Update specifications before or with implementation so the target contract remains explicit.
8. Implement the complete vertical through authoring, semantics, compiler, runtime, capabilities,
   packaging, tools, tests, applications, and documentation.
9. Dogfood the public authoring route on a real maintained program change.
10. Cut over directly and delete superseded paths.
11. Run focused, property, differential, integration, product, performance, fault, restart,
    corruption, and full checks as applicable.
12. Reproduce from a fresh checkout and ordinary deployment path.
13. Inspect final diffs and exact staging scope.
14. Perform only authorized Git publication actions.
15. Leave a compact exact handoff with unsupported claims and remaining limits stated explicitly.

## Completion standard

A capability is complete only when its semantics, public authoring, compilation, runtime route,
capability grants, resource bounds, failures, tests, diagnostics, packaging, deployment,
documentation, maintained consumer, and fresh-checkout reproduction agree.

- No private builder or hidden product policy remains.
- No predecessor normal path remains.
- Passing output is compact and complete logs are locatable.
- Failure, cancellation, overload, interruption, restart, and possible visibility are tested.
- The complete workflow meets its explicit performance and resource gate.
- The independent oracle remains available.
- Known absences and reversal gates are documented.

Before finishing, report the exact starting and ending state, selected design, serious rejected
alternatives, changed authorities and formats, language and application-model changes, capability
contracts, native boundaries, application migrations, deletions, verification, performance, security
assumptions, agent-economy observations, Git actions, and every requested action not performed.

## Capability contract audit catalog

Use the following catalog as a permanent review aid. It does not require every capability to be
implemented immediately. It defines the minimum questions when a capability enters the system.

### 1. command-line process boundary

- Purpose: adapt arguments, standard input, standard output, standard error, exit status, and
  process lifecycle.
- Application or semantic owner: typed command arguments, command behavior, and domain output.
- Trusted native or deployment owner: OS argument acquisition, byte streams, signal delivery, and
  process exit.
- Operation surface to evaluate: read bounded arguments; read bounded stdin; emit framed human or
  machine output; flush; observe shutdown.
- Failure classes to keep distinct: malformed arguments, excessive input, broken pipe, output
  failure, signal, and cleanup failure.
- Known or plausible consumers: `lkjscript`, batch tools, service administration, and packaged
  applications.
- Current priority: foundational.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 2. terminal session

- Purpose: adapt a live terminal without giving applications raw escape authority.
- Application or semantic owner: interaction state, commands, logical frame content, cursor intent,
  and product policy.
- Trusted native or deployment owner: raw mode, event decoding, safe projection, differential
  output, signals, and cleanup.
- Operation surface to evaluate: acquire; decode key/paste/mouse/resize/focus; render logical frame;
  suspend; resume; close.
- Failure classes to keep distinct: decode failure, unsupported capability, output failure, EOF,
  signal, panic, and cleanup failure.
- Known or plausible consumers: `lkjedit` and future terminal applications.
- Current priority: existing secondary.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 3. selected local filesystem

- Purpose: grant bounded local file and directory authority under an exact root.
- Application or semantic owner: which files to open, edit, search, create, rename, delete, or
  present.
- Trusted native or deployment owner: path confinement, descriptors, metadata, atomic publication,
  and platform adaptation.
- Operation surface to evaluate: list; stat; read; stream; search; create; expected-base replace;
  rename; delete; reconcile.
- Failure classes to keep distinct: absence, conflict, invalid type, permission denial, excessive
  data, I/O failure, and unknown visibility.
- Known or plausible consumers: editors, build tools, import/export, backup, and local data
  applications.
- Current priority: existing and extensible.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 4. immutable blob namespace

- Purpose: publish and inspect content-addressed immutable bytes.
- Application or semantic owner: blob purpose, reference, retention intent, and domain response.
- Trusted native or deployment owner: blob path or service adaptation, no-replace publication,
  digest verification, and reconciliation.
- Operation surface to evaluate: put immutable blob; inspect presence; read bounded blob; reconcile
  uncertain publication.
- Failure classes to keep distinct: already present, digest conflict, known failure, possible
  visibility, timeout, and corruption.
- Known or plausible consumers: durable instances, attachments, packages, caches, and object
  abstractions.
- Current priority: existing generic seed.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 5. deployment configuration

- Purpose: bind typed runtime configuration without making environment state semantic identity.
- Application or semantic owner: configuration schema, defaults, required fields, and application
  policy.
- Trusted native or deployment owner: environment, files, command flags, secret references, and
  deployment-source precedence.
- Operation surface to evaluate: load; merge under declared precedence; validate; inspect redacted
  values; watch only when specified.
- Failure classes to keep distinct: missing value, malformed value, conflicting sources, forbidden
  source, and redaction failure.
- Known or plausible consumers: all packaged applications and runtime topologies.
- Current priority: required for service platform.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 6. secret store

- Purpose: provide sensitive deployment values under least authority.
- Application or semantic owner: which named secret is required and how its absence affects
  application behavior.
- Trusted native or deployment owner: secret acquisition, protected memory best effort, redaction,
  rotation observation, and cleanup.
- Operation surface to evaluate: resolve exact secret; use through narrow adapter; compare or derive
  only when contract permits; close.
- Failure classes to keep distinct: missing, denied, expired, rotated, malformed, provider
  unavailable, and cleanup failure.
- Known or plausible consumers: database, object storage, outbound APIs, sessions, signing, and
  password-reset services.
- Current priority: required for service platform.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 7. wall clock

- Purpose: observe civil time explicitly.
- Application or semantic owner: expiry, timestamps, user-visible time policy, and domain
  comparisons.
- Trusted native or deployment owner: OS clock observation and conversion primitives.
- Operation surface to evaluate: observe UTC instant; convert under explicit timezone data; format
  under explicit policy.
- Failure classes to keep distinct: clock unavailable, out-of-range instant, unsupported timezone,
  and conversion failure.
- Known or plausible consumers: sessions, snapshots, audit events, cache expiry, jobs, and service
  responses.
- Current priority: required for service platform.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 8. monotonic clock and deadlines

- Purpose: bound operational waiting and measure elapsed stages without entering semantic identity.
- Application or semantic owner: whether a caller requests an observable timeout or cancellation
  policy.
- Trusted native or deployment owner: monotonic observation, timers, wakeups, and deadline
  enforcement.
- Operation surface to evaluate: create deadline; wait; observe elapsed; cancel wait; expire task.
- Failure classes to keep distinct: deadline exceeded, cancellation, timer resource exhaustion, and
  shutdown.
- Known or plausible consumers: network, database, object storage, workers, tests, and telemetry.
- Current priority: required with asynchronous runtime.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 9. secure randomness

- Purpose: generate unpredictable bytes through explicit authority.
- Application or semantic owner: token length, identifier policy, and use of the generated value.
- Trusted native or deployment owner: operating-system cryptographic randomness.
- Operation surface to evaluate: fill bounded bytes; generate typed token through library
  composition.
- Failure classes to keep distinct: entropy source unavailable, excessive request, and cancellation
  before use.
- Known or plausible consumers: sessions, CSRF, password reset, API tokens, nonces, and identifiers.
- Current priority: required for service platform.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 10. deterministic randomness

- Purpose: support reproducible algorithms and tests without pretending to be cryptographically
  secure.
- Application or semantic owner: seed, algorithm identity, stream splitting, and deterministic
  consumption order.
- Trusted native or deployment owner: none required beyond efficient generic primitives.
- Operation surface to evaluate: initialize; draw integer or bytes; split stream; serialize state
  only when specified.
- Failure classes to keep distinct: invalid range, excessive output, unsupported algorithm identity.
- Known or plausible consumers: property tests, simulations, procedural applications, and
  deterministic fixtures.
- Current priority: general library.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 11. identifier generation

- Purpose: produce canonical unique identifiers under an explicit algorithm.
- Application or semantic owner: identifier domain, display, ordering, and continuity policy.
- Trusted native or deployment owner: clock and secure-random combination when required.
- Operation surface to evaluate: generate; parse; format; compare; extract only specified fields.
- Failure classes to keep distinct: malformed spelling, unavailable time or randomness, and
  collision handling.
- Known or plausible consumers: resources, users, jobs, sessions, artifacts, and application
  entities.
- Current priority: required for service platform.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 12. cryptographic hashing

- Purpose: compute explicit integrity or lookup digests without granting provenance.
- Application or semantic owner: domain separation, algorithm, digest use, and equality policy.
- Trusted native or deployment owner: constant-space optimized implementation.
- Operation surface to evaluate: hash bounded bytes or stream; incremental update; finalize; parse
  and format digest.
- Failure classes to keep distinct: unsupported algorithm, excessive stream, cancellation, and
  malformed digest.
- Known or plausible consumers: artifacts, object keys, cache keys, tokens, external URL
  normalization, and evidence.
- Current priority: existing and extensible.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 13. password hashing

- Purpose: derive and verify password hashes under a deployment security policy.
- Application or semantic owner: credential lifecycle, authentication response, upgrade decision,
  and authorization policy.
- Trusted native or deployment owner: Argon2 or selected algorithm, secure randomness, constant-time
  verification, and resource limits.
- Operation surface to evaluate: hash password; verify; inspect parameters; recommend upgrade.
- Failure classes to keep distinct: mismatch, malformed hash, resource exhaustion, unsupported
  parameters, and infrastructure failure.
- Known or plausible consumers: local user authentication and password reset.
- Current priority: required for a multi-user reference service.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 14. structured logging

- Purpose: record bounded operational events without becoming application authority.
- Application or semantic owner: event names and safe application fields when explicitly emitted.
- Trusted native or deployment owner: timestamps, sinks, framing, rotation, filtering, and
  deployment integration.
- Operation surface to evaluate: emit event; attach trace context; flush; rotate; inspect
  dropped-event counters.
- Failure classes to keep distinct: sink unavailable, event rejected, excessive field, redaction
  failure, and flush failure.
- Known or plausible consumers: runtime, services, workers, adapters, and verification.
- Current priority: required for resident runtime.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 15. metrics

- Purpose: expose aggregate operational observations without changing semantic behavior.
- Application or semantic owner: stable metric meaning and dimensions chosen by reusable libraries
  or applications.
- Trusted native or deployment owner: collection, aggregation, scrape or export, and cardinality
  limits.
- Operation surface to evaluate: counter; gauge; histogram; snapshot; export.
- Failure classes to keep distinct: invalid metric, cardinality exhaustion, exporter failure, and
  dropped observation.
- Known or plausible consumers: service runtime, workers, database pools, queues, and performance
  evidence.
- Current priority: required for service operability.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 16. distributed trace context

- Purpose: propagate bounded correlation across capability calls.
- Application or semantic owner: none beyond explicitly surfaced correlation when application policy
  requires it.
- Trusted native or deployment owner: context parsing, propagation, span timing, sampling, and
  export.
- Operation surface to evaluate: accept context; create child span; annotate safe fields; close;
  export.
- Failure classes to keep distinct: malformed context, sampling drop, exporter failure, and
  cardinality exhaustion.
- Known or plausible consumers: HTTP services, database calls, object storage, and outbound
  providers.
- Current priority: later generic observability.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 17. JSON codec

- Purpose: convert between typed values and strict JSON at public boundaries.
- Application or semantic owner: schema, field names, optionality, unknown-field policy, and numeric
  interpretation.
- Trusted native or deployment owner: bounded parser and encoder.
- Operation surface to evaluate: decode exact type; encode exact type; stream when required; report
  path and offset.
- Failure classes to keep distinct: malformed syntax, duplicate field, unknown field, invalid
  Unicode, range, depth, size, and trailing input.
- Known or plausible consumers: CLI machine mode, HTTP APIs, configuration, database JSON, and
  provider adapters.
- Current priority: required.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 18. URL and form codec

- Purpose: parse and construct URI components and form payloads without string conventions.
- Application or semantic owner: route variables, query policy, allowed schemes, and application
  validation.
- Trusted native or deployment owner: standards-compliant parsing, normalization, and percent
  encoding.
- Operation surface to evaluate: parse URI; resolve relative reference; encode/decode query;
  encode/decode form.
- Failure classes to keep distinct: malformed URI, forbidden scheme, invalid percent encoding,
  excessive component, and ambiguous normalization.
- Known or plausible consumers: HTTP server/client, redirects, object links, and external embeds.
- Current priority: required for service platform.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 19. multipart codec

- Purpose: accept and emit bounded multipart bodies with streaming parts.
- Application or semantic owner: accepted fields, media policy, and application ownership of
  uploaded resources.
- Trusted native or deployment owner: boundary parsing, temporary spill policy, stream management,
  and cleanup.
- Operation surface to evaluate: iterate parts; read field; stream file; reject excess; emit
  multipart when needed.
- Failure classes to keep distinct: malformed boundary, duplicate field, excessive headers,
  excessive part, disk exhaustion, cancellation, and cleanup failure.
- Known or plausible consumers: file and media uploads, forms, and outbound provider APIs.
- Current priority: required for media-capable service.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 20. HTML construction and escaping

- Purpose: build safe server-rendered markup with context-correct escaping.
- Application or semantic owner: page structure, components, labels, navigation, and application
  presentation.
- Trusted native or deployment owner: efficient escaping and optional template compilation.
- Operation surface to evaluate: construct text nodes; attributes; URLs; fragments; trusted narrow
  nodes; stream output.
- Failure classes to keep distinct: invalid trusted fragment, excessive output, invalid URL context,
  and rendering exhaustion.
- Known or plausible consumers: server-rendered services and generated documentation.
- Current priority: required for a server-rendered reference service.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 21. Markdown parsing and sanitization

- Purpose: turn Markdown into a safe typed or rendered representation.
- Application or semantic owner: enabled syntax, link policy, embed policy, and product
  presentation.
- Trusted native or deployment owner: parser, sanitizer, and optimized rendering primitives.
- Operation surface to evaluate: parse; inspect nodes; sanitize links and HTML; render; extract
  plain text.
- Failure classes to keep distinct: excessive nesting, excessive output, invalid link, unsupported
  extension, and sanitizer rejection.
- Known or plausible consumers: note systems, documentation tools, previews, and content
  applications.
- Current priority: required for a note-service forcing function.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 22. HTTP server

- Purpose: accept bounded HTTP requests and dispatch typed handlers.
- Application or semantic owner: route table, middleware order, authentication, authorization,
  handler behavior, and response content.
- Trusted native or deployment owner: listeners, TLS boundary, HTTP parsing, connection lifecycle,
  body streams, and generic compression.
- Operation surface to evaluate: bind listener; accept; decode request; dispatch; stream response;
  graceful shutdown.
- Failure classes to keep distinct: bind failure, malformed request, size limit, timeout,
  disconnect, overload, handler failure, and shutdown.
- Known or plausible consumers: resource services, APIs, webhooks, dashboards, and local development
  servers.
- Current priority: required campaign capability.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 23. HTTP client

- Purpose: perform outbound bounded HTTP operations under an exact network grant.
- Application or semantic owner: endpoint choice, request data, domain retry decision, and response
  interpretation.
- Trusted native or deployment owner: DNS, TLS, connection pooling, protocol framing, streaming, and
  redirect mechanics.
- Operation surface to evaluate: send request; stream upload; stream response; cancel; close pool.
- Failure classes to keep distinct: DNS, TLS, connect, protocol, status policy, timeout,
  cancellation, size, redirect, and unknown request visibility.
- Known or plausible consumers: LLM providers, external embeds, webhooks, object adapters, and
  service integrations.
- Current priority: required after server vertical.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 24. WebSocket

- Purpose: exchange bounded framed messages over a long-lived upgraded connection.
- Application or semantic owner: session protocol, actor policy, message meaning, and close
  behavior.
- Trusted native or deployment owner: upgrade, framing, ping/pong, backpressure, and socket
  lifecycle.
- Operation surface to evaluate: accept or connect; send; receive; close; observe peer close.
- Failure classes to keep distinct: upgrade failure, invalid frame, excessive message, backpressure,
  disconnect, timeout, and shutdown.
- Known or plausible consumers: live status, collaborative features, event streams, and control
  channels.
- Current priority: later unless selected service slice requires it.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 25. streaming byte transport

- Purpose: move large or unbounded-by-single-value data with bounded retained memory.
- Application or semantic owner: chunk interpretation, completion, cancellation, and domain checksum
  policy.
- Trusted native or deployment owner: buffers, readiness, wakeups, backpressure, and transport
  cleanup.
- Operation surface to evaluate: read chunk; write chunk; flush; close; cancel; compute bounded
  transforms.
- Failure classes to keep distinct: producer failure, consumer failure, cancellation, timeout, size
  limit, and cleanup failure.
- Known or plausible consumers: HTTP bodies, object storage, file I/O, compression, media, and
  providers.
- Current priority: required campaign capability.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 26. compression

- Purpose: compress or decompress bounded streams under an explicit algorithm.
- Application or semantic owner: content negotiation and whether compression is acceptable for the
  domain.
- Trusted native or deployment owner: algorithm implementation and streaming buffers.
- Operation surface to evaluate: encode; decode; negotiate supported algorithms; bound expanded
  bytes.
- Failure classes to keep distinct: malformed stream, expansion limit, unsupported algorithm,
  cancellation, and output failure.
- Known or plausible consumers: HTTP, artifacts, backups, and object transfer.
- Current priority: generic adapter after streaming.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 27. relational database session

- Purpose: execute typed parameterized statements against a granted relational database.
- Application or semantic owner: queries, row interpretation, constraints, domain transactions, and
  application policy.
- Trusted native or deployment owner: driver protocol, connection pool, prepared statements, TLS,
  and cancellation.
- Operation surface to evaluate: acquire session; execute; query rows; stream rows; begin
  transaction; close.
- Failure classes to keep distinct: pool exhausted, connection, protocol, syntax, constraint,
  serialization, timeout, cancellation, and unknown commit visibility.
- Known or plausible consumers: multi-user services, analytics, search, sessions, audit, and durable
  queues.
- Current priority: required campaign capability.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 28. database migration

- Purpose: apply exact ordered schema changes as deployment authority.
- Application or semantic owner: migration declarations, application schema expectation, and cutover
  policy.
- Trusted native or deployment owner: database locks, transaction execution, checksum table, and
  operational reporting.
- Operation surface to evaluate: inspect current version; validate sequence; apply; verify; report.
- Failure classes to keep distinct: checksum mismatch, divergent history, lock failure, statement
  failure, partial visibility, and incompatible live state.
- Known or plausible consumers: service deployment and durable database-backed applications.
- Current priority: required before production service claim.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 29. object storage

- Purpose: store and retrieve named immutable or versioned objects under bucket and prefix grants.
- Application or semantic owner: key policy, metadata, visibility, retention, and application
  references.
- Trusted native or deployment owner: S3-compatible protocol, credentials, retries, multipart,
  ranges, checksums, and connection reuse.
- Operation surface to evaluate: ensure namespace; put; multipart put; get; range get; head; list;
  copy; delete; reconcile.
- Failure classes to keep distinct: not found, conflict, denied, checksum mismatch, timeout,
  cancellation, partial multipart, and unknown visibility.
- Known or plausible consumers: media, backups, large attachments, artifacts, and service data.
- Current priority: required campaign capability.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 30. durable queue

- Purpose: coordinate durable asynchronous jobs with bounded concurrency and explicit retry.
- Application or semantic owner: job payload, state machine, retry policy, cancellation, result, and
  domain idempotency.
- Trusted native or deployment owner: atomic claim, lease, heartbeat, persistence, wakeup, and
  worker admission.
- Operation surface to evaluate: enqueue; claim; heartbeat; complete; fail; retry; cancel; inspect;
  reap stale claim.
- Failure classes to keep distinct: duplicate, conflict, lease lost, storage failure, cancellation,
  exhaustion, poison payload, and indeterminate completion.
- Known or plausible consumers: transcription, media derivatives, email, indexing, imports, and
  maintenance.
- Current priority: required campaign capability.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 31. scheduled timer and recurring work

- Purpose: trigger bounded work from explicit time policies.
- Application or semantic owner: schedule, catch-up, missed-run, idempotency, and domain behavior.
- Trusted native or deployment owner: timer persistence or wakeup, clock observation, and leader
  coordination.
- Operation surface to evaluate: register; cancel; inspect due work; claim occurrence; complete
  occurrence.
- Failure classes to keep distinct: clock error, duplicate occurrence, missed deadline, storage
  failure, and shutdown.
- Known or plausible consumers: cache refresh, cleanup, reports, session expiry, and maintenance.
- Current priority: later after durable queue.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 32. bounded worker pool

- Purpose: execute independent tasks concurrently under deployment limits.
- Application or semantic owner: task graph, result handling, cancellation, and domain retry.
- Trusted native or deployment owner: threads or async tasks, queues, scheduling, panic containment,
  and shutdown.
- Operation surface to evaluate: admit task; execute; cancel; collect; drain; close.
- Failure classes to keep distinct: overload, panic, cancellation, deadline, stale result, and
  shutdown timeout.
- Known or plausible consumers: services, object transforms, searches, compilation, and durable
  jobs.
- Current priority: required with resident service.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 33. cache

- Purpose: reuse disposable derived values under exact invalidation.
- Application or semantic owner: whether stale data is acceptable and how cache absence affects
  behavior.
- Trusted native or deployment owner: storage, eviction, concurrency, serialization, and metrics.
- Operation surface to evaluate: lookup; populate; invalidate; evict; inspect; clear.
- Failure classes to keep distinct: miss, corrupt entry, capacity exhaustion, unavailable backend,
  and population race.
- Known or plausible consumers: compiled code, parsed templates, external embeds, rendered pages,
  and query acceleration.
- Current priority: evidence-gated.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 34. semantic project host

- Purpose: let an application inspect or propose changes to another exact semantic project.
- Application or semantic owner: project query, proposal, review, and application presentation.
- Trusted native or deployment owner: project discovery, locking, validation, publication, and
  bounded transport.
- Operation surface to evaluate: orient; query; validate; apply; history; diff; target; reconcile.
- Failure classes to keep distinct: stale base, foreign project, ambiguity, corruption, busy
  authority, output failure, and unknown publication.
- Known or plausible consumers: `lkjedit`, development tools, and future semantic automation.
- Current priority: existing secondary.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 35. child process

- Purpose: run a narrowly granted external executable when no safer adapter suffices.
- Application or semantic owner: command intent, arguments, input, expected output, and domain
  interpretation.
- Trusted native or deployment owner: spawn, environment isolation, pipes, signals, limits, wait,
  and cleanup.
- Operation surface to evaluate: spawn exact executable; stream input/output; signal; wait; cancel;
  close.
- Failure classes to keep distinct: not found, denied, spawn failure, output limit, timeout, signal,
  nonzero exit, and cleanup failure.
- Known or plausible consumers: tool integration and constrained build workflows only.
- Current priority: not required; threat-model before addition.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 36. image and media transform

- Purpose: perform reusable bounded decoding, validation, and transformation of media.
- Application or semantic owner: accepted media kinds, derivative policy, quality, dimensions, and
  product use.
- Trusted native or deployment owner: codec libraries, CPU-intensive transforms, streaming, and
  memory limits.
- Operation surface to evaluate: inspect; decode; resize; transcode; encode; extract metadata.
- Failure classes to keep distinct: malformed media, unsupported codec, dimension bomb, resource
  exhaustion, cancellation, and encoder failure.
- Known or plausible consumers: media services, document tools, previews, and asset pipelines.
- Current priority: standard library or adapter after core service.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

### 37. live media transport

- Purpose: relay real-time audio or video under explicit session authority.
- Application or semantic owner: rooms, membership, publication policy, controls, and product
  presentation.
- Trusted native or deployment owner: WebRTC or selected transport, codecs, ICE, congestion, and
  connection lifecycle.
- Operation surface to evaluate: create session; publish track; subscribe; signal; close.
- Failure classes to keep distinct: negotiation, network, codec, authorization, overload,
  disconnect, and cleanup.
- Known or plausible consumers: future live applications including a possible `kjxlkj` live surface.
- Current priority: explicitly outside initial foundation unless selected by evidence.
- The requirement and grant must be exact, typed, bounded, inspectable, and independently testable.
- Live handles, credentials, deployment locators, and adapter-internal state must not become durable
  application values.
- Production and deterministic fake adapters must share a conformance suite while remaining
  implementation-disjoint.
- Document idempotency, possible visibility, retry, cancellation, cleanup, restart, and resource
  accounting before production use.
- Reject any product-specific policy that migrates into this adapter merely to reduce lkjscript
  code.

## Identity audit catalog

When adding or changing one of these domains, write its allocation, canonical spelling, equality,
scope, retention, deletion, nonreuse, foreign-domain rejection, and recovery rules.

### 1. project

- Continuity: one maintained development authority.
- It is not interchangeable with: path, Git repository, or package.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 2. project revision

- Continuity: one accepted authored and semantic state.
- It is not interchangeable with: wall-clock time or Git commit.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 3. revision record

- Continuity: canonical facts about one accepted transition.
- It is not interchangeable with: human log line.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 4. module

- Continuity: one authored namespace and loading unit.
- It is not interchangeable with: file path alone.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 5. package

- Continuity: one exact reusable authored and semantic closure.
- It is not interchangeable with: registry coordinate alone.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 6. declaration

- Continuity: one durable semantic entity when continuity is required.
- It is not interchangeable with: name or source position.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 7. local binding

- Continuity: one scope-local value origin.
- It is not interchangeable with: durable declaration identity.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 8. type

- Continuity: one exact structural or nominal contract.
- It is not interchangeable with: display spelling alone.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 9. effect or capability requirement

- Continuity: one exact requested interface contract.
- It is not interchangeable with: deployment grant.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 10. capability grant

- Continuity: one concrete bounded external authority binding.
- It is not interchangeable with: interface requirement.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 11. adapter

- Continuity: one implementation kind for a capability.
- It is not interchangeable with: application behavior.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 12. component

- Continuity: one composable application unit.
- It is not interchangeable with: process.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 13. application

- Continuity: one exact runnable component closure.
- It is not interchangeable with: artifact path or product install.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 14. application artifact

- Continuity: one immutable distribution object.
- It is not interchangeable with: development workspace.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 15. deployment

- Continuity: one concrete binding of artifacts, grants, config, and topology.
- It is not interchangeable with: application identity.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 16. runtime process

- Continuity: one operational execution lifetime.
- It is not interchangeable with: durable instance.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 17. runtime task

- Continuity: one admitted structured unit of execution.
- It is not interchangeable with: thread ID or queue position.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 18. request

- Continuity: one inbound operation and response lifecycle.
- It is not interchangeable with: connection.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 19. connection

- Continuity: one transport lifecycle.
- It is not interchangeable with: authenticated actor.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 20. stream

- Continuity: one ordered bounded-flow lifetime.
- It is not interchangeable with: complete byte value.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 21. transaction

- Continuity: one database atomicity scope.
- It is not interchangeable with: durable application revision.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 22. database row

- Continuity: one database-owned entity continuity.
- It is not interchangeable with: application nominal value without mapping.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 23. object

- Continuity: one object-storage key/version continuity.
- It is not interchangeable with: local file path.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 24. queue job

- Continuity: one durable asynchronous work item.
- It is not interchangeable with: worker task.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 25. job attempt

- Continuity: one claim and execution attempt.
- It is not interchangeable with: job identity.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 26. timer occurrence

- Continuity: one scheduled trigger occurrence.
- It is not interchangeable with: wall-clock timestamp alone.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 27. user

- Continuity: one authenticated principal continuity.
- It is not interchangeable with: email or username spelling.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 28. session

- Continuity: one authenticated session lifecycle.
- It is not interchangeable with: user.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 29. service account

- Continuity: one non-human principal continuity.
- It is not interchangeable with: API token.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 30. API token

- Continuity: one revocable credential continuity.
- It is not interchangeable with: service account.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 31. secret

- Continuity: one deployment-sensitive value binding.
- It is not interchangeable with: ordinary text.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 32. terminal session

- Continuity: one acquired terminal lifecycle.
- It is not interchangeable with: interactive application state.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 33. filesystem root

- Continuity: one granted path authority.
- It is not interchangeable with: canonical path string.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 34. file observation

- Continuity: one exact observed external file base.
- It is not interchangeable with: buffer or path.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 35. buffer

- Continuity: one editable content continuity.
- It is not interchangeable with: file observation.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 36. view

- Continuity: one presentation and local interaction continuity.
- It is not interchangeable with: buffer.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 37. tab or layout item

- Continuity: one movable interactive item.
- It is not interchangeable with: rendered coordinates.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 38. render cache

- Continuity: one disposable previous-output optimization.
- It is not interchangeable with: logical frame.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 39. compiled unit

- Continuity: one derived executable form bound to exact semantic inputs.
- It is not interchangeable with: semantic declaration.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 40. cache entry

- Continuity: one disposable derived value under exact invalidation.
- It is not interchangeable with: authority.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 41. backup

- Continuity: one immutable recoverable copy under an exact contract.
- It is not interchangeable with: live project.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

### 42. evidence receipt

- Continuity: one observation record for an exact run.
- It is not interchangeable with: semantic authority.
- State the owning layer and the exact point at which the identity is allocated.
- State whether equality is nominal, structural, digest-based, sequence-based, or deployment-local.
- State whether the identity survives rename, movement, serialization, restart, backup, restore, and
  import.
- State retention, tombstone, reuse, and garbage-collection behavior.
- Reject bytes from another identity domain even when their printed form matches.
- Keep paths, positions, indexes, addresses, process IDs, and timestamps outside identity unless the
  contract explicitly says otherwise.

## Failure classification catalog

Use stable typed classes across human output, machine output, tests, adapters, and evidence. A
domain may refine these classes, but it must not collapse materially different retry or publication
behavior.

### 1. `malformed_input`

- Meaning: closed syntax, encoding, or framing is invalid.
- Required behavior: perform no domain work.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 2. `noncanonical_input`

- Meaning: input decodes but violates canonical form.
- Required behavior: reject before identity or publication use.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 3. `unsupported_version`

- Meaning: the exact contract or format is not current.
- Required behavior: reject without compatibility fallback.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 4. `foreign_identity`

- Meaning: a value belongs to another authority or identity domain.
- Required behavior: reject before lookup or mutation.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 5. `ambiguous_selector`

- Meaning: a friendly selector resolves to more than one exact target.
- Required behavior: require a more exact selector.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 6. `missing_authority`

- Meaning: the required project, package, instance, grant, or resource is absent.
- Required behavior: report the missing domain exactly.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 7. `authority_denied`

- Meaning: the caller lacks the exact grant or authorization.
- Required behavior: perform no denied operation.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 8. `authority_busy`

- Meaning: bounded admission cannot accept the operation.
- Required behavior: reject or expose an exact retryable operational outcome.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 9. `stale_base`

- Meaning: the expected authority revision or observation no longer matches.
- Required behavior: do not refresh or retry silently.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 10. `semantic_rejection`

- Meaning: well-framed authored meaning violates the language or application contract.
- Required behavior: publish nothing.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 11. `incomplete_program`

- Meaning: the selected execution closure contains a hole or missing body.
- Required behavior: do not execute.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 12. `invalid_derived_ir`

- Meaning: compiler output fails independent verification.
- Required behavior: fail closed and preserve the authored authority.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 13. `runtime_trap`

- Meaning: an exact language operation contract is violated.
- Required behavior: publish no external authority by itself.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 14. `resource_exhausted`

- Meaning: a named semantic or operational limit is exceeded.
- Required behavior: report unit, limit, and requested amount.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 15. `overload`

- Meaning: queue, worker, connection, or pool admission is saturated.
- Required behavior: apply the documented reject, queue, or shed policy.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 16. `cancelled`

- Meaning: the owning scope requested cancellation before the non-cancellable boundary.
- Required behavior: run specified cleanup and return cancellation.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 17. `deadline_exceeded`

- Meaning: an operational deadline expired.
- Required behavior: distinguish from deterministic fuel exhaustion.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 18. `timeout_unknown`

- Meaning: a timed operation may have crossed an external visibility boundary.
- Required behavior: do not retry automatically.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 19. `known_previsibility_failure`

- Meaning: external work is known not to have become visible.
- Required behavior: allow policy-controlled retry.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 20. `unknown_visibility`

- Meaning: external publication may be visible but is not confirmed.
- Required behavior: require reconciliation.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 21. `conflict`

- Meaning: external or durable state no longer matches the exact expected base.
- Required behavior: require explicit application policy.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 22. `duplicate_idempotent`

- Meaning: the exact operation was already completed with the same intent.
- Required behavior: return the prior exact outcome.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 23. `idempotency_conflict`

- Meaning: an idempotency key was reused for different intent.
- Required behavior: reject and preserve evidence.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 24. `not_found`

- Meaning: the selected external object or row is absent.
- Required behavior: return an exact expected outcome when domain-appropriate.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 25. `permission_denied`

- Meaning: the external system denied the operation.
- Required behavior: keep distinct from application authorization.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 26. `constraint_violation`

- Meaning: a database or external invariant rejects the requested mutation.
- Required behavior: map through a closed typed boundary.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 27. `serialization_failure`

- Meaning: a concurrent transaction cannot commit under the selected isolation.
- Required behavior: apply only the declared retry policy.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 28. `connection_failure`

- Meaning: a transport or database connection could not be established or was lost.
- Required behavior: classify visibility and retryability.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 29. `protocol_failure`

- Meaning: a peer violated the selected protocol contract.
- Required behavior: close or reject under the protocol policy.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 30. `checksum_mismatch`

- Meaning: received or retained bytes do not match the expected digest.
- Required behavior: fail closed and preserve evidence.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 31. `corrupt_authority`

- Meaning: durable authority fails canonical validation.
- Required behavior: fail closed and do not repair silently.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 32. `cache_corrupt`

- Meaning: disposable acceleration is invalid.
- Required behavior: discard and fall back to the independent owner.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 33. `cache_miss`

- Meaning: disposable acceleration is absent.
- Required behavior: recompute without changing semantics.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 34. `output_failure`

- Meaning: the result could not be fully encoded, written, or flushed.
- Required behavior: do not roll back already accepted authority.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 35. `cleanup_failure`

- Meaning: a live resource is not known to be restored or closed.
- Required behavior: attempt remaining cleanup and report precedence.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 36. `worker_panic`

- Meaning: a native worker failed unexpectedly.
- Required behavior: contain it, classify in-flight work, and preserve process policy.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 37. `stale_result`

- Meaning: an asynchronous result no longer matches reachable pending state.
- Required behavior: discard or expose explicitly without mutating unrelated state.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 38. `shutdown`

- Meaning: the runtime is stopping admission or execution.
- Required behavior: apply the documented drain and cancellation policy.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 39. `unavailable_dependency`

- Meaning: a required tool, adapter, service, or platform is unavailable.
- Required behavior: never report pass.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

### 40. `indeterminate`

- Meaning: the system lacks enough evidence to classify the outcome safely.
- Required behavior: preserve uncertainty and provide a recovery route.
- State whether semantic or external publication occurred.
- State retryability, reconciliation, cancellation, and cleanup implications.
- Include bounded exact identity and continuation facts where applicable.
- Keep this class distinct from transport text, application display messages, and unrelated cleanup
  failures.

## Cross-boundary verification catalog

Apply every relevant dimension below to each changed authority, format, capability, runtime
topology, and maintained application. Mark irrelevant dimensions explicitly in the campaign ledger
rather than silently ignoring them.

### 1. empty input and minimum valid input

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 2. typical valid input

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 3. maximum exact valid input

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 4. one unit over every bound

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 5. repeated identical success

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 6. semantic no-change

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 7. validate-only parity

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 8. stale expected base

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 9. future or nonexistent base

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 10. foreign authority identity

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 11. foreign nominal type with equal shape

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 12. ambiguous friendly selector

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 13. unknown field or operation

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 14. duplicate field or member

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 15. truncated encoding

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 16. trailing encoding

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 17. invalid UTF-8

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 18. noncanonical integer, digest, identifier, or path spelling

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 19. excessive nesting and recursion

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 20. excessive collection items

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 21. excessive visible bytes

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 22. excessive retained bytes

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 23. fuel exhaustion

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 24. frame or stack exhaustion

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 25. queue saturation

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 26. worker saturation

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 27. connection-pool saturation

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 28. stream backpressure

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 29. consumer cancellation

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 30. producer cancellation

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 31. deadline before external visibility

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 32. deadline after possible external visibility

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 33. known external failure

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 34. unknown external visibility

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 35. reconciliation present

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 36. reconciliation absent

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 37. reconciliation indeterminate

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 38. output encoding failure before publication

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 39. output write failure after publication

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 40. cleanup failure after success

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 41. cleanup failure after domain failure

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 42. process restart before work

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 43. process restart during disposable acceleration

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 44. process restart after durable publication

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 45. cache miss

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 46. cache hit

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 47. cache eviction

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 48. cache corruption

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 49. old format direct rejection

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 50. corrupt current authority

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 51. corrupt unrelated historical authority

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 52. backup and restore

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 53. fresh-checkout reproduction

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 54. deterministic repeated build

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 55. parallel independent requests

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 56. conflicting concurrent mutations

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 57. maximally interleaved task schedule

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 58. graceful shutdown with no work

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 59. graceful shutdown with in-flight idempotent work

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 60. graceful shutdown with in-flight possibly visible work

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 61. authentication failure

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 62. authorization denial

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 63. cross-tenant access denial

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 64. secret redaction

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 65. log and metric cardinality bounds

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 66. database transaction rollback

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 67. database serialization retry boundary

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 68. object multipart interruption

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 69. durable queue duplicate delivery

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 70. durable queue lease loss

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 71. worker stale completion

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 72. HTTP malformed request

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 73. HTTP oversized header

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 74. HTTP oversized body

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 75. HTTP client redirect restriction

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 76. HTTP disconnect during response stream

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 77. terminal malformed sequence

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 78. terminal output failure and cleanup

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 79. filesystem path substitution

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 80. filesystem symlink or mount escape

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 81. editor large-file and viewport behavior

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 82. agent-oriented compact success output

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

### 83. bounded failure excerpt with complete retained log

- Define the exact immutable fixture or generated corpus.
- Define the independent expected semantic result and publication result.
- Define the expected stable failure or success class.
- Define exact resource, timing, ordering, and cleanup observations when relevant.
- Exercise the public production boundary, not only a private helper.
- Retain a focused regression when this dimension has caused or could cause a serious failure.

## Performance measurement catalog

### 1. project discovery

- Separate stages: directory traversal, marker validation, and authority selection.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 2. project open

- Separate stages: current authority decode, history checks, indexes, and locks.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 3. task orientation

- Separate stages: initial bytes, files, commands, and latency needed to identify owners.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 4. task-scoped context

- Separate stages: relevant source and semantic bytes, omissions, and expansion calls.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 5. edit validation

- Separate stages: parse, normalization, type checking, impact, and response preflight.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 6. edit publication

- Separate stages: durable writes, synchronization, record creation, and output.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 7. history query

- Separate stages: record loading, diff, pagination, and reconstruction.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 8. package resolution

- Separate stages: dependency loading, identity validation, and closure selection.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 9. parsing

- Separate stages: source bytes, syntax tree allocation, errors, and incremental reuse.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 10. semantic validation

- Separate stages: scope, type, effect, capability, and target validation.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 11. lowering

- Separate stages: reachable closure discovery, IR construction, and verification.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 12. prepared application startup

- Separate stages: artifact decode, validation, compilation, and adapter binding.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 13. interpreter dispatch

- Separate stages: instructions, branches, calls, values, and frame traffic.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 14. optimized execution

- Separate stages: bytecode, specialization, compiled code, and deoptimization.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 15. value materialization

- Separate stages: copying, sharing, serialization, and boundary conversion.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 16. text editing

- Separate stages: splice, undo, line navigation, grapheme work, and viewport extraction.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 17. interactive event

- Separate stages: decode through visible frame flush.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 18. render construction

- Separate stages: logical frame or scene projection.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 19. terminal emission

- Separate stages: diff, encoding, write, flush, and acknowledgment.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 20. service request

- Separate stages: accept through complete response or stream close.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 21. route dispatch

- Separate stages: method/path matching, middleware, and handler selection.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 22. request body streaming

- Separate stages: retained memory, backpressure, and cancellation.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 23. database query

- Separate stages: pool wait, prepare, execute, row decode, and handler use.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 24. database transaction

- Separate stages: begin through commit or rollback including contention.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 25. object read

- Separate stages: request, first byte, throughput, checksum, and close.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 26. object write

- Separate stages: upload, multipart, checksum, commit, and reconciliation.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 27. durable queue

- Separate stages: enqueue, claim latency, execution, completion, retry, and backlog.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 28. worker throughput

- Separate stages: admission, concurrency, CPU, memory, failures, and drain.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 29. shutdown

- Separate stages: stop admission, in-flight completion, cancellation, cleanup, and exit.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 30. verification

- Separate stages: selected gates, repeated work, logs, success bytes, and failure quality.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

### 31. fresh build

- Separate stages: toolchain, dependency, compile, package, artifact reproduction, and install.
- Freeze an equal complete workload and exact environment before optimization.
- Measure optimized production builds and retain raw receipts.
- Record latency, throughput, CPU time, retained memory or logical accounting, bytes, calls, and
  dominant stage as available.
- Compare against the simple independent oracle and the reproduced predecessor.
- Set a quantitative keep, stop, and reversal gate before retaining complexity.
- Do not attribute whole-workflow improvement to one stage without stage evidence.

## Architectural anti-pattern catalog

### 1. Do not retain a design whose justification is adding a product noun to the language schema.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 2. Do not retain a design whose justification is moving application policy into a native adapter.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 3. Do not retain a design whose justification is keeping a private builder because public authoring is inconvenient.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 4. Do not retain a design whose justification is maintaining both source and graph as editable truth.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 5. Do not retain a design whose justification is preserving a format reader for reassurance.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 6. Do not retain a design whose justification is introducing editions instead of direct cutover.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 7. Do not retain a design whose justification is encoding all effects as opaque bytes.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 8. Do not retain a design whose justification is granting ambient filesystem or network access.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 9. Do not retain a design whose justification is using environment variables as semantic identity.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 10. Do not retain a design whose justification is using a digest as authorization or provenance.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 11. Do not retain a design whose justification is using a process boundary as a sandbox claim.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 12. Do not retain a design whose justification is using one scalar fuel value for memory, I/O, queues, and time.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 13. Do not retain a design whose justification is raising limits to hide an asymptotic defect.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 14. Do not retain a design whose justification is optimizing a microbenchmark while the complete workflow regresses.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 15. Do not retain a design whose justification is adding async without cancellation and structured ownership.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 16. Do not retain a design whose justification is adding a worker pool with an unbounded queue.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 17. Do not retain a design whose justification is retrying after possible external visibility.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 18. Do not retain a design whose justification is letting a transaction handle escape its scope.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 19. Do not retain a design whose justification is serializing a live resource into durable state.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 20. Do not retain a design whose justification is using wall-clock time as deterministic semantics accidentally.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 21. Do not retain a design whose justification is logging secrets or private content by default.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 22. Do not retain a design whose justification is printing every passing test to a coding agent.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 23. Do not retain a design whose justification is discarding full diagnostics to make output compact.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 24. Do not retain a design whose justification is caching without exact input identity and fallback.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 25. Do not retain a design whose justification is calling skipped or unavailable verification a pass.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 26. Do not retain a design whose justification is adding a package registry before exact local package composition.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 27. Do not retain a design whose justification is building an ORM before typed query boundaries are proven.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 28. Do not retain a design whose justification is building a plugin system before a trust and capability model.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 29. Do not retain a design whose justification is adding WebRTC to the core because one product mentions live media.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 30. Do not retain a design whose justification is keeping application-specific Rust bindings as the real API.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 31. Do not retain a design whose justification is calling an artifact-reproduction test a satisfactory product.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 32. Do not retain a design whose justification is treating `lkjedit` acceptance as general-language acceptance.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 33. Do not retain a design whose justification is treating `kjxlkj` schema as standard-library semantics.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 34. Do not retain a design whose justification is treating a recent design as permanent because it was expensive.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 35. Do not retain a design whose justification is leaving obsolete docs or tests active after cutover.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 36. Do not retain a design whose justification is splitting code by arbitrary line limits while duplicating owners.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 37. Do not retain a design whose justification is combining unrelated authority domains for atomicity convenience.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 38. Do not retain a design whose justification is making failures strings that callers must parse.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 39. Do not retain a design whose justification is letting machine output mix with progress text.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.

### 40. Do not retain a design whose justification is claiming token or monetary savings from byte counts.

- Identify the underlying consumer obligation instead.
- Choose the narrowest coherent semantic owner.
- Keep external mechanics in a generic adapter with explicit grants.
- Preserve an independent oracle and exact failure behavior.
- Delete the workaround when the replacement is complete.
