# Evidence-gated roadmap

Implemented work is intentionally absent. Each item requires a complete consumer, exact contract,
measured reversal gate, and direct-cutover deletion plan.

1. **Incremental validation and lowering.** Replace complete candidate reconstruction on every
   transaction with stable-owner dependency invalidation while retaining the current full packed
   oracle. Require operation-sequence differential tests and material improvement on 90,000-owner
   retained-history workloads before selection.
2. **Packed history compaction and garbage collection.** Add retention policy, reachability proof,
   interrupted compaction tests, and before/after deep reconstruction. Do not delete any accepted
   revision, draft, receipt, or backup without explicit policy.
3. **Persistent semantic conflict workflow.** Convert a conflicted merge into an explicit draft and
   add bounded conflict show/resolve operations. Accepted HEAD must remain conflict-free.
4. **Convenience command projection.** Add create/replace/delete/rename/move/rebind,
   change-signature, add/remove field/case, extract, inline, repair, receipt-show, and saved-query
   commands only as projections onto the one transaction/query protocol.
5. **Large-graph physical revision.** Revisit module-table sharding when a local edit rewrites more
   than impact-proportional data, warm exact lookup exceeds 250 ms, or history/file-count overhead
   dominates. Compare packed multi-module segments, canonical table shards, and an embedded
   transactional index without changing logical graph authority.
6. **Million-module policy.** Current graph contract 1 caps roots at 100,000 modules. A higher
   contract needs measured memory, cold open, mutation, doctor, build, backup, restore, and
   interruption behavior; raising the limit alone is forbidden.
7. **Query and transaction evidence expansion.** Add explicit receipt retrieval, expansion handles
   for complete affected-owner sets, selected-field transaction projections, elapsed cancellation,
   and broader indexed/oracle topology suites.
8. **Semantic collaboration transport.** Add Git/PR automation for exact semantic diff summaries,
   graph bundle exchange, and branch allocation only after testing independent creation,
   rename/move composition, delete/modify, and tampered transport.
9. **Execution scaling.** Measure bytecode against the semantic oracle on sustained pure, HTTP,
   JSON, database, and worker workloads. Consider specialization, AOT, or JIT only if execution is
   at least 30 percent of a maintained complete workload.
10. **Production adapter breadth.** PostgreSQL TLS/cancellation, live S3 conformance, response
    streaming, outbound HTTP, terminal, selected filesystem, structured observations, secret
    rotation, and cross-platform production each require a named consumer and failure workflow.
    Hostile-code sandboxing remains a separate project.

Reversal gates remain: remove local short/index shards if they do not improve equal complete tasks;
remove text recovery import if its independent value does not exceed grammar/security cost; retain
bytecode only while differential equality holds; and keep the per-command topology until a daemon
shows material complete-workflow gain without owning meaning.
