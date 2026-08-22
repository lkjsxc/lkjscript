# Evidence-gated roadmap

Implemented work is intentionally absent. Meaning graph contract 4, direct CLI v4, binary-only
`new`, six persistent Merkle maps, exact-ID imports and targets, local module rename, persisted
authenticated semantic summaries, four local transaction classes, and explicit rank-1 generics
with named pure function values are current architecture, not roadmap items. Local exact-owner/name
index contract 3 also delta-reuses content-addressed shards; the broad relation index does not.

Each remaining item requires a complete consumer, exact contract, measured reversal gate, and
direct-cutover deletion plan.

1. **Extend incremental validation and lowering.** The current precondition-free pure-body,
   independent-module-create, module-rename, and declaration-rename paths validate bounded slices
   and delta-update the authenticated semantic index. Extend frontier-driven invalidation to
   signatures, types, effects, capabilities, targets, tests, dependencies, and mixed changes. Remove
   fallback candidate reconstruction/cloning only after randomized and adversarial differential
   evidence is clean.
2. **Finish publication and broad-query locality.** Avoid reconstructing the complete logical graph
   before applying a bounded root delta, delta-update the broad relation index, verify only new
   immutable bytes plus exact parent bindings during publication, and extend deep doctor to the
   complete retained canonical/dependency/draft closure plus the full semantic oracle. Measure page
   reads/writes, reused objects, fsyncs, and true semantic closure.
3. **Complete declaration move.** Imports, targets, exports, and expressions are exact-ID bound;
   module and declaration rename are local. Add declaration move, preserve expected-name
   preconditions for caller intent, and prove declaration continuity and locality in diff/history.
4. **Large-graph physical evidence.** Exercise persistent pages at 10,000, 100,000, and one million
   logical owners across sparse, dense, wide-fanout, large-literal, and long-history topologies.
   Compare the current object-per-module layout with immutable packs before selecting packing.
   Exercise segmented backup/restore beyond the predecessor 128 MiB bound and retain peak RSS,
   throughput, corruption, and cross-filesystem staging evidence. Do not infer end-to-end locality
   from persistent-map unit tests or a one-at-a-time payload loop alone.
5. **Retention, packing, compaction, and garbage collection.** Extend the current read-only
   HEAD-plus-draft retention preview with exact revision pins, active-reader leases, registered
   backup roots, an independent reachability oracle, and interruption evidence before enabling any
   deletion. Add physical packing/compaction only after measurements. Never delete reachable
   authority.
6. **Persistent semantic conflict workflow.** Convert a conflicted merge into a typed
   non-executable draft and add bounded conflict list/detail/resolve operations. Accepted HEAD must
   remain conflict-free.
7. **Remaining authoring and refactoring forms.** Add delete, move, rebind, change-signature,
   add/remove field or case, extract, inline, repair, receipt expansion, and saved queries only as
   projections onto change v3 and query v2. Complete machine discovery for nested type/expression
   forms as part of that work.
8. **Incremental compilation and specialization evidence.** Bind compiler units to exact semantic
   inputs, compare incremental and clean artifacts, and measure maintained pure, HTTP, JSON,
   database, and worker workloads. Consider specialization, AOT, or JIT only if complete-workload
   evidence supports it.
9. **Additional abstraction only from consumers.** Constraints, closure capture, type inference,
    component composition, or graph-native recipes require multiple real users and end-to-end
    graph/CLI/validator/compiler/reference/diff/merge semantics. Do not leave half-features or a
    hidden macro language.
10. **Agent and verification economy.** Retain stateless correctness. Add a resident stdio session
    only if equal complete workflows show material benefit after exact revision pinning,
    cancellation, and resource bounds. Expand verification fingerprints, graph-impact selection,
    context handles, and optional real provider telemetry without inferring tokens or cost.
11. **Production adapter breadth.** PostgreSQL cancellation, live S3 conformance, response
    streaming, outbound HTTP, terminal, selected filesystem, structured observations, secret
    rotation, and cross-platform production each require a named consumer and complete failure
    workflow. Hostile-code sandboxing remains a separate project.

TLS is not a roadmap item. The supported HTTP listener is plaintext and PostgreSQL uses `NoTls`.
lkjscript does not plan HTTP TLS termination, PostgreSQL TLS, certificate parsing, certificate
management or rotation, ACME, or speculative TLS hooks. Encrypted transport belongs at an
appropriate external trusted boundary or in a different adapter outside current product scope.

Reversal gates remain: remove short/index shards if they do not improve equal tasks; remove the
source-shaped test oracle if its independent value no longer exceeds its cost; retain bytecode only
while differential equality holds; and reject any session or packing prototype whose
complete-workflow evidence is not material.
