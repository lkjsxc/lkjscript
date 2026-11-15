# Structural operations and checked review

Text is the stored Git projection. The compiler addresses resolved source items,
checks a complete candidate, then publishes that projection through an
authenticated recovery protocol. These operations do not introduce a second
trusted program representation.

## Asking without reopening source

Build with `--index --store DIRECTORY --store-key KEY` and retain the printed
snapshot identity. Query with the same source/library selector and explicit
`--snapshot ID`. The selector identifies the build context; query never opens
the source tree. Without a pin, a valid old latest-pointer record is possible,
so the answer explicitly says `freshness=not-asserted`.

Every fact begins with an identity and a level. Generic declarations, resolved
references and inferred definition types are `source-claim`. Monomorphic
operations, reachable effects, ownership cleanup and layouts are `checked-ir`.
An instance's source association is still a labelled producer claim. Native code
sizes are measurements of the selected emitter output, not a checker theorem.

`outline` returns signatures to a chosen depth. `item`/`text` returns one
item's exact source; `slice` adds the signatures it directly mentions and can
expand to a specified depth. `uses`, `references`, `callers`, `callees`,
`impls`, `instances`, `type`, `layout`, `authority` and `dependents`
have explicit identities, bounds and continuations. The entire reply, including
protocol and cursor, is subject to `--bytes`. A bound that cannot contain a
valid reply refuses before writing stdout. See the authenticated snapshot
contract in [IR](ir.md).

Module and declaration IDs follow resolved source paths, without an initial
`crate::`; the root module is `crate`. Fields use `::field:NAME`, variants
`::variant:NAME`, parameters `::param:NAME`. Query the outline for an
implementation's displayed identity. Physical file moves, whitespace and
neighboring items do not determine these identities. A semantic path rename or
move changes the addressed path.

A binding or call site belongs to the token-normalized body epoch plus its local
ordinal. Reformatting and comments preserve that epoch. After a token change
there is no promise that a prior body-local identity still denotes anything.
Use the enclosing function identity for a whole-body replacement, or query the
new body first.

`references-complete=false` and unresolved-reference facts are explicit limits,
not evidence that an item has no uses. Declaration-only capture deliberately
omits body references and never permits a rename proof from their absence.
Compiler checkpoints can reuse this smaller capture; a full typed index must
obtain the missing resolved/inferred evidence.

## One checked request

```sh
lkjscript edit src/main.lkj --source-dir . --library-dir library \
  --request changes.json --store-key /private/compiler.key --dry-run
lkjscript edit src/main.lkj --source-dir . --library-dir library \
  --request changes.json --store-key /private/compiler.key
```

```json
{
  "version": 1,
  "operations": [
    {"op": "replace-body", "id": "math::increment", "source": "{ value + 2 }"},
    {"op": "add-item", "id": "math", "before": "math::next",
     "source": "pub fn twice(value: i64) -> i64 { value * 2 }"}
  ]
}
```

All operations name the same original snapshot. They are planned together;
later operations do not look up a partially renamed or moved tree. Nonoverlapping
edits and independent insertions share one candidate. Distinct overlapping
writes refuse; insertions at the same boundary retain request order. Untouched
item bytes are copied exactly. Removing an item also removes its otherwise
empty indentation and trailing line break; detached comments remain text.

The optional request `snapshot` is the source-input digest printed as
`source=`, not the immutable query snapshot ID. Each operation can carry
`expect`, the exact item's text digest from its metadata. Stale bytes, ambiguous
scope, incomplete required references and overlapping writes produce
identity-addressed `E_EDIT_*` refusals.

The basic operations are `replace-body`, `replace-item`, `add-item`,
`remove-item`, `rename` and `move`. Replacement source must parse as exactly
one body or one item and is elaborated in its real surrounding scope, including
visibility, generics and imports. `add-item` names a module; `before` is an
optional existing child identity. The basic move planner only changes position
within the same module. Refactor planners extend these forms; their supported
cases, evaluation order and explicit decision sites are in
[refactors](structure-refactors.md).

Renames and moves rewrite exact resolved declaration/use spans. Import aliases
retain their local spelling. These operations must also preserve the
checked-meaning digest. In a mixed request, the rename/move subset is separately
elaborated and checked for that postcondition before the complete candidate is
accepted. This conservative condition can reject a request whose relocations
would only become valid after other operations; split or restate it explicitly.
Moves require the full reference catalog, including outgoing references in the
relocated item; declaration-only capture cannot establish that proof.

A transaction that touches an explicit library file promotes validation to all
library definitions, including bodies the selected program never calls. The
original, rename/move subset, candidate and any replay target use that same
mode. Promotion retains the writer lock, parsed request and pinned source read
sets; it never rereads a different request or silently changes the original
source snapshot. The report states `library-validation=all` or `demanded`.
This conservative whole-library pass can refuse an already-invalid unused body
and adds checking cost until definition-level validation is available.

The plan names changed identities/files and sites, authority gained/lost per
entry, original/candidate checked meaning, fresh/reused instance counts and
frontend resumption. Dry run performs candidate elaboration and checking, but
does not publish source. Body/item replacement may intentionally change
behavior; successful type/ownership checking is not a claim of behavioral
equivalence.

The implementation currently captures and checks a whole source program on both
sides. Smaller declaration catalogs remove reference collection from body/item
replacement, but do not make its total cost independent of program size.
Recorded signature dependents are conservative source relations, not yet a
complete per-definition interface reuse boundary. Scale costs and the first
resource refusal are in [the ladder](../bench/scale.md).

## Replaying a checked target

