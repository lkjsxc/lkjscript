# Structural operations and checked review

Text is the stored Git projection. The compiler addresses resolved source items,
checks a complete candidate, then publishes that projection through an
authenticated recovery protocol. These operations do not introduce a second
trusted program representation.

## Asking without reopening source

Build with `--index --store DIRECTORY --store-key KEY` and retain the printed
snapshot identity. Query with the same source/library selector and explicit
`--snapshot ID`. The selector identifies the build context; query never opens
the source tree. The version-three snapshot identity includes its producing
compiler's exact executable digest and authenticated container manifest. An
explicit pin authenticates a locator, the fixed manifest and requested node
extents without reading the current executable or the whole container. The answer
reports `producer=HEX`, `validation=authenticated-producing-compiler` and
`current-check=not-performed`: the facts were admitted by that producing compiler.
The query reader cannot turn historical evidence into a current checker proof,
image admission or gate reuse. Unpinned lookup and ordinary compilation keep
the existing current-producer authentication path.

Every answer says `freshness=not-asserted`. Without a pin, a valid old
latest-pointer record is possible; a historical explicit pin also does not
claim that the current source tree still matches it. Missing, incompatible or
corrupt pinned data refuses without falling back to another snapshot or producer.

Every fact begins with an identity and a level. Generic declarations, resolved
references and inferred definition types are `source-claim`. Monomorphic
operations, reachable effects, ownership cleanup and layouts are `checked-ir`.
An instance's source association is still a labelled producer claim. Native code
sizes are measurements of the selected emitter output, not a checker theorem.

`outline` returns signatures to a chosen depth. `item`/`text` returns one
authored item's exact source; `slice` adds the signatures it directly mentions and can
expand to a specified depth. `uses`, `references`, `callers`, `callees`,
`impls`, `instances`, `type`, `layout`, `authority` and `dependents`
have explicit identities, bounds and continuations. The entire reply, including
protocol and cursor, is subject to `--bytes`. A bound that cannot contain a
valid reply refuses before writing stdout. See the authenticated snapshot
contract in [IR](ir.md).

Signature text is trimmed at its ASCII whitespace edges and split at UTF-8
boundaries into fragments of at most 1,024 bytes. One-fragment signatures retain
their original fact spelling. Longer signatures add `chunk=` (zero-based,
16-digit hexadecimal) and `chunks=` (decimal total); `bytes=` is the full
signature byte count, and only the last fragment has `complete=true`.
Both signature and parent-outline queries include every fragment and continue
through the usual authenticated cursor. Fragment indices determine their order,
including when multiple fragments contain identical text.

A supported pinned snapshot retains the facts its producing compiler published.
Reading a historical pin does not replace or upgrade those facts. Version-three
readers support historical producers using the same format; version-two pins and
cursors refuse and require rebuilding the index.

Each snapshot owns one immutable-by-publication container. Its fixed authenticated
manifest binds the source/IR identities, root extent and complete physical length;
a query checks exact EOF, then reads and authenticates only the bounded node
extents its answer needs. The total file may exceed the reader's per-read limit:
that limit applies to individual manifest/node allocations. A retained readonly
descriptor does not seal the inode against another writer; already verified owned
bytes remain valid historical evidence, without a freshness claim.

Every new snapshot charges its full container to the store budget. It can
duplicate payloads retained by an earlier snapshot; unchanged reuse keeps the
same identity. The first writable opening retires exact old semantic page/root/
latest/locator names under the existing lock, while preserving unrelated signed
entries. This first migration still visits the old population. Old-format writers
must not share the directory with the new format.

## Semantic identities and selection

The root module is `crate`. Module-scope declaration IDs use resolved source
paths without an initial `crate::`, prefixed by their resolver namespace.
Modules, structs, enums, traits and aliases use `type@PATH`; free functions
and constants use `value@PATH`. A legal module/function pair is therefore
`type@shared` and `value@shared`. The marker qualifies the final path
segment. A function inside that module is `value@shared::value`, with
parent `type@shared`.

Descendants retain their owner's complete identity: `value@f::param:x`,
`value@f::body:EPOCH::call:0`, `type@Pair::field:a`,
`type@E::variant:A` and `type@Tr::type:Item`. Associated methods keep
their existing identities, such as `S::new`, `Tr::f` and `<S as Tr>::f`.
Implementation, use and macro identities also keep their existing forms; query
the outline for their exact spelling. Checked-IR instance and type IDs retain
`ir:PATH` and `ir:type:NAME`; their source associations name canonical
source identities. Type displays remain source type spellings.

