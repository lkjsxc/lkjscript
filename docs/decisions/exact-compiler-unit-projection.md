# Exact compiler-unit projection

## Boundary

An independently checked semantic owner is not necessarily a compilation unit.
Ports carry exact callable contracts but compile inside their component. HTTP
routes belong to targets. Expressions and bindings likewise compile through an
enclosing declaration or target. Module membership and presentation attachments
must not make a module-sized compilation unit.

The impact owner computes the union of the first declaration/target reached in
the before and candidate ownership paths. A newly created child has no before
path; a deleted child has no candidate path. Absence terminates that particular
path. It never promotes the child itself to a unit or borrows a record from the
other snapshot to decide membership. A moved child contributes both enclosing
units; a removed declaration remains a unit-removal request.

Issue #9 exposed the opposite behavior: on the missing before path of a newly
created port, the planner consulted the candidate canonical record and inserted
that port into compiler impact. The correct compiler cache guard then rejected
`compilation_incremental_owner_domain`, even though semantic publication had
succeeded. Adding a target was not necessary. The regression isolates exact
allocated Port IDs, not just an unexplained difference in counts.

## Selection

Keep compiler-unit projection dependent only on revision-specific ownership,
not on a candidate canonical-record lookup. Preserve both before and candidate
traversals, cycle detection, exact owner identities and the ownership-step bound.
Do not filter illegal units inside the compiler cache. Its strict domain, kind,
parent-manifest and publication-evidence checks remain unchanged.

A direct semantic change or incoming validation dependency needs the same unit
projection for its semantic and compiler sets. Compute that projection once and
insert its roots into both sets. Keep the directly checked child in the semantic
set: validating a component is not a substitute for checking a port's callable
contract. Dependency-binding and behavior/test propagation retain their own
traversals and obligations.

Charge each traversal step actually performed. A normal unchanged port-to-component
projection visits four positions across its two snapshots. The shared insertion
uses those four positions once instead of repeating the same projection for the
two destination sets. This is a local work reduction, not a measured whole-edit
speedup or a claim that all impact traversal is deduplicated. Limits are not raised.
No reusable cross-operation cache, new trusted intrinsic, semantic encoding or
artifact encoding is introduced.

## Evidence obligations

Source regressions compare the exact planned unit identities, ordinary incremental
build counters and complete manifest/artifact bytes against a separate clean build.
They cover one/two-port components with and without a Command target, an unrelated
reused function, malformed compiler-impact rejection and unchanged authority.
A separate accepted lifecycle adds, rebinds, moves and deletes a port, then removes
its former component. A moved port keeps its identity while both parents change.
Empty components remain invalid; an initially mistaken zero-port positive fixture
must not be counted as a product regression.

Isolated projection tests enumerate all 729 before/after arrangements of three
children assigned to neither, either of two roots, for both port/expression and
HTTP-route families. Compare each complete result with forward descendant closure
from each root, not a second implementation of the parent walk. These synthetic
read maps establish projection laws, not admission of 729 complete programs.
Separate exact-fit/one-below limits and cycles check bounded failure.

Copied-product tests perform ordinary native authoring on Command projects. They
require accepted edits, `derived-cache status=updated`, subsequent
`compilation cache=exact-current`, unchanged template test outcomes and complete
Command results both attached and after deleting the authoring project. A wrong
port contract rejects without changing project contents, followed by a valid edit.

Final-candidate acceptance generation 6 requires the three exact behavior tests
under `native_declarations::incremental_units::` and selects every future case in
that namespace. Missing, substituted, ignored, failed, duplicated and unrun cases
reject. Existing Map, fold and other native families remain required. Generation 5
retains its own historical obligations; neither an old acceptance terminal nor
mixed contract/workload generations establish this new boundary.

## Long-term consequence

Keep semantic validation, dependency impact and code-generation granularity
separate as compiler passes move toward native lkjscript. A native successor must
preserve these ownership and failure laws rather than inherit Rust's physical
record layout or silently broaden rebuilds. Match preparation, edit and execution
costs on complete consumers before selecting further caching or finer units.
[Status](../status.md) owns accepted source and executable availability; this
selection alone is not evidence of mainline delivery or public distribution.
