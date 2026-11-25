# Authenticated checking units

A unit is an explicitly named group of source modules. `check-units` reads,
hashes, elaborates and independently checks one unit at a time.
`build-units` also emits its native object and streams it into an executable.
The existing `check`/`build` routes retain their whole-program contract.

## Declaring the boundary

A manifest contains exactly these literal declarations:

```rust
const UNITS: &[&str] = &["math=math.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["app=math"];
const ENTRY: &str = "app::main";
```

The declarations are unconditional literals and may appear in any order.
Ordinary and raw UTF-8 strings use the language's existing escape semantics.
Nested comments, trailing commas, parenthesized types and values, reference
lifetimes and visibility within the root crate are accepted. Attributes and
`cfg` are refused with a diagnostic: select a separate manifest for each
configuration. Computed constants, macros and other declarations are refused.

Unit names are identifiers; sources are relative `.lkj` paths beneath
`--source-dir`, without `..`. Dependencies must form a DAG. Mutually
dependent modules belong to the same unit. The compiler repeatedly selects the
lexically smallest ready unit, including units made ready by the preceding
selection. It constructs the complete schedule and refuses cycles before
validating ENTRY or checking any unit source.

Manifest capture streams UTF-8 validation and a complete source digest before
the lexer can report input text. An exact private-key input is excluded first.
Raw chunks, decoded token chunks and literal rows live in an authenticated
command-local disk store. Only one lexeme or literal is decoded at a time.
Individual definition, path, dependency and remaining-count records feed the
schedule. Initially ready units form a sorted disk stream. The scheduler merges
its head with the minimum of a disk heap for newly ready units. Completed
ordinal rows are bound to each unit's stored rank. A caller restores one unit's
definition and direct-dependency list. Diagnostic paths reuse that active
definition and restore an imported unit's source root once per checking unit. Final
authentication includes the complete scratch file and exact inventory, followed
by a streamed rehash of the original manifest before publication. There is no
manifest-size or nesting policy cap.

Each unit source is mounted as a module bearing its unit name. For example,
`math.lkj` can declare `pub fn twice(value: i64) -> i64 { value + value }`,
and `app.lkj` can call `crate::math::twice(21i64)`. Existing module-local
paths keep their meaning; an old crate-root path to a unit's own child must
now include the unit name, or use an appropriate relative path. This is an
explicit source boundary, not automatic decomposition of an existing crate.

```sh
key=$(sh tools/store-key.sh)
target/lkjscript check-units units.lkj --source-dir example \
  --store target/unit-store --store-key "$key" --stats
target/lkjscript build-units units.lkj --source-dir example \
  --store target/unit-store --store-key "$key" --grant out \
  -o target/example --stats
```

The second example assumes `app::main(out: Stdout)`. Normal exact entry
grant checking applies. The manifest selects the entry; `--entry` does not.
A missing or cyclic manifest, failed unit, missing grant or failed link leaves
the previous executable intact. Publication uses a new temporary file and an
atomic rename only after linking finishes and every source input and the manifest
are rehashed. A command records the selected units' input claims by directory and
path. Conflicting claims refuse publication even if a file changes back before
the final sweep. Repeated cache lookups may share an observation within that
command; fresh admission hashes the bytes it actually loads, and the final sweep
always reads again. Failed cache candidates cannot supply fresh input identities.

Selected records and the complete input-claim set live in separate command-local
authenticated disk stores. Only the selected authenticated projection supplies a
retained record's requirement list. The adjacent units' observation and
selected-claim caches remain in RAM. Selected claims persist in scratch and
participate in conflict checks after an observation leaves the cache. Both
complete scratch files are verified before publication. Scratch keys bind paths
to ordinals for the invocation's exact source and library directories. The final input sweep
walks the selected-claim index in key order, authenticating each row and
rereading its corresponding source input. The walk retains only ancestor
locators, the previous key and one current row.

## What an interface proves

`CheckedUnit` and `Interface` are separate from `CheckedProgram`. A bodyless
declaration is never fabricated into a whole-program proof. The independent
unit checker verifies every local body and every imported contract before it
can construct `CheckedUnit`.

