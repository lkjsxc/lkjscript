# Current implementation status

**Status: currently implemented in this checkout.** This is a concise report of checkout behavior,
not a compatibility promise or normative specification. Code, tests, CLI definitions, schemas, and
manifests remain the executable authority.

## User path

The active product is local package compile/run plus the Semantic Source stdio interface. Package
files and the provisional line-oriented `.lkjscript` notation remain program authority. The
compiler loads exact package modules and imports, produces resolved typed HIR, checks
types/effects/ownership and a HIR memory plan, lowers and verifies SSA, validates bytecode, binds an
in-process prepared identity, and executes the program.

Implemented source behavior includes typed functions and calls, bindings and explicit mutation,
conditionals and loops, nominal products and enums, exhaustive matching, generics and trait-dispatch
subsets, numeric conversions, bytes and byte vectors, lists, typed host resources, errors, and
explicit capabilities. Executable examples and compiler/runtime tests own the exact accepted
surface; this document does not copy their tables.

`lkjscript run` synchronously attempts one supported baseline-native group reachable from `main`.
Eligibility, lowering, installation, or typed pre-entry decline drops the entire native attempt and
executes the unchanged validated program in the VM. Once native entry begins, its result is final
and the VM is not run. Unsupported I/O, generic, recursive-stack, and other native shapes therefore
remain valid through the generic VM route. There is no public engine, threshold,
automatic-transition, or forced-native option.

The broadly tested host is Linux x86-64. Portable Rust may build elsewhere, but another host or
native target is not claimed as tested.

## Phase 2 broader-platform deletion

The workspace now has 11 members and one application binary, `lkjscript`, as reported by Cargo
metadata. The following speculative products and mechanisms are deleted rather than archived or
feature-flagged:

- `lkjscript-runtime`, `lkjscript-resource`, `lkjscript-database`, and
  `lkjscript-linux-host`;
- daemon, process-cell, cell-test-worker, session-broker, resource benchmark, scheduler,
  topology-observation, and app `system` wiring;
- service database, durable control-store, directory-capability, local-control, and database-tenant
  host providers that had no consumer after service deletion;
- process bootstrap/provenance and execution-outcome codecs, resource-plane/runtime-control/
  component contract descriptors, and the global platform revision; and
- target-matrix, platform-revision, and empty configuration placeholders.

The app no longer has dependencies or tests for those surfaces. Contract and package locks are
regenerated from the retained local product. Prepared descriptors now contain only package
provenance, memory-plan and witness closure, semantic and optional native SSA identities, validated
bytecode identity, and the local execution contract digests. The resulting prepared identity is
used only in-process by verified SSA, validated bytecode, and compilation caches.

Deleting the service database wrapper did not delete the language SQLite capability. VM host
operations still dispatch SQLite directly through `lkjscript-sys`; stdio, clock, filesystem,
network, terminal, and entropy behavior used by local programs also remains. The retained hello,
Mandelbrot, editor, HTTP, byte, filesystem, hash, SQLite, and comparison smoke paths exercise the
local product rather than a daemon.

## Executable boundary

Native image installation remains a pre-entry, failure-atomic operation. It validates image
integrity and contracts, accounts the object, applies relocations in a private RW mapping, seals the
mapping RX, and publishes installer usage only after success. Dropping an installed image releases
both its mapping and accounted lease.

Collector-free `prepare_invocation` and explicit `prepare_region_invocation` validate entry and
typed arguments, materialize and reserve machine-call and runtime bookkeeping state, and perform
immediate cancellation, deadline, resource, and configured whole-group stack checks. Success
returns a non-cloneable `PreparedInvocation`; `enter(self)` consumes it exactly once across the
unsafe generated ABI call. Pre-entry and entered failures are disjoint types, and there is no VM
retry after entry.

Product metrics report `baseline-native` or `vm-fallback`, an optional decline reason, whether
native entry began, and preflight/lower/install/prepare/native/VM/total durations. Automatic
thresholds, per-function call records, retries, invalidation, runtime sessions, forced execution,
optimizing native lowering, optimization certificates, and the proof-oriented optimizer are
absent. Baseline normalization retains only the independently verified simple pass sequence.

## Scale and policy result