A unit, tuple or fieldless braced struct occupies both resolver namespaces but
has one authored identity, `type@S`. It reserves `value@S` against a second
declaration. Constructor references target `type@S`; an explicit
`value@S` selector refuses and identifies that owner. Bare `S` uniquely
selects the struct. Same-namespace duplicate declarations remain invalid.

Edit identity selectors, including `id`, `parent` and `before`, and query
`--id` use the same selection rule. A selector containing `@` must have
exactly one `type@` or `value@` prefix and a nonempty remainder; it selects
only that canonical key. An absent or malformed explicit key refuses. A bare
selector is checked as its exact spelling,
`type@SPELLING` and `value@SPELLING`, with equal precedence. Exactly one
match succeeds. Multiple matches refuse with `E_EDIT_AMBIGUOUS` or
`E_QUERY_AMBIGUOUS`, listing canonical candidates and kinds. The operation
or query kind never chooses a namespace, and an exact-text `expect` does not
disambiguate a selector.

Query selection uses authenticated snapshot metadata without reopening source.
When a bare spelling selects a canonical identity, the report includes
`selected=CANONICAL`. Query cursors bind that canonical selection, so the
unique bare spelling and canonical spelling resume the same cursor. Historical
pins retain their producer's stored facts and exact bare keys; reading one
does not rewrite its identities or assert current freshness.

Physical file moves, whitespace and neighboring items do not determine
canonical identities. Adding an opposite-namespace sibling leaves existing IDs
unchanged, while making their shared bare selector ambiguous. A semantic path
rename or move changes the addressed path.

An anonymous `_` parameter uses `::param:POSITION`, where POSITION is its
zero-based position in the complete signature, including a receiver. For example,
`fn f(_: u8, named: i64, _: bool)` has `value@f::param:0`,
`value@f::param:named` and `value@f::param:2`. Numeric positions cannot
collide with legal parameter names. Anonymous IDs remain stable until signature positions
change; after adding, removing or reordering parameters, query the new outline.
`::param:_` is not an alias. Signature changes and inlining refuse a referenced
wildcard until it has an explicit named-parameter decision.

A binding or call site belongs to the token-normalized body epoch plus its local
ordinal. Reformatting and comments preserve that epoch. After a token change
there is no promise that a prior body-local identity still denotes anything.
Use the enclosing function identity for a whole-body replacement, or query the
new body first.

Default and Debug derives have semantic function and implementation identities, but their
expansion is not an independently authored source item. The catalog validates
the complete parser Item relation for functions and implementations, then
recognizes the exact resolved Default expansion of a defining struct and Debug expansions of structs or enums. Debug validation checks the canonical trait, copied generic contracts, field labels/order, borrows, exhaustive variant patterns, builder calls and fresh generated nodes. Unknown
orphan functions or implementations refuse with `E_IDENTITY_ORIGIN`; the loader's
injected module declarations are outside this narrower authorship rule.

Generated facts carry `origin=generated`, the `definition` identity and
`source-projection=false`. Their spans are labelled definition anchors.
Signatures describe the generated callable or implementation; `text` returns
`text-unavailable` instead of the defining declaration's bytes. Generated calls use
`FUNCTION::derived:EPOCH::call:N`; the epoch hashes the defining identity and
token-normalized declaration under `lkjscript-derived-default-body-1` for Default or `lkjscript-derive-debug-body-1` for Debug. The `expansion` fact names the producer. It is
distinct from a source-body epoch. Changing that declaration invalidates these
call IDs; whitespace and comments do not. There are no aliases for the old
erroneous source-span call IDs.

Generated reference edges retain their actual expansion owner and defining
identity in a separate uneditable collection. Shared authored field type nodes
and generic bounds remain ordinary editable references. Direct edits of a
generated item, including use as an insertion parent, refuse with
`E_EDIT_GENERATED_SOURCE`. A rename, move, signature change, inline or
removal that would rewrite a proven generated reference refuses with
`E_EDIT_GENERATED_REFERENCE`. Edit the defining declaration instead; candidate
elaboration regenerates and checks the expansion. Replay selects authored
declarations. Existing historical snapshots retain the producing compiler's
facts, including an older incorrect projection; rebuilding publishes the
repaired origin contract.

