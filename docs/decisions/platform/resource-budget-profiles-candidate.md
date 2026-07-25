# Resource Budget Profiles: Current Compiler Foundation

[Authority](resource-budget-profiles.md)

## Status

**Current for the compiler foundation described here.** Resource Profile V1,
checked ledgers, the five named ceiling sets, profile-aware compiler entry
points, successful compile metrics, and publication guards are implemented.
This does not weaken or replace an Edition 1 or implementation safety limit.
The one-shot Semantic Protocol now selects the same closed profile identities
and ceilings for its request-local categories. Pre-allocation compiler charging
and one ledger shared across compiler, protocol, repository, agent-state,
artifact, proof, or runtime authorities remain Accepted work.

## Identity And Selection

The identity is `lkjscript.resource-profile` version `1` with implementation
maxima version `1`, one registered name, and SHA-256 over the closed ordered
ceiling array. The exact names are `sandbox`, `default`, `build`,
`trusted-local`, and `deterministic`. Unknown names fail. `trusted-local` equals
bounded implementation maxima; it is not an unsafe or unbounded mode.

A host may lower any category from a selected profile. Raising it fails.
Existing compile entry points select `default`; explicit Rust API entry points
accept a `ResourceProfile`. The `run` and `disasm` CLI commands accept
`--resource-profile NAME` before the source path and reject unknown names.
Successful `CompileMetrics` exposes profile identity and exact charged totals,
and `ExecutableProgram` retains the profile identity. Package manifests do not
yet select profiles.

## Closed V1 Categories

The 25 categories are `source_bytes`, `source_units`, `import_edges`, `tokens`,
`schema_nodes`, `top_level_declarations`, `product_fields`, `parser_work`,
`validation_work`, `path_work`, `type_nesting`, `type_work`, `trait_work`,
`ownership_expressions`, `ownership_retained_state`, `hir_functions`,
`hir_expressions`, `ssa_functions`, `ssa_blocks`, `ssa_values`, `ssa_edges`,
`ssa_frame_states`, `diagnostics`, `protocol_request_bytes`, and
`protocol_response_bytes`.

Bytes categories use exact bytes; parser, validation, path, type, and trait work
use deterministic work units; all other categories use records. Unknown
categories require a new profile version rather than an alias.

## Concrete Ceilings

Columns are exact inclusive ceilings.

