# Evidence-gated roadmap

Implemented work is intentionally absent. Graph 5 authority, CLI 12, offline minimal, command, and
HTTP recipes, exact built-in interface/deployment discovery, compact task/capability authoring,
normalized check/build/pure-run, artifact 10, maintained standard/`lkjournal` artifacts, reviewed
authored change, normalized query, standalone artifact-10 service/worker deployment, and both
static and PostgreSQL-backed copied-binary HTTP acceptance are current architecture. Immutable
`v0.1.9` is the supported public `x86_64-unknown-linux-musl` distribution; its exact and latest
downloads independently passed static, distributed HTTP, and stateful HTTP acceptance. Immutable
v0.1.8 remains an unclosed historical recovery point.

Each future item requires a named maintained consumer, exact contract, independent oracle,
measured reversal gate, and dependency-closed cutover/deletion plan.

1. **First-party semantic data engine and complete SQL/PostgreSQL cutover.** Use the public BBS and
   `lkjournal` as maintained workloads and PostgreSQL as a temporary differential oracle. Define
   provider-independent logical operations, transaction and durability semantics, ordering and
   query needs, migration/backup/restore/corruption behavior, resource bounds, and measured
   reversal conditions before selecting storage representation. Cut over all maintained consumers,
   reject predecessor configuration, delete SQL-specific application meaning and permanent dual
   production paths, and retain PostgreSQL only as evidence where explicitly required.
2. **Dependency-closed remaining HTTP topology authoring.** Expose component, requirement, port,
   and target creation only when another maintained workflow needs topology beyond the closed
   recipe. Move every maintained consumer in one cutover and retain exact review, validation,
   failure, and predecessor-rejection evidence.
3. **Bounded context traversal.** Select an explicit traversal contract and independently justify
   a stateless continuation or bounded external output. Prove canonical ordering,
   revision/selector binding, cancellation, locality, and full-oracle equality without adapting a
   predecessor cursor or query index.
4. **Worker project recipe from a maintained binary-only consumer.** Add one only when a real
   standalone consumer fixes its semantic topology, grant closure, deployment defaults, and live
   acceptance. Do not infer a worker recipe from the existence of the resident runner.
5. **Outbound HTTP from a maintained consumer.** Select exact URL, DNS, TLS, secret, redirect,
   response-limit, cancellation, retry, and SSRF policy before adding an interface or adapter. Do
   not infer outbound authority from the existing inbound HTTP server.
6. **Large-graph compiler, artifact, and retention evidence.** Extend current locality and command
   lifecycle measurements to controlled 100,000- and million-owner topologies, sparse/dense and
   wide-fanout relations, large literals, and long histories. Measure compiler-unit selection,
   pack/catalog I/O, artifact closure, fsyncs, CPU, RSS, interruption, and cache recovery. Add
   revision pins, reader leases, registered backup roots, and an independent reachability oracle
   before any deletion or compaction mechanism.
7. **Remaining maintained authored operations.** Add operations such as move, rebind, signature
   and member/case edits, extraction, inline, and repair only from maintained workflows. Keep typed
   intent, exact identity continuity, reviewed semantic effects, complete discovery, proving tests,
   and predecessor rejection in each vertical slice.
8. **Additional platform releases one target at a time.** Treat each architecture and operating
   system as its own dependency-closed admission with a hosted execution oracle, exact runtime
   inventory, stable asset identity, and public-download smoke. Do not introduce a speculative
   build matrix.
9. **Normalized package management and removed operational workflows.** Introduce package
   inspection/staging/publication, history, drafts, review, backup, restore, and repository health
   only by exact consumer. Do not reinstate Graph 4 readers, compatibility commands, a general
   remote registry, or storage bytes as authoring input.
10. **Broader incremental compilation and validation.** Generalize beyond the currently selected
   compiler impacts and semantic edit classes only after clean/incremental artifact equality and
   randomized full-oracle evidence hold for signatures, types, effects, capabilities, targets,
   tests, dependencies, and mixed changes.
11. **Broad branch and pull-request CI.** Select this only when its independent operating value,
   required gate profile, retention, trust model, and recovery policy are explicit. Do not treat
   the release workflow or its transient artifacts as general CI.
12. **Installers, package managers, signing, and build provenance.** Each registry, installer,
   updater, mirror, signing identity, or provenance mechanism needs a named consumer, mutable
   authority policy, credential boundary, revocation/recovery procedure, and maintenance owner.
   None follows automatically from immutable release integrity.
13. **Language abstraction from real consumers.** Constraints, inference, lexical closures,
   component composition, specialization, AOT, JIT, SIMD, or custom allocation require multiple
   maintained workloads, independent semantic/reference behavior, measurements, and explicit
   reversal conditions.

TLS is not a roadmap item. The present HTTP/PostgreSQL boundary is plaintext and requires an
appropriate external trusted transport boundary. Hostile-code sandboxing and multi-tenant
isolation are separate unselected problems.