The stored interface contains exact normalized types, native layouts, Copy
and ownership structure, destructor bindings, function parameter/result
types, mutability, returned-view origin contracts and reachable authority.
It also binds the target ABI, defining-unit linkage identities and the
dependency-interface closure. Type and function indices are remapped and
compared structurally; equal names alone are insufficient.

The interface's semantic identity excludes local bodies and source hashes.
Its separate admission payload binds the exact input, checked body and
dependencies. Store authentication binds the exact compiler producer, private
key domain, namespace and lookup key as well as payload bytes. The private
`ProofDomain` and `VerifiedPayload` types preserve those bindings; a caller
cannot substitute a boolean claiming verification.

A wrong producer, key, unit, input, dependency, namespace, payload or framing
cannot grant admission. A damaged cache entry is discarded and independently
checked again when its source is available. When a dependent needs a record
that the command has not already authenticated and retained, eviction or failed
authentication refuses that read. No unsigned declaration replaces it. A
requirement list already obtained from an authenticated projection remains valid
in command scratch after its cache entry is evicted. Interface and projection
reads still require their own authentication. Repeated imports of a genuine reexport are accepted
only when their defining binding, exact signature and effects agree.

Interface restoration reads the framed IR directly from its bounded payload
span. It still verifies structure, origins, layouts, effects, exact EOF and the
complete canonical encoding. Only then does it hash canonical spans with the
provenance texts and dependency vector replaced by their encoded empty values.
The original semantic encoder and checked-interface constructor remain separate
identity oracles; the interface format and semantic identity are unchanged.

## Source declarations and generic templates

Dependents load authenticated declaration projections, not the original
nongeneric bodies. Free functions and methods of nongeneric implementations
have opaque declarations and exact internal linkage bindings. Private helpers
can therefore be called by a generic instantiation without copying their bodies.
Internal linkage is distinct from source visibility: the resolver still rejects
a caller's illegal access to a private item.

Generic functions, generic implementations and trait defaults remain untrusted
source templates. The dependent elaborates and independently checks each
demanded instance. Changing a template therefore invalidates affected consumers.
Includes inside retained templates are expanded to their exact original literal
bytes before projection; including a source file cannot accidentally return its
redacted declaration text. Source and include reads cannot expose the private
store key, including through a hardlink.

Opaque membership uses exact lowered instance identities, including `Self`
and associated substitutions. It is not inferred by trimming arbitrary
name suffixes. Unused private imports are pruned; the retained declaration and
template requirements determine the dependency source frontier. Transitive
interface identities remain authenticated even when their source projection
does not need to be loaded. A selected record may retain its authenticated
projection's requirement list for this command, avoiding repeated decoding
solely to rediscover the same source frontier. A freshly checked dependent still
opens the complete authenticated interfaces and projections it actually needs.

A same-line nongeneric body edit can preserve the dependency-facing projection
and interface, so consumers reuse their prior admission and native objects.
Projection source positions remain exact for retained templates; changes that
move those positions may invalidate those consumers. This is not a claim that
every textual edit is constant time.

## Native admission and memory

A native object is accepted only under its authenticated checked-body,
interface, runtime ABI and complete source-table identity. The emitter compares
its layouts with the independent checker's layouts. Local and imported
relocations carry exact defining-unit bindings. The linker rejects conflicting,
missing or stale definitions, malformed relocations and unencodable relative
offsets. Custom trap messages are resolved against the final canonical table,
including when a later unit adds an earlier-sorting message.

Version-two native objects store export offsets and sites in exact authenticated
interface order. Their key also binds the ordered checked-body identity; an
ordinal cannot select a different implementation. Per-object symbol and message
tables supply checked relocation ordinals. Each function name is bound to its
exact defining unit and semantic identity in a stored per-unit claim. Earlier
object namespaces are cache misses, with no fallback admission.

