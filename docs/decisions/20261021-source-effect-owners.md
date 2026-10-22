# Interface, requirement and effect owner admission

Date: 2026-10-22 (UTC). Scope: a restricted E1 extension and physical compaction.
Candidate publication requires the final staged tree and fresh verification receipts.

## Decision and boundaries

Admit Interface declarations, Operations, Operation-owned Parameters, Component
Requirements, and Function-owned RequirementParameter and EffectParameter records.
Decode pure/task function rows and TaskFunction descriptors. The compiler's own
implementation remains within the existing pure Tier 1 bootstrap profile.

Five modules separate shape parsing, endpoint/context checks and function field
projection: `source-effect-shape`, `source-capability-shape`, `source-effect`,
`source-capability` and `source-function`. The maintained inventory contains
62 modules and 465 functions. The [source-input specification](../spec/selfhosting-source-input.md)
owns the exact wire tags, parents, namespaces and admission boundaries.

Current codecs determine all discriminants. Operation uses record tag 6,
directory domain 6 and owner kind 14. Requirement uses 10/10/18; EffectParameter
uses 3/16/24; RequirementParameter uses 16/17/25. Interface uses declaration-kind
byte 3, payload 2 and directory kind 4. Directory tags and owner-envelope enum
encodings are checked independently.

## Admission and independent controls

Shape checks require canonical names, ordered arrays and current enum variants.
Requirement constraints bind one Interface and a strictly ordered operation set.
Component limits admit at most 1,024 entries, increasing names, nonzero full-width
u64 magnitudes and unit tags 0 through 4. Effect rows admit at most 100,000 atoms.

Endpoints bind exact package/member identities and compatible parents. Constrained
operations belong to the referenced Interface. Function and Component readers walk
decoded arrays rather than skipping former empty vectors. Containment checks exact
forward membership and reverse parents. All 12 namespace classes use those parents.

Every supplied TaskFunction enters structural DAG/depth proof, including unused
descriptors. Context checks run at each owner's actual roots and direct task rows.
R/E identity must belong to the exact enclosing Function. Shared digests cannot
cache success across contexts. Structurally valid unused mixed-context descriptors
remain permitted until used in an invalid context.

Malformed shape is 12; unsupported forms/foreign packages are 13; missing or
incompatible endpoints are 14; containment/value scope is 15; namespace is 16;
type/R/E scope and structural depth are 17. Nominal flow remains 18 and resource
refusal 6. Each scoped row atom reserves work before lookup; remaining allowance
passes into children. No semantic, evaluator or native-memory budgets are conflated.

Independent fixtures cover operation parameters, requirements, R/E constraints,
direct/nested task rows, ordinary type parameters, empty task rows and u64 limits
above I64::MAX. Rebound negatives pass their preceding layer before rejection.
The same TaskFunction digest is tested in both valid and invalid declaration
contexts, in both traversal orders. Untaken branches and unused descriptors remain
in the source validators; only the independent Rust oracle uses a live-type
projection where needed.

Raw TaskFunction controls alternate parameter/result nesting at depths 256 and
257, with explicit allowance 128,000,000. Carriers occupy 54,451 and 54,612 bytes;
all four paths first pass namespace admission, then return 0 and 17 respectively.
Minimum-allowance refusal 6 is separate resource evidence. Exact composed work
2734/2735/2736 returns 6/0/0 for its own fixture. Derived type-cycle controls
remain distinct: ordinary rehashing cannot construct a hash-consistent raw cycle.
The corrected boundary receipt retains complete carrier hashes and allowance fields.

## Adopted graph and distinct capacity boundaries

Twenty-two ordinary public transactions admitted
`rev_e8682917d0eaef1ccd6dd37517ef3024b023d56cb96fcd71540f22fe37d660ea`.
Its semantic state is
`semantic_state_d10ac03835ca364518838b7c29ad2bd8dd69b9ecb6ec269b9fd84c5b8ff54b9b`.

