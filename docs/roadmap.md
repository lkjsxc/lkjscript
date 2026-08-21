# Evidence-gated roadmap

Implemented work is intentionally absent from this file. Each item needs a complete consumer and a
direct-cutover plan before it becomes a public contract.

1. **Interactive and batch consumers.** Select or build a useful interactive application before
   implementing the declared interactive runner. Reintroduce terminal and selected-filesystem
   mechanics only as generic capabilities, with no editor profile or product action enum. A batch
   runner needs an ordinary data-transform consumer.
2. **Complete web boundary.** A service requiring large generated responses should force
   task-scoped response streams. Multipart, complete URI/form/cookie codecs, typed multi-context
   markup, Markdown parse/sanitize, compression, WebSocket, and HTTP client authority remain gated
   by separate complete workflows.
3. **Production adapter conformance.** Run the shared object suite against a selected S3-compatible
   service, including multipart interruption, range, metadata, and reconciliation. Add PostgreSQL
   TLS and protocol-level cancellation only with a deployment that needs them.
4. **Durable workflow breadth.** Add scheduled jobs or serialized semantic continuations only when
   a plain bounded queue payload and application state machine cannot express a maintained worker.
5. **Execution scaling.** Measure bytecode against the AST oracle on sustained pure, route, JSON,
   and worker workloads. Consider specialization or JIT only if execution remains dominant after
   prepared-program reuse; never expose compiler offsets as identity.
6. **Incremental developer economy.** Add declaration-level affected facts and reusable pass
   receipts only after measuring repeated equal edits. Cache identity must cover complete inputs,
   dependencies, toolchain, environment policy, and checker version.
7. **Deployment breadth.** Structured logging/metrics sinks, secret rotation, configuration watch,
   outbound allowlists, cross-platform production, and stronger tenant isolation each require a
   named operator and failure workflow. Hostile-code sandboxing remains a separate project.