The linker writes code and relocation spools incrementally and patches the
unpublished output file through bounded buffers; it does not assemble a
whole-program IR or executable byte vector. A borrowed immutable runtime context
reuses its exact identity within the command. Authenticated object payloads keep
their validated code spans without copying the code into a second allocation.
Each object's temporary relocation frame keeps its names once and uses ordinal
rows. Symbol lookup groups entries by exact provider and interface identity,
loading one provider claim at a time. Resolved offsets stay only in the current
frame. The linker validates bindings at their relocation rows in original order.
It reports a missing binding with the full name and source location of the first
affected row.
Initial and runtime-support records retain their named form. Frame and name
allocations are bounded by maxima observed by the trusted writer, with local
byte/count checks before allocation. These are extent facts, not resource policy.
A failed object poisons the linker, and arrival order remains unrestricted.
Relocation consumption still verifies the complete recorded extent, physical EOF
and digest before the ELF header is written or the output can be published.

`Linker::new(output, relocations, records)` takes a third scratch `File` for
command-local claims and tables. The driver creates its scratch files as
exclusive mode-0600 files beneath the selected `--store` Directory and
immediately unlinks their names while retaining their open handles. The store
must allow creation and removal even for an unchanged build. The unpublished
output is a regular file managed by the atomic publication path.
The ordinary-source `native::records::Store`
provides insert, replace, lookup and ordered traversal through an append-only
AVL index. The writer constructs the trusted root locator and digest and keeps
them in RAM. Every node and value read checks its exact offset, length and
digest before decoding. Read allocations use maxima observed by the writer.
Child extents, heights, key order and local framing are checked independently.
Insertion retains the authenticated index path, reusing small decoded nodes
within one write. The complete file digest and physical EOF are verified before
publication, including the header and superseded nodes. Any failed public store
operation poisons further use.

Stored claims preserve checked imports, dependencies, export offsets and sites,
including private helper bindings and full generic instance names. Each unit also
stores a small identity record binding its exact name, runtime context and semantic
interface identity. Dependency checks read this record. Imports and relocation
bindings load the complete provider claim. Finish compares every identity
with its full claim in unit-name order and requires exactly one claim, identity
and ordinal table per pushed unit. A separate authenticated ordinal key preserves
the arrival order of source and message table validation. Registry corruption
poisons the linker before ELF publication.

`build-units` also supplies `ImageTables` with a separate private, unlinked
regular scratch file. Source blocks bind their exact starting ordinal, count
and ordered paths. Repeated paths keep distinct ordinals. At most one decoded
source block is cached. Raw trap messages are exact keys in the authenticated
store. Their lexical order assigns codes before display escaping or truncation,
so distinct long messages with equal displays remain distinct.

The final diagnostic data is streamed into the unpublished output. Count and
pointer/length rows are reserved and patched through bounded buffers, followed
by one escaped string at a time. Only runtime support code enters the ordinary
assembler. The linker checks its fixed data exports against the written data
extent.
The table file's complete digest and physical EOF are checked with claims and
relocations before writing the ELF header. `check-units` creates no final-image
table file. Both the slice-based linker API and the unit driver's disk-backed
API use the same validation and streaming path.

Source AST, elaboration and IR storage are released after each unit. The source
frontier includes needed declarations/templates and the standard library.

The native linker keeps one decoded provider, the current relocation frame,
runtime support symbols and bounded I/O windows in memory. The registry and its
index grow in filesystem-backed scratch files. These files can occupy
reclaimable operating-system page cache separately from process RSS.
The manifest, its lexical schedule, selected records, input claims and final
source/message tables grow in authenticated filesystem scratch storage. Manifest
reading retains bounded chunks, the largest lexeme and an authenticated index
path. Definition lookup restores one direct-dependency list. Only adjacent unit
input caches remain resident. Reuse-store accounting streams its authenticated
size index into command-local records. It scans directory entries one at a time
and streams checkpoint rows under the existing writer lock. The persistent
envelope and accounting format are unchanged. Scratch files and reclaimable
page cache remain outside the published-byte quota and process RSS, respectively.