`edit SOURCE --source-dir ROOT --against DESIRED_SOURCE` derives one request
from two independently checked source catalogs. `--against-library-dir`
selects the target's library; `--dry-run` prints the same plan without publication.
A request file and `--against` are mutually exclusive. Existing declarations are
paired by stable source identity and source files by an unambiguous bijection.
Include-only inputs inherit the longest directory correspondence proved by
those item pairs; ambiguous mappings refuse.

The planner chooses body/item replacement, addition or removal. It preserves
untouched source bytes. Changed imports or module framing, existing declaration
reordering, new files, splits/joins and changed include-only data require
explicit operations. Repeated reads of one path must contain identical bytes.
Unchanged binary data is compared as bytes, never tokenized as source.

Before publication the candidate's checked-meaning digest must equal the
desired program's digest. The report prints both and `verified=true`; both
input read sets are validated again. This proves equality under the same
current checker and selected libraries. It does not claim to reproduce the
output of a historical compiler producer. The history harness freezes its
revision population and compiler bytes, records compatibility environments
and refusals, and applies explicit external memory/time limits.

## Publication and recovery

Apply/recovery requires a private 32-byte authentication key outside every
source/library read grant. The same private-key source-digest exclusion used
for child images applies during elaboration. Embedded library text can be
queried, but editing it requires an explicit editable `--library-dir`.

One writer holds `.lkjscript-edit.lock`. The compiler validates all original
source bytes and the actual positive/negative dependency read set, then syncs
an authenticated `.lkjscript-edit.pending` intent containing old/new bytes.
That durable intent is the commit point. Recovery validates its fixed
authentication envelope before allocating/decoding the payload, and verifies
that every affected file is still exactly old or new before writing any file.

The generation stamp is published and synced before the first projected source
file. Each source replacement uses a private same-directory temporary, file
fsync, atomic rename and parent-directory fsync. Once all new files are durable,
the intent is removed and its directory synced. The generation stamp is a
reader fence, not an authentication proof. Ordinary compiler reads reject a
pending transaction and check the relevant generation stamps before and after
compilation; the stored-image fast path also checks them before publication.

```sh
lkjscript edit src/main.lkj --source-dir . --recover \
  --store-key /private/compiler.key
```

After a killed writer, authenticated recovery finishes the committed new
projection; before a durable intent, the old projection remains. Multiple
filesystem renames are not a simultaneous OS transaction. Uncoordinated readers,
including Git, can observe an intermediate mixture until recovery completes.
Participating compiler reads reject that state. Use a consistent project scope
for writers/readers; the compiler fences the selected project root, entry-file
parent and explicit library root, not arbitrary independently managed nested
projects. Refusals do not publish candidate source; reserved lock/recovery
metadata is outside the tracked projection.

## Comparing checked meaning

`compare SOURCE --against OTHER_SOURCE` checks both source trees.
`--against-library-dir` selects the other library closure. Default output is
two summary lines, at most 27 fact lines, and one continuation/detail line.
`--id ID` selects detail; a larger explicit `--limit` can request more facts.
The bounded summary samples authority, source declarations, representation and
instances in deterministic rounds. Generated IR names follow named facts
within their category. The final line counts every category, including omitted
facts; presentation order never changes an identity or proof. Authority facts
state the gained/lost effects and grants explicitly, plus entry presence on
both sides.

Source signature/body/text changes are labelled claims. Absolute checkout paths
are excluded from that summary. Text equality compares exact bytes under the
same unambiguous loaded-file correspondence used by replay; transaction snapshot
digests still bind paths for stale-input protection. Checked facts
compare entry authority, physical layout, nominal type representation,
destructor binding and each monomorphic instance body. Whole-program identities
and the explicit verdict `text differs, checked IR identical` are separate
from diagnostic-metadata equality.

The checked-meaning projection erases diagnostic paths, positions and labels.
It retains operation order, literal/trap bytes, type/ownership contracts,
control flow, calls and destructors. It is an equality proof, not a general
equivalence solver. Whole-program arena ordering remains conservative: some
equivalent moves can report different checked IR. Admission and stored proof
reuse continue to use their exact domains; a review digest alone never admits
an untrusted program.

Type/layout queries expose `representation-id` in domain
`lkjscript-ir-type-identity-linux-x86_64-1`. It binds canonical nominal names,
field/variant order and types, copy rules, actual target layout and cleanup
requirements. Cyclic type components are encoded once; dependency digests make
unrelated arena renumbering irrelevant. Equality is a representation contract,
not a wire codec, transfer permission or identity of a destructor's implementation.
Programs must choose and check the separate message/transfer contract.

Instance digests normalize local type IDs through type-component fingerprints
and call IDs through displayed instance identities. They compare the admitted
body itself; called-body changes appear at the called instance. Review reports
are not an assertion that arbitrary source-to-instance associations were checked.

## Closed test reuse

Fresh `authority` and `build-entries` reports include a diagnostic-preserving
checked closure digest: entry, reachable calls, referenced type graphs and owned
destructors under canonical local numbering. Unrelated declarations cannot
renumber that closure. Conservative extra references can cause misses.

The closed gate requires this digest in addition to the same exact sealed ELF,
stable runner, authority report, gate script, selected entry, case count and
fixed argument/input/output contract. The authenticated record includes all of
them. A source-body change outside the closure can reuse a freshly checked,
identical closed image. A changed compiler still checks fresh IR; no compiler
admission verdict crosses producer generations. Runtime or emitted-byte changes
still invalidate the execution proof even if the checked closure is unchanged.
