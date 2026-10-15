# Checked native package dependency plans

`project/` owns the accepted editable program. It imports an independently
exported checker and planner, together with their exact worklist and indexed
storage suppliers. `generated/package-plan.lkja` is derived through ordinary
public `build`; its selected pure Command target is `package-plan`.

The checker was authored, checked and exported before the producer project was
created. It checks the original proposal and the producer's complete outcome with
its own connectivity and stage predicates. The consumer returns the exact checked
valid, invalid or capacity claim. A failed check becomes `inconsistent`; it cannot
expose an unchecked successful plan. Integer and Boolean stage storage both select
Flat, and the worklist selects FlatCells.

The maintained BlockRead implementation keeps the actual `storage-flat-flat` native
entry and forwards `indexed-flat::Blocks<I64>` beside its integer IndexedData
witness. Its levels map uses the shared scoped block traversal described by the
[planner](../../examples/dependency-plan/README.md), preserving ascending positions
and omission of zero levels. All four public integer/Boolean storage choices use
the same materializer. The generic stage core is authored before its independent
storage suppliers, then receives the default Flat wrapper.

`requests/blockread-adoption/prefix/` and `requests/blockread-adoption/finish/`
retain the current admitted literal requests, original base revisions and exact
supplier bindings. Prefix requests author the generic stage core before its
independent storage suppliers, then append the explicit default Flat wrapper.
Finish requests author/export the checker before the producer and bind the
checked consumer to queried exact declarations. These requests are provenance;
the accepted editable project remains the program-meaning authority. The
[original provenance](requests/blockread-adoption/provenance.json) binds all 27
requests, exact suppliers, accepted project and public proof.

The original files directly under `requests/` remain provenance. They include the
`entrypoint.lkjc` declaration body and admitted `compiler-entrypoint.lkjc`. The admitted
`checked-consumer-exact.lkjc` uses queried exact owners because exported names
`Outcome`, `flat` and `chunked` are ambiguous. The original refused
`checked-consumer.lkjc` and its diagnostic remain preserved; that proposal never
published an accepted revision. Edit current meaning with public canonical drafts
and rebuild after applying a reviewed change.

```sh
lkjscript --project tools/native-package-plan/project check
lkjscript --project tools/native-package-plan/project build --output rebuilt-package-plan.lkja
lkjscript --project tools/native-package-plan/project change draft \
  --module package-plan --output edit-package-plan.lkjc
```

An untouched draft plans unchanged. A maintained rebuild must match the selected
artifact's exact bytes while the accepted project and suppliers remain unchanged.
No external semantic generator, graph writer or host-produced plan participates.
Schema changes require public check/build to regenerate `generated/package-plan.lkja`.
Preserve exact request/provenance originals; rebuilding does not supply fresh source,
runtime or finalized-byte acceptance. [Status](../../docs/status.md) owns those outcomes.

The target takes one ordinary Proposal positional argument:

```json
[{"roots":[0],"nodes":[
  {"id":0,"successors":[1]},
  {"id":1,"successors":[]}
]}]
```

Its complete valid outcome is:

```json
{"case":"valid","value":{
  "components":[
    {"members":[0],"cyclic":false,"dependencies":[1]},
    {"members":[1],"cyclic":false,"dependencies":[]}
  ],
  "stages":[[1],[0]],"reachable":[0,1],"unreachable":[]
}}
```

All nodes and edges participate in admission, including unreachable ones. Output
components and peers retain authored identity order; dependency stages precede
users and use the earliest complete stage assignment. Cyclic components are
reported in valid graph-analysis outcomes. The compiler boundary maps those to
package-cycle refusal and maps ordinary indices back to exact revision identities.
Stages grant no execution, parallelism or publication authority.

The shared native validator admits 10,000 nodes, 10,000 roots and 100,000 successor
entries inclusively; repeated entries count. It saturates an excessive edge count
at 100,001 before addition. Capacity precedence is nodes, roots, edges, followed by
duplicate identities, missing roots and missing successors in authored order.
The independent checker uses the same exact validator cut through its own
exported package.

The compiler supplies a separate finite JSON embedding envelope for complete
maximum-capacity projections. Ordinary CLI JSON defaults remain unchanged.
Raw type admission, runtime value limits, source closure limits, cancellation,
cleanup and output encoding each retain their own boundary. A native capacity
bound does not promise that every maximum-sized package body fits all other limits.
There is no host topology fallback or cumulative instruction quota for trusted
foreground planning.

Public source acceptance, copied-product adoption, compiler-envelope boundaries,
matched costs and finalized-byte verification remain separate required evidence.
The [matched public build comparison](../../docs/decisions/native-package-planning.md#observed-public-build-costs)
retains the measured slowdown on tiny, diamond and maintained-planner projects;
native ownership does not imply lower compilation cost.
See the [independent checker](../../examples/dependency-plan/verification/README.md)
and [planner](../../examples/dependency-plan/README.md) for the native algorithm and
complete-outcome contracts.

The original provenance retains its exact current_requests and runtime_scope fields;
asset regeneration does not rewrite their original bytes, source or counts.
