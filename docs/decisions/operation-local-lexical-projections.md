# Operation-local lexical witness projections

The demanded recursive callable proof repeatedly resolves an implementation
parameter's authored ordinal within its exact lexical declaration. Keep a small,
lazy projection of ordered parameter IDs for each exact package/declaration pair
within that proof. This extends [incoming call input sharing](operation-local-callable-inputs.md),
not the language's witness selection rules. [Status](../status.md) owns acceptance.

## Boundary

`Demands` owns the projection table. Its immutable reader is already bound to one
complete source closure, and the table ends with this proof operation. The key
includes both package and declaration identity. Equal parameter IDs, matching
contract shapes, other methods and new source operations do not merge lexical
owners. Each request still checks its explicit scope and forwards its own exact
witness path and type slots.

A cold lookup reads the declaration through the existing source owner and resolves
the requested ordinal with the original routine. Only after successful lookup,
modeled storage reservation, ordered copying and the final cancellation/work
checkpoint does it publish the complete ID vector. A missing declaration or
parameter, cancellation or capacity refusal cannot publish a partial entry.
The projection retains neither a whole declaration nor a semantic-validity proof.

A warm lookup retains cancellation, bounded work and ordered ID comparisons. It
returns the first matching authored ordinal, preserving the existing local lookup
behavior even in raw duplicate-ID fixtures; complete source admission still owns
uniqueness. Missing-parameter diagnostics remain on the original path. No source,
interface, type, ownership, effect or callable-closure check is removed.

## Evidence and tradeoffs

The maintained incoming-call fixture measures only the demanded recursive proof
phase. At widths 1, 4, 16 and 64, the predecessor reads its lexical declaration
2, 8, 32 and 128 times; this projection reads it 2, 5, 17 and 65 times. Incoming
mapping reads remain one, type reads remain 2, 5, 17 and 65, and exact graph slots
and edges remain unchanged. Other lexical lookups are deliberately not covered by
this table.

At width 64, modeled work increases from 56,474 to 66,068 and modeled metadata
from 231,413 to 234,573 bytes. These costs are retained, not relabeled as a total
speedup. Source-record reads, modeled work, retained storage and wall time are
different quantities. This change avoids repeated whole-record clones at one
boundary but does not establish faster complete preparation or execution.
The separately maintained local lookup comparison executes the literal predecessor
operation and the projected operation over identical ordered queries. It records
both timing directions without a timing-based pass criterion or a host-wide claim.

Tests cover equal IDs in different packages and lexical owners, changed source in
a new operation, all cold cancellation checkpoints, warm cancellation, exact and
one-short work/metadata limits, original missing-input diagnostics and complete
ordered lookup results. Existing eager-vs-demanded recursive proof tests remain
independent semantic oracles, including expanding types at every witness position.

No canonical meaning, interface, artifact or runtime encoding changes. No stored
program migration is required. Finite preparation costs and therefore a refusal
at a narrowly selected work limit can change; resource refusal is not semantic
invalidity. The remaining declaration/type reads and large genuinely demanded
path sets require separate measurements before broadening reuse.