Debug-generated local bindings and enum patterns also remain outside authored spelling references. Explicit-argument formatting participates in ordinary function rename and body replacement. Implicit format captures retain a located `E_EDIT_REFERENCES` refusal when their decoded literal has no independently recorded editable byte span; the compiler does not guess an edit inside that string. [Debug query and mutation controls](../tools/debug-format-test.sh) cover both engines, unchanged source on refused transactions, and ten malformed generated-origin cases.

Declarative macros also expose source definitions, invocations and expansion
identities. A definition is named `macro-definition:N:PATH`, an invocation
`macro-invocation:N`, and its expansion `macro-expansion:N`. Definition and
authored-invocation queries retain their real text. Expanded declarations,
parameters, fields, patterns and call sites have `expansion=macro-rules`,
`source-projection=false` and an invocation anchor; they never present
transcribed bytes as writable source. Expansion references name both the
definition and invocation. Only reachable expanded expressions become reference
or call-site facts; unused arena copies are still classified as generated.

This classification is validated by reparsing the authenticated input byte map
into the exact original source program, then replaying token matching,
transcription, fresh opaque-capture materialization, immutable include reads and
name resolution. The complete resolved AST and topology receipts must agree.
The proof covers source scopes, export attributes, literal spelling, capture
contexts, parent expansions, allocation ranges and alias replacement. Replay
uses no ambient filesystem reads. These checks run while constructing an index;
bounded queries read the authenticated published facts. Missing or inconsistent
origin evidence refuses with `E_IDENTITY_ORIGIN`.

Macro expansion epochs hash the definition and invocation token streams along
the parent chain and the immutable included bytes. A same-length include change
therefore invalidates expanded call identities. A Default or Debug derive
created by a macro keeps its strict derive validator and identity, with the
macro's authenticated inputs added to its epoch. Authored Default and Debug
declarations retain their previous epoch construction exactly.

Checked authored function-body changes re-expand and recheck macro calls.
Direct generated-item changes refuse with `E_EDIT_GENERATED_SOURCE`;
renaming a helper or field with a proven generated reference refuses with
`E_EDIT_GENERATED_REFERENCE`. Unavailable targets remain partial claims.
[Macro origin controls](../tools/macro-origin-test.sh) cover indexed item,
expression and nested expansions, both engines, source-preserving refusals,
template/include epoch changes and 25 malformed-origin cases.

`references-complete=false` and unresolved-reference facts are explicit limits,
not evidence that an item has no uses. Declaration-only capture deliberately
omits body references and never permits a rename proof from their absence.
Compiler checkpoints can reuse this smaller capture. Ordinary and declaration-only
inference omit the two reference-export recording vectors while retaining binding
origins, constraints and closure capture. Full recording must be chosen before
inference; it cannot be promoted from empty vectors or a warm ordinary checkpoint.
A full typed index re-infers the missing evidence unless a separate authenticated
full snapshot can be reused. The frontend rejects a late full-export request with
`E_SEMANTIC_CAPTURE` before changing the caller's catalogs.

Associated calls such as `Widget::make()` and `Self::make()` publish an
`associated-call` source reference only from a successful frontend selector.
A declared trait contract can identify its member throughout an unused generic
definition. An inherent source selection is exported only when the calling
function has no type or const parameters, its enclosing implementation has none,
and it is not a trait-owned function; lifetime parameters do not vary selection.
Classification uses the declared function owner rather than an enclosing source
span. Agreeing observed generic instances do not establish a definition-wide
inherent target.

The member's exact terminal token is separate from its type/module/alias prefix.
Rename and signature operations can therefore use a proven member reference
without renaming an owner alias. Builtin, conflicting and unavailable selections
remain explicit gaps. A type argument list before the member, as in
`Boxed::<T>::make()`, currently prevents the terminal-token proof and refuses
refactoring. A trait default calling `Self::make()` also retains the existing
frontend constructor-routing refusal; this evidence extension does not change
language admission. `uses` and `references` include the source claims, while
`callers` and `callees` continue to describe checked monomorphic calls only.

## Type-outlives source identities