| category | sandbox | default | build | trusted-local | deterministic |
| --- | ---: | ---: | ---: | ---: | ---: |
| source_bytes | 1,048,576 | 16,777,216 | 134,217,728 | 268,435,456 | 8,388,608 | <!-- LKJ-EXACT-DATA -->
| source_units | 1,024 | 8,192 | 32,768 | 65,536 | 4,096 | <!-- LKJ-EXACT-DATA -->
| import_edges | 4,096 | 65,536 | 262,144 | 524,288 | 32,768 | <!-- LKJ-EXACT-DATA -->
| tokens | 393,216 | 3,145,728 | 12,582,912 | 25,165,824 | 1,572,864 | <!-- LKJ-EXACT-DATA -->
| schema_nodes | 393,216 | 3,145,728 | 12,582,912 | 25,165,824 | 1,572,864 | <!-- LKJ-EXACT-DATA -->
| top_level_declarations | 8,192 | 65,536 | 262,144 | 524,288 | 32,768 | <!-- LKJ-EXACT-DATA -->
| product_fields | 122,880 | 983,040 | 3,932,160 | 7,864,320 | 491,520 | <!-- LKJ-EXACT-DATA -->
| parser_work | 786,432 | 6,291,456 | 25,165,824 | 50,331,648 | 3,145,728 | <!-- LKJ-EXACT-DATA -->
| validation_work | 786,432 | 6,291,456 | 25,165,824 | 50,331,648 | 3,145,728 | <!-- LKJ-EXACT-DATA -->
| path_work | 8,192 | 73,728 | 294,912 | 589,824 | 36,864 | <!-- LKJ-EXACT-DATA -->
| type_nesting | 32,768 | 131,072 | 524,288 | 1,048,576 | 65,536 | <!-- LKJ-EXACT-DATA -->
| type_work | 2,000,000 | 12,000,000 | 50,000,000 | 100,000,000 | 8,000,000 | <!-- LKJ-EXACT-DATA -->
| trait_work | 250,000 | 1,000,000 | 5,000,000 | 10,000,000 | 750,000 | <!-- LKJ-EXACT-DATA -->
| ownership_expressions | 16,384 | 16,384 | 16,384 | 16,384 | 16,384 | <!-- LKJ-EXACT-DATA -->
| ownership_retained_state | 131,072 | 262,144 | 750,000 | 1,000,000 | 196,608 | <!-- LKJ-EXACT-DATA -->
| hir_functions | 8,193 | 65,537 | 262,145 | 524,289 | 32,769 | <!-- LKJ-EXACT-DATA -->
| hir_expressions | 16,384 | 16,384 | 16,384 | 16,384 | 16,384 | <!-- LKJ-EXACT-DATA -->
| ssa_functions | 8,193 | 65,537 | 262,145 | 524,289 | 32,769 | <!-- LKJ-EXACT-DATA -->
| ssa_blocks | 262,144 | 1,000,000 | 3,000,000 | 4,000,000 | 524,288 | <!-- LKJ-EXACT-DATA -->
| ssa_values | 2,000,000 | 12,000,000 | 50,000,000 | 100,000,000 | 8,000,000 | <!-- LKJ-EXACT-DATA -->
| ssa_edges | 524,288 | 2,000,000 | 6,000,000 | 8,000,000 | 1,000,000 | <!-- LKJ-EXACT-DATA -->
| ssa_frame_states | 262,144 | 1,000,000 | 3,000,000 | 4,000,000 | 524,288 | <!-- LKJ-EXACT-DATA -->
| diagnostics | 4,096 | 65,536 | 500,000 | 2,000,000 | 32,768 | <!-- LKJ-EXACT-DATA -->
| protocol_request_bytes | 16,777,216 | 67,108,864 | 268,435,456 | 268,435,456 | 33,554,432 | <!-- LKJ-EXACT-DATA -->
| protocol_response_bytes | 16,777,216 | 67,108,864 | 268,435,456 | 268,435,456 | 33,554,432 | <!-- LKJ-EXACT-DATA -->

## Current Charging Points

One checked ledger flows through one compiler request. Addition is checked and
exhaustion rejects the attempted increment without mutating the recorded total.

- A complete validated source tree is charged exactly after loading/parsing and
  before HIR construction. Edition 1 and Semantic Source Foundation V1 limits
  remain the pre-allocation defenses for this phase.
- Constructed and ownership-checked HIR is charged before effect inference and
  SSA construction. Existing type, ownership, and HIR maxima remain the
  allocation defenses.
- Verified normalized SSA is charged before bytecode lowering and before an
  executable can be published. Existing IR and verifier maxima protect SSA
  construction.
- On compiler failure, one diagnostic record is charged. Charges completed by
  earlier phases are not rewound.

This is exact post-phase aggregate accounting and a publication guard, not a
claim that profile ceilings stop the first allocation in each phase. Moving the
same charges before allocation is a separate Current gate.

## Diagnostics And Failure

A resource diagnostic contains profile schema/version/name, category and unit,
configured limit, charge before, and attempted increment. Arithmetic overflow
uses the same deterministic diagnostic shape. No over-profile executable is
returned. Successful metrics expose the complete totals; failed requests do not
yet return a complete ledger summary or responsible semantic node.

`protocol_request_bytes` and `protocol_response_bytes` are reserved at zero in
the compiler ledger. Semantic Source V1 selects one of the same five profile
identities and intersects its request/response, source, schema-node, and
validation-work ceilings with stricter protocol/foundation maxima. Its ledger
is request-local and does not silently claim to be the compiler ledger.

## Edition 1 Non-Regression

Source depth 8, form children 16, 384 tokens per file, 8 top-level forms,
15 product fields, and 16 combined immediate source-directory entries remain
Current and unchanged. Foundation V1 source byte/unit/tree limits and all
compiler, IR, bytecode, proof, native-image, and execution maxima also remain
enforced. No fixed source rule becomes a profile or lint through this slice.

## Verified Boundaries

Focused tests cover all five profiles across the canonical source roots, unknown
names, lower-only ceilings, zero/exact/+1/overflow ledger behavior, deterministic
diagnostics, profile identity, nested phase propagation, diagnostic charging,
and type/ownership charge invariance. Full workspace, release, runtime, and
Docker evidence is recorded only after those commands run on the integrated
revision.
