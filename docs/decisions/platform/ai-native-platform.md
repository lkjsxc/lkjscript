# AI-Native Language And Platform

## Purpose

Define the long-term product identity and the migration authority for replacing
implementation-era constraints without misrepresenting the current compiler and
runtime.

## Status

**Accepted Target.** Semantic Source Foundation V1, typed HIR, verified SSA,
reference bytecode VM, callable Linux x86-64 baseline JIT, exact native roots,
and forced proof-checked optimizing JIT remain **Current**. This record changes
the accepted destination and dependency order; it does not make complete
Semantic Source/Agent Protocol V1, Edition 2, packages, general ownership, AOT,
Wasm, concurrency, or self-hosting Current.

The selected implementation sequence is bounded repository topology,
repository graph/context, agent work state, first Semantic Source operations,
and aggregate resource profiles. These are **Accepted Implementation
Contracts**, not Current code; they are indexed in [Platform
Decisions](README.md). Automatic optimizing promotion remains a valid later
measured experiment.

## Problem

The Current system has strong backend authority boundaries, but its primary
editing contract is physical text, its source tree is intentionally tiny, and
several host, memory, numeric, package, and execution choices are prototype
slices rather than a sufficient general platform. Continuing feature-by-feature
without first giving agents stable semantic entities would multiply migration
cost and preserve brittle line-oriented editing as accidental product identity.

## Decision

lkjscript is an AI-primary, statically typed, memory-safe language and platform
with:

- one versioned Semantic Source Schema for complete and incomplete programs;
- deterministic text projections for version control, audit, search, and
  recovery rather than text spans as the primary edit interface;
- one resolved typed HIR and verified semantic SSA family shared by evaluation,
  VM, baseline native compilation, proof-checked optimization, JIT, AOT, Wasm,
  analysis, and eventual self-hosting;
- value semantics by default and explicit identity-bearing abstractions;
- explicit effects and separately supplied typed capabilities;
- a hybrid memory model that distinguishes plain values, affine ownership,
  lexical borrows, regions, immutable sharing, worker-local tracing,
  synchronized sharing, pinning, and external resources;
- no undefined behavior in ordinary safe source;
- structured concurrency with scoped child lifetimes and ownership-derived
  transfer/share facts;
- reproducible packages, components, providers, artifacts, diagnostics, and
  tests; and
- a measured evaluator/VM/JIT/AOT/cache/Wasm execution portfolio.

Compiler-derived types, effects, capabilities, ownership facts, layouts,
proofs, and optimization eligibility remain derived authority. Serialized
source claims cannot grant those facts.

## Authority And Safety Invariants Preserved

This redesign retains the strongest Current foundations:

- canonical `.lkjscript` input remains the only accepted source extension until
  an editioned adapter explicitly changes that contract;
- exact types, dedicated `Unit`, explicit `Option`, typed empty values, exact
  Boolean conditions, eager specified evaluation order, checked integer
  arithmetic, and explicit recoverable failure remain binding;
- imports contain declarations only and library import has no execution effect;
- no universal dynamic equality, unchecked optimizer assumptions, raw pointers
  in safe source, or conservative GC root scanning;
- source lowers once through resolved typed HIR into verified typed SSA;
- whole-artifact validation, opaque verified artifacts, bounded proof checking,
  exact roots, W^X, structured outcomes, deterministic budgets, and no silent
  forced-engine fallback remain binding; and
- unsafe Rust remains confined to `lkjscript-sys`, whose safe interface must be
  sound for every Rust caller.

A replacement may strengthen or generalize these invariants, but may not bypass
them during migration.

## Superseded Product Assumptions

The following are no longer permanent product decisions:

| Earlier assumption | Replacement authority | Current migration rule |
| --- | --- | --- |
| Physical named-open/named-close lines identify the language | Versioned Semantic Source with measured deterministic projections | Edition 1 text remains Current until roundtrip and migration gates pass |
| Tiny depth, token, form, field, and directory-width numbers are semantic forever | Implementation safety maxima, host-selected profiles, and AI-maintainability lints | No Current limit is weakened before aggregate replacement bounds are Current |
| One program-global imported declaration namespace is sufficient | Explicit package/module/declaration identities and qualified imports | Current resolution remains until package migration is complete |
| `I64` and `F64` are the final numeric surface | Edition 2 exact-width numeric slices with explicit conversions | Current numeric behavior remains exact and unchanged |
| `Result T Str` is a general system error model | Nominal typed provider/domain errors and distinct outcome channels | Current wrappers remain until mechanically migrated |
| One universal `Handle` is the public resource model | Typed affine resources and typestate | Current handles retain stale-safe behavior during migration |
| Host authority may be ambient | Explicit typed capabilities supplied through an application/component context | Existing ambient wrappers are transitional Current behavior |
| Ordinary aggregates are semantically heap objects | Value semantics with compiler-selected placement | Current representation is not a future semantic promise |
| Stable handles are the only native reference representation worth considering | Exact direct generated-code references may be measured behind stack maps; handles remain valid boundaries | No representation changes without exact-root and stale-reference evidence |
| Runtime JIT is the only final deployment strategy; offline PGO/cache are permanently rejected | The measured execution portfolio in [Execution Portfolio](../execution/execution-portfolio.md) | Current JIT modes and evidence remain unchanged |
| Zero dependencies is an end in itself | Measured trusted-computing-base classification | Every added dependency still requires an accepted measured decision |
| `Owned Buf` is the final ownership surface | Inferred modes and type-specific ownership with place-based borrowing | The Current safe island remains honestly partial |