Named type-outlives bounds on admitted free functions and inherent methods retain
the type parameter and lifetime declaration identities. Function type summaries
show `where T: 'a`; signature queries retain the written inline or where form and
the ordinary trait bounds beside it. A `type-outlives-bound` reference selects
only the exact lifetime token. An enclosing implementation lifetime points to its
implementation binder, independently of a method's own lifetime parameters.

Checked body edits and function renames preserve those predicates. Replacing an
item to remove a bound needed by an opaque result fails complete candidate
checking, including for an otherwise unused generic body. A transaction containing
that invalid replacement publishes none of its operations. The original source
and authenticated snapshot remain reusable. These source claims do not add a
region opcode or change the independent checked-IR contract.
`tools/type-outlives-test.sh` exercises the scoped references, both execution
engines, rename/body edits, atomic bound-removal refusal and warm snapshots.

## Core facade source identities

Primitive `usize::MAX` and the other admitted integer limits resolve to ordinary
source constants. Their reference facts select the exact written `MAX/MIN`
token and use `alias-value` to retain that stable spelling. A lexically
shadowing module or alias selects its own declaration and ordinary reference
kind. Renaming that authored constant updates only its resolved references.

A named source type that uses existing array storage keeps its declared nominal
name in query keys and type identities. Structural array spellings still display
as arrays. Thus `PhantomData<u8>` and `PhantomData<String>` remain separate
source and checked-type names even though each has zero bytes and alignment one.
The representation ID continues to bind the actual checked type and layout;
the label does not establish an ownership, region or transfer proof.

Checked edits reconstruct the complete candidate and reject an array literal
where a marker is required. A changed library marker declaration invalidates
its prior authenticated source input. Cold/warm unit builds preserve the
nominal argument in admitted headers, reject mismatched arguments and recheck
a changed marker application. `tools/core-facade-test.sh` exercises these
facts together with source-signature queries, shadow renames, body edits and
atomic refusal.

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

The basic operations are `replace-body`, `replace-item`, `replace-call`, `add-item`,
`remove-item`, `rename` and `move`. Replacement source must parse as exactly
one body, one item or one expression as the operation requires, and is elaborated
in its real surrounding scope, including
visibility, generics and imports. `add-item` names a module; `before` is an
optional existing child identity. The basic move planner only changes position
within the same module. Refactor planners extend these forms; their supported
cases, evaluation order and explicit decision sites are in
[refactors](structure-refactors.md).

An `add-item` operation can also supply the complete source of one new external
module in a `projection`:

```json
{
  "version": 1,
  "operations": [
    {"op": "add-item", "id": "crate", "source": "pub mod added;",
     "projection": {"path": "src/added.lkj",
                    "source": "pub fn value() -> i64 { 42 }"}}
  ]
}
```

Here the entry is `src/main.lkj` and `--source-dir` selects the project root.
The path is normalized, relative to that root, and must be the loader's actual
`added.lkj` or `added/mod.lkj` candidate. The selected semantic parent and
physical parent directory must already exist. An empty module source is valid;
absence is distinct from an existing empty file. The declaration must contain
exactly one enabled external module. Creating directories, deleting source
files, adding a new parent module in the same request and spanning separately
checked roots remain explicit gaps.

The frontend records each successfully loaded module's identity, selected file
and alternate candidate. Inline modules use their nearest loaded ancestor.
Insertion into an empty external module uses that provenance rather than a
child-item guess. Both candidates become captured negative dependencies before
checking; the candidate must load the proposed file under the intended identity
and parent. An occupied alternate or a later change to either captured candidate
refuses. Creating an explicit library module promotes the transaction to full
library validation just as changing an existing library body does.

Renames and moves rewrite exact resolved declaration/use spans in the selected
namespace, leaving an opposite-namespace sibling intact. Import aliases retain
their local spelling. A single import spelling that binds both namespaces
refuses a one-namespace rewrite with `E_EDIT_REFERENCES`; the planner does not
infer a split. These operations must also preserve the checked-meaning digest.
In a mixed request, the rename/move subset is separately elaborated and checked for that postcondition before the complete candidate is
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

### Replacing one authored call

`replace-call` selects the existing `call-site` identity returned by a function's
outline, including its token-normalized body epoch and call ordinal. It replaces
that one authored call or method-call expression. It does not require the agent
to reproduce the enclosing function, and two identically spelled calls retain
distinct addresses.

