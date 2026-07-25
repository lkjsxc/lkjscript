# Resource Budget Profiles

## Purpose

Replace prototype-sized permanent source semantics with layered safety maxima,
host-selected resource profiles, and AI-maintainability lints without weakening
safety during migration.

## Status

**Accepted Target.** All Current Edition 1 limits remain enforced unchanged:
source depth 8, form children 16, tokens per file 384, top-level forms 8,
product fields 15, and 16 combined immediate entries per lkjscript source
directory. Existing compiler, IR, bytecode, runtime, proof, native-image, and
execution bounds also remain Current.

No limit is raised or reclassified until aggregate replacement bounds for its
amplification path are implemented, tested, documented, and Current.

This record supersedes the permanent-policy part of [Essential Source
Limits](limits/essential-limits.md) and [Lkjscript Source-Tree
Width](source-tree-limit.md). Those records remain authoritative descriptions
of Current Edition 1 behavior until the migration gate passes.

## Problem

The tiny Current limits are effective adversarial scaffolding, but they force
physical fragmentation and make valid larger programs impossible independent
of available compiler/runtime resources. Simply deleting or increasing them
would expose unbounded source bytes, import closure, parser nodes, solver work,
ownership state, SSA growth, proof work, metadata, and agent context.

Language safety and determinism require explicit bounded amplification. Human
and AI maintainability require different signals than overflow and denial-of-
service protection.

## Decision

Resource control has three non-interchangeable layers.

### 1. Implementation safety maxima

Very high, versioned, checked maxima prevent integer overflow, address-space
abuse, malformed-artifact amplification, verifier state explosion, and host API
misuse. They are always enforced and cannot be disabled by a profile.

A maximum belongs at every allocation/work authority boundary, including:

- source and protocol bytes;
- source units, imports, packages, nodes, tokens, nesting, declarations, fields,
  patterns, and generic/type depth;
- name resolution, trait solving, ownership/loan/region analysis, and fixed-point
  work;
- HIR/SSA functions, blocks, values, edges, substitutions, effects, places,
  frame states, safepoints, and roots;
- optimization candidates, edits, e-classes if later used, proof records,
  checker work, compile time, and compiler memory;
- bytecode, constants, tables, native code, relocations, metadata, stack maps,
  cache entries, and serialized artifacts; and
- runtime stack, heap, allocations, logical work, handles, tasks, channels, IO,
  output, deadlines, and provider resources.

All arithmetic used to compute or aggregate a charge is checked before
allocation or indexing. Exhaustion fails with a structured diagnostic or
outcome and cannot publish partially verified authority.

### 2. Host-selected resource profiles

A manifest, host, or explicit CLI selection chooses a named profile. Profiles
never change type, ownership, memory, capability, or artifact-validation
semantics. They only select lower resource ceilings or execution policy within
implementation maxima.

The registered target profile names are:

- `sandbox`: small untrusted inputs and strict deterministic amplification
  limits;
- `default`: ordinary local development and execution;
- `server`: bounded long-running throughput with explicit tail and queue limits;
- `build`: larger compiler/proof/artifact budgets with hermetic inputs;
- `trusted-local`: explicit local high ceilings, never an unsafe mode; and
- `deterministic`: logical metering, reproducible scheduling/time/provider
  policy, and no optimizer-dependent charge removal.

Concrete numbers are selected only from retained corpus, adversarial,
performance, and AI-maintenance measurements. A profile is versioned and its
identity participates in diagnostics and artifact/cache keys where it can alter
an artifact. Unknown profiles fail explicitly.

Hosts may choose lower ceilings for an execution. A source package cannot raise
host ceilings or weaken implementation maxima. Budget exhaustion reports the
profile, category, configured limit, observed/attempted charge, and responsible
semantic node or artifact section when available.

### 3. AI-maintainability lints

Function/module/package size, directory width, declaration count, nesting,
dependency fan-out, control-flow complexity, ownership-state complexity, and
retrieval-context footprint are configurable lints. They can offer semantic
split/move/extract repairs. They do not decide whether a program is type- or
memory-safe.

Lint thresholds may vary by repository policy and may be warnings or CI errors.
Canonical language validity does not vary with a lint configuration. A lint
must prefer semantic cohesion and measured retrieval cost over line count.

## Aggregate Source And Compiler Charges

The first replacement slice must add aggregate, pre-allocation charges for at
least:

- total bytes in the loaded source closure;
- number of source units and imported edges;
- total tokens and source-schema nodes;
- total top-level declarations and product fields;
- parser/validator work;
- type depth and type-checking work;
- ownership-expression and retained CFG-state work; and
- HIR/SSA construction totals.

Per-file and per-form checks remain as defense in depth until aggregate fuzzing
and corpus measurements justify migration. Directory enumeration is sorted and
charged before recursive loading. Symlink/canonicalization and import-cycle
checks remain fail closed.

## Semantic Metering Versus Physical Resources

Deterministic semantic charges are explicit IR events or preserved metadata for
logical work, logical allocation, IO, and other metered semantics. An optimizer
may remove physical allocation, checks, branches, or calls only while preserving
required logical charges in deterministic mode.

Normal performance profiles account actual allocation, code, memory, IO, and
runtime work as specified. They do not expose an optimizer's physical allocation
choice as ordinary source semantics. Allocation counters used for diagnostics
or performance are labeled physical/estimated rather than semantic.

This separation is required before general scalar replacement, stack/region
placement, allocation elimination, or reuse is adopted.

## Migration Gates

A Current fixed source limit can become a profile ceiling or lint only when:

1. its complete amplification path has aggregate checked charges and an
   implementation maximum;
2. malformed/adversarial inputs exhaust the named category before excessive
   allocation, recursion, or wall time;
3. diagnostics are deterministic and identify category/limit/charge;
4. canonical and external-project boundary tests retain rejection where the
   replacement is not yet active;
5. corpus and weak/strong-agent tasks measure compile memory/time, context,
   edit/repair behavior, and source cohesion before and after;
6. the formatter, Semantic Source service, one-shot compiler, and daemon apply
   the same profile contract; and
7. documentation and edition migration explicitly remove the old semantic rule.

The 16-entry source-directory rule does not become a lint before aggregate
source-unit/import/byte/node limits satisfy this gate. The same rule applies to
depth, child, token, top-level, and field limits.

## Rejected

- deleting Current limits before aggregate replacements exist;
- replacing one unexplained hardcoded number with a larger unexplained number;
- profile settings that weaken type or memory safety;
- unbounded `trusted` modes;
- counting optimizer-dependent physical allocations as deterministic language
  semantics;
- silently truncating source, diagnostics, proof search, or analysis and then
  granting verified authority; and
- treating directory/file width alone as a proxy for AI maintainability.

## Not Current

No named profile, aggregate source budget, semantic-charge IR operation, or
maintainability lint is implemented by this decision. Current limits and tests
remain unchanged until complete Phase 2 slices replace them.