| Measurement | Previous slice | Current slice | Delta |
| --- | ---: | ---: | ---: |
| Owners | 26,815 | 28,326 | +1,511 |
| Types | 26 | 27 | +1 |
| Current retirements | 7,393 | 9,881 | +2,488 |
| Raw carrier bytes | 6,015,449 | 6,793,590 | +778,141 |
| Artifact input bytes | 7,311,970 | 7,690,384 | +378,414 |
| Canonical units | 448 | 467 | +19 |
| Full-kernel semantic work | 8,629,356 | 9,097,803 | +468,447 |

The 6,793,590-byte carrier (6.479 MiB) has minimum allowance 908,735,232 and
1,595,018 bytes below the 8,388,608-byte source bound. Its SHA-256 is
`c387db4db5404258a9d6026c01f27e9c140c0002d8456ebdeb889f3ac38add7f`.
Semantic state does not commit unused descriptors; the complete carrier hash does.

The separate 7,690,384-byte mode-1 input (7.334 MiB) has 698,224 bytes left
within its 8 MiB bound; SHA-256
`b652bc6cfd69f89de6af668e7d7a3e48175ffaa64b713858227296b69fdcd1d5`.
It emits an 8,126,517-byte artifact (7.750 MiB); SHA-256
`06b14d5afdadde2ed47acf9e5cf10bab9597a8d30349900cb4364dfe025e8b1a`.
Aggregate kernel work leaves 902,197 of 10,000,000 units. No limit is raised.

The nominal validator uses 241 reachable functions, 13,424 nodes, 20 type rows
and 637,133 normalized-input bytes. Maximum encoded local/operand-stack slots
are 56/10; these do not measure native call depth or RSS. The Tier 1 ELF closure
remains 244 functions, 16,111 nodes and 794,236 input bytes; C2=C3 is 2,519,420
bytes. Backend artifact equality and behavioral mutation controls remain intact.
These are Tier 1 fixed points, not source-driven Tier 2 evidence.

## Physical storage and acceptance

In a quiescent window with 631,104,806,912 free bytes (587.762 GiB), public
`lkjscript gc` retained all 200,012 objects and their immutable history.
It reduced 956 packs / 393,152,063 bytes (374.939 MiB) to 13 packs /
334,241,707 bytes (318.758 MiB), reclaiming 58,910,356 bytes (56.181 MiB).
Unreachable-object reclamation was zero. GC completed in 158,246 ms.
HEAD bytes, publication identities, full status and source-carrier hash are unchanged;
replacement publication and cleanup completed. The post-GC carrier test separately
reopens the maintained graph and exercises independent sources.

Canonical retained payload is 416,184,233 bytes. Including the existing 256-byte
per-object charge, GC retention uses 467,387,305 of 536,870,912 bytes, leaving
69,483,607 bytes. This preparation limit is independent of free disk and source
capacity. No history horizon, retirement deletion, identity change or reseed occurs.

The recovered focused run passed 113 selfhost and 10 native ELF tests with no
failures; the explicit same-carrier benchmark remains ignored. Selfhost test
time was 4,456.02 seconds; complete driver wall time was 4,696,710 ms.
Its complete input hashes were rechecked before compaction. Original observations
remain bound to their physical inputs; post-compaction acceptance uses the new tree.

Final acceptance requires staged-tree bootstrap, changed selection, every current
fresh full gate and release-target inspection. Terminal evidence lives in
`.artifacts/selfhost/accepted-e-source-effects/receipt.json` and component receipts.
No public release or deployment is part of source integration.

This extension is not E1-final. Match, witnesses, associated types, read views,
lexical Regions and nonempty R/E applications still need source admission.
Next choose a dependency-ready active-profile E2/E3 vertical slice; broader E1
families need not precede it. Audit the coupled envelope before capacity changes.
Full Tier 2 also requires full-profile independent programs, source-driven fixed
points, emitted-stage behavioral changes and measured same-process reclamation.