```json
{"version":1,"operations":[
  {"op":"replace-call","id":"value@main::body:BODY_DIGEST::call:0",
   "source":"value + 2","expect":"CALL_ITEM_TEXT_DIGEST"}
]}
```

The optional `expect` is the selected call's `item-sha256` from `meta`, with the
same meaning as other operations. The request's optional `snapshot` still pins
the complete original input digest. A token-changing body edit invalidates its
old call IDs; a trivia-only edit retains them, but exact-text expectations remain
exact. All operations in a request address the original catalog, so two disjoint
calls can change together. Nested overlapping selections refuse before publication.

Replacement source must be one value expression. The raw source is parsed with
the existing expression grammar to the end of input, with no recovered errors,
before adding projection delimiters. Block-, `if`- and `match`-led arithmetic,
block casts and explicitly written tuples retain their value-expression meaning;
a bare comma list cannot acquire tuple syntax from the projection.
The projected bytes are an opening parenthesis, newline, supplied source, newline
and closing parenthesis. Parentheses preserve expression precedence and the
newlines isolate trailing line comments. Shape validation retains macro
invocations; the complete candidate expands and independently checks them in the
actual source scope. Replacement may intentionally change behavior and authority;
the ordinary plan reports both. It does not claim equivalence to the old program.

The current catalog must prove an authored call within an authored function body.
Generated macro/derive anchors remain uneditable. Some grouped callees or
receivers have an incomplete recorded written span, such as `helper)(x)` from
`(helper)(x)`; these refuse with `E_EDIT_CALL_SITE` rather than guessing surrounding
bytes. Replace the authored owning body in that case. This slice changes neither
parser span provenance nor snapshot identities.

Call IDs require full catalog capture. Checking still covers the whole original
and candidate program, with existing authenticated instance reuse and explicit
library promotion. The existing source validation, writer, intent, recovery and
precommit refusal rules apply unchanged. [Focused controls](../tools/structure-call-test.sh)
prove exact source bytes, checked-meaning agreement with an independent desired
tree, both execution engines and multi-file refusal atomicity.

## Replaying a checked target

`edit SOURCE --source-dir ROOT --against DESIRED_SOURCE` derives one request
from two independently checked source catalogs. `--against-source-dir` selects
the target's source Directory independently of the editable `--source-dir`;
its `--against` entry is resolved beneath that grant. `--against-library-dir`
selects the target's library; `--dry-run` prints the same plan without publication.
The source option requires `--against`; an unavailable explicit Directory refuses
without falling back to the working directory. Use explicit source/library roots
for both trees and keep the private key outside every one of those grants.
Without the option, the existing entry-based source selection still applies:
a relative desired entry uses the current directory, an absolute entry uses `/`.
Different include-path spellings can still make the conservative file pairing
refuse; the option does not infer aliases or broaden the correspondence rule.
A request file and `--against` are mutually exclusive. Existing declarations are
paired by stable source identity and source files by an unambiguous bijection.
Include-only inputs inherit the longest directory correspondence proved by
those item pairs; ambiguous mappings refuse.

The planner chooses body/item replacement, addition or removal. It preserves
untouched source bytes. A scoped replay can introduce a separately selected new
external-module declaration beneath an existing loaded parent, supplying its
complete body as the same explicit projection form. Empty existing modules are
paired from loader provenance. Other new files, changed imports or unsupported
module framing, existing declaration reordering, splits/joins and changed
include-only data still refuse. A new physical directory or new semantic parent
is not inferred. Repeated reads of one path must contain identical bytes.
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
The loader records embedded origin before assigning diagnostic paths. A physical
`include_bytes!` input with the same displayed path cannot turn embedded code
into editable source, even when its length and all later spans match.

One writer holds `.lkjscript-edit.lock`. Preparation retains the project root
and each existing output parent as Directory capabilities. A changed slot is the
parent's observed device/inode pair plus its basename; two path spellings of the
same slot refuse before any intent. Equal basenames in distinct parents are valid.
These identities describe held kernel objects, not semantic or image identities.

