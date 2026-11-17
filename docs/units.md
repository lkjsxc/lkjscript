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

Unit names are identifiers; sources are relative `.lkj` paths beneath
`--source-dir`, without `..`. Dependencies must form a DAG. Mutually
dependent modules belong to the same unit. The compiler orders units itself.

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
checked again when its source is available. A required dependency record that
was evicted or fails authentication refuses the request. It is never replaced
by an unsigned declaration. Repeated imports of a genuine reexport are accepted
only when their defining binding, exact signature and effects agree.

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
does not need to be loaded.

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

The linker writes code and relocation spools incrementally and patches the
unpublished output file through bounded buffers; it does not assemble a
whole-program IR or executable byte vector. A borrowed immutable runtime context
reuses its exact identity within the command. Authenticated object payloads keep
their validated code spans without copying the code into a second allocation.
Relocation consumption still verifies the complete recorded extent, physical EOF
and digest before the output can be published.
Source AST, elaboration and IR storage are released after each unit. The source
frontier includes needed declarations/templates and the standard library.

The driver still retains unit/dependency/input/source metadata, and the linker
retains a compact registry of exported and internal function symbols. These
grow with the program. Thus bounded source/IR storage is an architectural fact;
strict total-memory O(largest unit) is not asserted without measurement.
`--stats` reports maxima for definitions, IR functions/types, imported
interfaces, projection bytes and source entries to make that distinction visible.

The existing store default is 256 MiB. Large cohorts must explicitly provision
enough `--store-bytes` for live interfaces, projections and objects; eviction
is a refusal or cache miss, not permission to trust stale bytes. Unit builds
currently emit original checked IR; optimization flags are rejected rather
than silently ignored.

## Evidence and reproduction

The ordinary gate runs:

- `tools/check-units.sh`: independently checked interfaces, authenticated
  decoding, structural and ownership attacks, including a three-unit destructor
  reexport. The current focused run refuses 954 deliberate mutants.
- `tools/unit-native.sh`: 32 native contract cases covering cross-unit
  execution, wrong domains, stale bodies, relocations, spool alteration and
  preserved custom traps.
- `tools/unit-source-test.sh`: cold checking/building, byte-identical warm
  reuse, public/private/method body edits, generic invalidation, exact includes,
  corrupted interfaces, key exclusion, cycles and atomic rejection, plus eight
  snapshot-consistency attacks on both engines and failed-publication cleanup.
- `tools/unit-boundaries-test.sh`: returned field views, imported destruction,
  narrow authority, fresh implementation linkage and rejected caller mutation.

The size ladder and stranger trials are outside the gate. The ladder records
exact source lines separately from manifest/driver lines, independently checks
generated output, compares fixed body edits and captures external memory,
wall/CPU time, load and frontier counts. A retained successful compile is not
substituted for a failed or unattempted rung.