The active unit still retains its exact transitive dependency vector and
selected declaration/template interfaces. These can grow with the dependency
closure of that unit. Record index paths also grow logarithmically with the
number of stored records. Measurements hold the checking frontier fixed to
isolate the removed program-wide retention.
`--stats` reports maxima for definitions, IR functions/types, imported
interfaces, projection bytes and source entries to make that distinction visible.
It also reports the largest flat dependency vector and the count of input-cache
entries sampled after each unit.

The existing store default is 256 MiB. Large cohorts must explicitly provision
enough `--store-bytes` for live interfaces, projections and objects; eviction
is a refusal or cache miss, not permission to trust stale bytes. Unit builds
currently emit original checked IR; optimization flags are rejected rather
than silently ignored.

## Evidence and reproduction

The ordinary gate runs:

- `tools/check-units.sh`: independently checked interfaces, authenticated
  decoding, structural and ownership attacks, including a three-unit destructor
  reexport, canonical span identities and reauthenticated malformed payloads.
  The current focused run refuses 2,809 deliberate mutants.
- `tools/unit-native.sh`: 243 native contract cases covering cross-unit
  execution, wrong domains, stale bodies, ordinal tables, bounded spool frames,
  names larger than 64 KiB, unpublished headers on refusal and preserved custom
  traps with exact unresolved-binding locations. Registry controls cover all
  four AVL rotations, replacement and nonmembership at empty/UTF-8/prefix keys,
  duplicate/missing-key poisoning, root/internal/value/obsolete-record changes,
  swapped values, length/UTF-8 damage, truncation, trailing/sparse bytes and
  rollback. The ordered walk covers empty and early-stop visits, exact row
  order and payloads, and an ancestor changed after descent. Native controls
  also preserve source/message validation arrival
  order, allow unused well-formed missing relocation names, reject missing/stale
  declared dependencies despite an independent entry, and report the first
  original unresolved row when provider sorting would select another name.
  Identity records additionally reject malformed framing, context/name/digest
  disagreement, every truncated prefix, invalid UTF-8, trailing bytes, extra or
  missing records and backing-file tampering before an ELF header is written.
  Eighteen table controls compare streamed support bytes, definitions and
  relocation sites with the independent vector encoder. They compare complete
  ELF images and their execution through both table adapters, and check duplicate
  source ordinals, empty/Unicode/escaped strings, display limits and colliding
  truncated displays. Damaged header/EOF/node/value data, cached-value changes
  and poisoned tables refuse before publication. An out-of-range ordinary
  assembler export also refuses.
- `tools/unit-source-test.sh`: cold checking/building, byte-identical warm
  reuse, public/private/method body edits, generic invalidation, exact includes,
  corrupted interfaces, key exclusion, cycles and atomic rejection, plus eight
  snapshot-consistency attacks on both engines, ten disk-retained input contract
  groups on both engines, transitive generic requirements,
  corrupted source projections and failed-publication cleanup. The hard-link
  key attack keeps its key outside the source grant on the same filesystem.
  Successful and refused unit builds must leave no named scratch spools.
- `tools/unit-plan-test.sh`: the literal-parser oracle, semantic refusal order,
  all declaration orders, lexical ready-heap changes, UTF-8/comment/escape/raw
  string boundaries, oversized literals, retained-file mutations and poisoned
  schedule reads. The actual unit commands exercise changed declaration order,
  Unicode paths, warm reuse, attribute refusal, cycle refusal before ENTRY or
  source reads, private-key manifest exclusion and unchanged published output.
  Arbitrary malformed whole-language parser precedence is outside this grammar.
- `tools/unit-boundaries-test.sh`: returned field views, imported destruction,
  narrow authority, fresh implementation linkage and rejected caller mutation.
- `tools/budget-test.sh`: 809 authenticated accounting controls and 11 host
  controls for index restore and checkpoint, traversal, eviction, and tampering.

The size ladder and stranger trials are outside the gate. The ladder records
exact source lines separately from manifest/driver lines, independently checks
generated output, compares fixed body edits and captures external memory,
wall/CPU time, load and frontier counts. A retained successful compile is not
substituted for a failed or unattempted rung.
