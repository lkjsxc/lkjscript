# Resource Budget Profiles: First Current-Candidate Contract

[Authority](resource-budget-profiles.md)

## Status

**Accepted Implementation Contract.** This defines aggregate accounting shared
by topology, Semantic Source, repository graph/context, and agent work state.
It is not Current. Every Current Edition 1 limit remains enforced unchanged.

## Profile Identity

A budget set has identity `lkjscript.resource-profile`, version `1`, a registered
profile name, policy fingerprint, implementation-maxima version, and sorted
closed category map. Unknown names, versions, fields, categories, units, and
arithmetic modes fail. A host may lower a registered ceiling but cannot raise an
implementation maximum or disable a safety category.

Profiles are `sandbox`, `default`, `server`, `build`, `trusted-local`, and
`deterministic`. The first implementation may register only profiles with
measured concrete numbers; an unregistered name is not an alias. Artifact-
affecting profile identity participates in artifact/cache keys.

## Exact Aggregate Charge Categories

V1 category names and units are:

- input: `source_file_bytes`, `source_closure_bytes`, `protocol_request_bytes`,
  `protocol_response_bytes`, and `repository_input_bytes` in exact bytes;
- topology: `paths`, `directories`, `directory_entries`, `directory_depth`,
  `links`, `manifest_records`, and `provenance_records` in records;
- source shape: `source_units`, `import_edges`, `tokens`, `schema_nodes`,
  `top_level_declarations`, `product_fields`, and `nesting_depth` in records;
- analysis: `parse_work`, `resolution_work`, `type_work`, `trait_work`,
  `effect_iterations`, `ownership_expressions`, `ownership_cfg_cells`, and
  `fixed_point_work` in deterministic work units;
- IR/artifact: `hir_nodes`, `ssa_functions`, `ssa_blocks`, `ssa_values`,
  `ssa_edges`, `proof_records`, `proof_work`, `bytecode_bytes`,
  `native_code_bytes`, `artifact_metadata_bytes`, and `stack_map_roots`;
- repository intelligence: `graph_nodes`, `graph_edges`, `graph_traversal_work`,
  `context_nodes`, `context_bytes`, and `context_ranking_work`;
- task state: `tasks`, `task_history_records`, `task_operations`,
  `task_preconditions`, `task_evidence_records`, and `publication_bytes`; and
- runtime: `fuel`, `frames`, `stack_values`, `heap_estimated_bytes`,
  `allocations`, `collections`, `handles`, `tasks_spawned`, `channel_items`,
  `io_bytes`, `output_bytes`, and `deadline_polls`.

A subsystem may define a more specific child category only in a new profile
version. It must still charge the parent amplification path. Physical bytes,
record counts, deterministic work units, and wall/deadline policy are not
silently converted into one another.

## Charging Semantics

Every authority boundary precharges metadata and worst-case retained growth
before allocation, recursion, indexing, or staging. Actual bytes read are then
charged exactly, including a sentinel read used to detect overflow. Counts and
products use checked integer arithmetic. A semantic item that amplifies more
than one independent resource charges each applicable category; this is not
double counting.

Nested operations share one aggregate ledger. Child completion returns its
charges but does not reset them. Failed and rejected work remains charged for
the request. Rollback releases staged memory but never rewinds logical work or
IO charges. Cached or optimized execution may reduce physical work only where
the profile permits; `deterministic` preserves declared semantic charges.

Exhaustion occurs before the first operation that would exceed a ceiling. The
structured diagnostic contains category, unit, configured limit, charge before,
attempted increment, profile/version, responsible authority/node, and complete
request charge summary. No truncated analysis, partial graph, partial semantic
revision, verified artifact, or task publication is returned as success.

## Edition 1 Non-Regression

The following remain Current and unchanged: source depth 8, form children 16,
384 tokens per file, 8 top-level forms, 15 product fields, and 16 combined
immediate files/directories per lkjscript source directory. Current compiler,
IR, bytecode, proof, native-image, execution, and Foundation V1 maxima also
remain enforced.

Repository authored-file and directory limits are a separate topology policy.
They do not raise or reclassify Edition 1 source limits. No Current source limit
becomes a profile ceiling or lint until the authority's migration gates are
implemented, measured, documented, and Current.

## Acceptance Gates

Focused tests must cover zero, exact-limit, limit-plus-one, arithmetic overflow,
nested-ledger propagation, failed-work charging, rollback, lowered host limits,
unknown schema/category, deterministic diagnostics, and cross-subsystem
aggregate exhaustion. Corpus/adversarial measurements select concrete profile
numbers; documentation alone does not make any profile Current.