The ordinary trusted local path has no compiler profiles, cross-phase count budgets, or source/HIR/
SSA count ledgers that decide language validity. Trusted source loading, initial bytecode validation,
and local execution explicitly select unrestricted policy. The former token, child, top-level-form,
nesting, source-unit, HIR-memory, SSA-CFG, SSA-ownership-work, and retained-state-cell admission
quotas are removed.

Source positions and spans, revision-scoped source nodes, HIR and SSA identities, executable
operands and links, structural metadata, and runtime structural identities use wide representations
where they carry user-scale data, with checked conversion before host indexing. Parser/source-tree,
package, type/match, CFG, structural graph, and tested deep-destruction paths use explicit work
stacks or equivalent stack-safe designs. A few compiler-recursive paths remain localized behind
[`stack.rs`](../crates/lkjscript-compiler/src/stack.rs); its heap-backed segment geometry is private
tuning, not a language-depth limit.

Committed generated tests cross former boundaries, including 20,000 nested source expressions,
10,000-block CFG verification, 44,000 owned SSA parameters/places, 300-field products and enums,
1,024 parameters/arguments/locals, table index 65,536, and runtime structural collections above
65,535. Some largest production geometries remain ignored release stress tests because of high
runtime and memory cost.

## Retained validation and host boundaries

Semantic Source requests retain explicit coarse request/source/response byte, request-count, and
cancellation policy. Artifact validation may select a total-artifact-byte policy. The reusable
execution API still supports explicit limited policy for fuel, VM values/frames, heap/allocation,
handles, output, wall time, and cleanup-report retention; ordinary local compile/run selects the
unrestricted form. These policies control a request and do not redefine language validity.

Fail-closed validation remains at source and Semantic Source input, package/manifest/lock/import and
compiler path/symlink entry, capability dispatch, bytecode validation, relocation and W^X
installation, generated native entry, FFI, SQLite, filesystem/socket/terminal calls, and operating-
system errors. Filesystem/network/SQLite language operations use the current process's explicitly
granted capability and direct system mechanism; the deleted service sandbox, process framing,
peer-authorization, database-tenant, durable-store, and platform-observation boundaries are not
claimed as retained.

The execution-outcome wire codec is absent. `SemanticDagSnapshot` and authenticated sealed
structural snapshot import/export remain in memory for VM/JIT behavior and differential tests.
Their memory witness facts are named semantic-snapshot facts rather than process-codec facts.

## Semantic Source bootstrap

The current Semantic Source JSON/stdio service provides source-derived snapshots, revision-labelled
node and entity reads, diagnostics, typed-hole context and legal-action queries, and atomic preview
or publish transactions for declaration rename, expression replacement, and typed-hole operations.
It stages and validates source publication and rejects stale revisions and failed preconditions.
Candidate and legal-action responses are complete when returned; if response policy cannot carry a
complete result, the service returns typed `OutputLimit` before publication rather than truncating.

This is not the target semantic workspace. Its schema mirrors the current syntax tree, spans,
canonical text subtrees, and source files. A `NodeId` contains a revision and dense `u64` index;
identity is not stable across edits. Transactions rewrite text, and compilation reparses text.
General incomplete semantic states, edit-stable identity, broad semantic queries, pagination, and
direct semantic compilation are not implemented.

## Known gaps

- Text is still source authority; direct compilation from a syntax-independent semantic snapshot is
  absent.
- Semantic Source remains syntax-shaped and its wide dense IDs are not edit-stable semantic IDs.
- Recursive semantic-operation, transaction, runtime structural-value, and specialization paths do
  not all have deep-stack evidence. Some scale paths retain poor complexity and high peak memory.
- The SSA evaluator is an explicit test oracle behind `lkjscript-ir/test-oracle`; it is not a public
  runtime engine. Workspace `--all-features` verification compiles it for tests.
- Compact native layouts, machine-code offsets, registers/opcodes, OS fields, SQLite fields, and host
  `usize` remain private or external representation boundaries. Native specialization must decline
  to the generic VM before entry when it cannot represent an otherwise supported program.
- Daemon, multi-tenant database, distributed, scheduler, and broader platform products are absent by
  design until the local semantic model and measurements justify them.
- Representative post-reset startup, execution, memory, and generated-code measurements remain
  pending. The reproducible Phase 2 build-time and final-binary comparison is in
  [`performance.md`](performance.md).