The compiler validates original and newly discovered candidate dependencies
against those retained parents. A differently spelled input alias of a changed
slot refuses, including a previously absent library probe. Exact physical input
spelling is required for the byte overlay; normalization does not manufacture
that correspondence. Final input leaves are opened with `open_regular`, so a
symlink, FIFO, socket or directory cannot substitute for a regular source input.
Ordinary build/query source loading is unchanged by this transaction rule.
Expected-present reads are bounded by the captured length and require exact EOF.

Source inputs cannot occupy a changed leaf `NAME`'s staging basename
`.NAME.lkjscript-next`, or the root `.lkjscript-edit.lock`,
`.lkjscript-edit.pending`, `.lkjscript-edit.generation`,
`..lkjscript-edit.pending.lkjscript-next` and
`..lkjscript-edit.generation.lkjscript-next` slots.
`E_EDIT_RESERVED_INPUT` identifies that refusal; differently spelled aliases of
changed source slots use `E_EDIT_PARENT_ALIAS`. Checks use parent identity as
well as basename, including inputs discovered only while checking the candidate.

The authenticated version-two intent encodes old state as explicit `null`
(absent) or text (present, including empty text); missing or malformed old state
refuses. Version-one intents are unsupported and have no migration path.
Recovery authenticates the fixed envelope before allocating or decoding its
payload. It then validates every affected leaf as absent, exact old bytes or
exact new bytes, as applicable, before writing any projected source. Only
`ENOENT` beneath an already-held existing parent establishes creation absence.

After checking all source and dependency preconditions, the writer syncs the
authenticated `.lkjscript-edit.pending` intent. That durable intent is the
commit point. Source staging starts only after it. Each output uses a private
same-parent temporary and file fsync; the generation stamp is published and
synced before the first source leaf. Replacements use atomic rename. Creations
use `rename_new`, which refuses an occupied destination without overwriting it
and has no fallback when the host lacks the atomic operation. Each affected
parent is synced. Once all new files are durable, stages and intent are removed
and the root is synced.

Pending/generation reads open existing regular leaves without creating them or
opening special leaves for data. Recovery without an intent cleans only the two
known root metadata staging names; it does not delete arbitrary per-source
stages. The generation stamp is a reader fence, not an authentication proof.
Participating compiler reads reject a pending transaction and bracket compilation
with the relevant generation stamps; stored-image publication uses that fence too.

```sh
lkjscript edit src/main.lkj --source-dir . --recover \
  --store-key /private/compiler.key
```

After a killed writer, authenticated recovery finishes the committed new
projection; before a durable intent, the old projection remains. An error after
that commit point requires recovery and does not promise rollback. Multiple
filesystem renames are not a simultaneous OS transaction. Uncoordinated readers,
including Git, can observe an intermediate mixture until recovery completes.
Participating compiler reads reject that state. Use a consistent project scope
for writers/readers; the compiler fences the selected project root, entry-file
parent and explicit library root, not arbitrary independently managed nested
projects. Precommit refusals do not publish candidate source; reserved
lock/recovery metadata is outside the tracked projection. A dry run uses the same
planning, candidate-checking and physical-precondition path without a
source-transaction lock or publication; configured store caching may still write
its disposable cache. Apply additionally requires a signing key, excludes it from
source inputs and performs authenticated recovery. A key-free dry run refuses
pending state instead of recovering it.

The protocol assumes cooperating writers keep the parent namespace stable.
Retained handles keep writes on the acquired directory objects; they do not
authenticate the continued pathname-to-parent mapping. Recovery after a crash
reacquires those parents. Arbitrary parent renames or changes by nonparticipating
processes are outside this contract. No global filesystem snapshot or stronger
multi-directory atomicity is claimed.

## Comparing checked meaning

`compare SOURCE --against OTHER_SOURCE` checks both source trees.
`--against-source-dir` selects the other source Directory with the same rules
as replay; `--against-library-dir` selects the other library closure. Default output is
two summary lines, at most 27 fact lines, and one continuation/detail line.
`--id ID` selects detail; a larger explicit `--limit` can request more facts.
The bounded summary samples authority, source declarations, representation and
instances in deterministic rounds. Generated IR names follow named facts
within their category. The final line counts every category, including omitted
facts; presentation order never changes an identity or proof. Authority facts
state the gained/lost effects and declared capability slots explicitly, plus entry
presence on both sides. Both host-grantable entries and the authenticated child
transfer profile participate; Channel, File and ReadFile parameters do not vanish
from comparison merely because the host CLI cannot create those grants.

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