The historical records remain evidence. Their incompatible permanent policy is
superseded; their descriptions of Current behavior and measured results are not.

## Target Architecture And Status Matrix

| Layer | Current at the adoption baseline | Accepted target | First acceptance evidence |
| --- | --- | --- | --- |
| Source | Edition 1 line syntax parsed to internal forms | Versioned Semantic Source, deterministic projections, typed holes | Exact corpus parse/format/parse and byte-canonical roundtrip |
| Agent interface | Files, human diagnostics, CLI compilation | Revisioned semantic queries and atomic edits with structured diagnostics | Stale/precondition rejection and no partial writes |
| Semantics | Products, marker traits, I64/F64, partial ownership island | Edition 2 ADTs/match/Never, exact widths, typed errors, general safe ownership | Cross-evaluator/VM differential and malformed-boundary gates per slice |
| Authority | Operation/effect summaries plus transitional ambient host services | Explicit capabilities and typed provider resources | Capability-confinement and fake-provider tests |
| Packages | Contained import roots and environment fallback | Manifest, lock, content identity, explicit modules/components | Clean locked hermetic rebuild and fingerprint tests |
| IR | Resolved typed HIR, verified SSA, exact roots/proofs | Ownership/effect/capability/drop/metering-complete IR family | Independent verifier and differential gates |
| Execution | Evaluator, validated VM, baseline JIT, forced proof JIT on Linux x86-64 | VM, generated baseline, optimizing JIT, AOT, cache, optional local PGO, OSR, Wasm | Workload-specific predeclared adoption gates |
| Runtime | Precise non-moving traced heap and stable handles | Replaceable placement/collector plans and runtime profiles | Exact-root stress plus latency/throughput/RSS evidence |
| Concurrency | Synchronous single-worker runtime | Isolates, structured task scopes, bounded channels, deterministic scheduler tests | Race/cancellation/leak schedule exploration |
| Portability | Linux x86-64 acceptance | Linux AArch64, Wasm/components, then measured additional targets | Same semantic IR and explicit unsupported-target failures |
| Self-hosting | Rust bootstrap implementation | Staged lkjscript frontend, analyses, optimizer, and tools | Reproducible normalized stage comparison |

A Target cell is not a capability claim.

## Dependency Order

The accepted sequence is:

1. Semantic Source, structured diagnostics, semantic transactions, typed holes,
   and an AI-authorability harness;
2. aggregate safety budgets, resource profiles, and maintainability lints;
3. Edition 2 ADTs, control flow, typed errors, and exact conversion semantics;
4. modules, packages, capabilities, typed resources, and provider schemas;
5. final data, ownership, drop, region, sharing, and tracing foundations;
6. IR, logical metering, incremental queries, and artifact identities;
7. measured runtime and execution portfolio;
8. structured concurrency, components, and Wasm;
9. second native architecture and staged self-hosting; and
10. Web, database, and later product ecosystems.

A later visible feature does not pre-empt an inconsistent foundational
contract.

## Evidence Method

Every nontrivial slice receives independent architect, implementer,
adversarial, verification, performance, AI-usability, and integration reviews.
One process may perform roles sequentially, but one implementer's confidence is
not acceptance evidence. Public contracts precede implementation. Positive,
negative, malformed, adversarial, resource-boundary, and differential tests
precede adoption. Obsolete paths are deleted after complete migration; they are
not retained as aliases or fallback lookup.

Performance and AI-authorability use retained predeclared protocols. Category-
specific claims replace universal language-speed or AI-superiority claims.
Failed experiments and counterexamples remain recorded.

## Research Inputs

The initial direction adopts, as hypotheses to test rather than authorities:

- CODESTRUCT's named-entity read/edit action space: its reported SWE-Bench gains
  and token reductions motivate semantic entity operations, but do not establish
  lkjscript's schema or weak-model results;
- typed-hole contextualization and Hazel's meaningful incomplete-program model:
  expected types and visible bindings motivate compiler-produced context, while
  release artifacts still reject unresolved holes;
- type-constrained generation and XGrammar: legal-action masks are a future
  compiler service, but any incomplete constrainer must disclose coverage and
  must not silently reject valid programs; and
- proof/translation-validation work: discovery remains untrusted and small
  independent checkers retain execution authority.

Each later ownership, effects, GC, concurrency, optimization, component, or
formal-method mechanism is adopted only against a documented lkjscript problem
and predeclared gate.

## Not Current

This decision alone implements no new syntax, package, capability, ownership,
collector, execution mode, backend, concurrency feature, component, proof,
agent daemon, or self-hosted stage. Current behavior remains exactly as stated
in [Current State](../../current-state.md).
