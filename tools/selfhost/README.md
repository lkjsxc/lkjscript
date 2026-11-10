# Semantic compiler bootstrap

The maintained lkjscript compiler emits standalone Linux x86-64 ELF and canonical
`.lkja` artifacts for its supported source closure. The Rust host builds C1 from
`project/`; C1 builds C2 and C2 builds C3 from the same raw source. The fixed-point
control checks `C2 = C3` and `Ahost = A1 = A2 = A3`, strict artifact loading,
independent behavior and source mutations that change emitted behavior.
[Status](../../docs/status.md) records the current source revision and next action.

## Source and input

`project/` owns the maintained compiler's accepted meaning. The `.lkjc` files here
are authoring proposals. Adopt changes through public transactions and confirm
public readback before rebuilding the native generations. Tests replay proposals
on an isolated copy and compare canonical compilation units and backend input.

| Entry | Input | Compiler work |
| --- | --- | --- |
| `compile-elf` | `LKJCI1` mode 0/profile 9 | Admit the normalized backend projection and emit ELF |
| `compile-elf` | `LKJCI1` mode 1/profile 8 | Admit the normalized backend projection and construct a canonical artifact |
| `compile-source-elf` | `LKJSRQ01` profile 2 around `LKJSG002` profile 3 | Admit source meaning and custody, select the target, lower it and emit ELF or an artifact |

The raw carrier contains the complete current owner/type inventory, revision
binding and semantic-state claim. Source admission checks object identities,
reference closure, containment, lexical scope, namespace, contextual types,
structural DAGs, nominal parameter flow and ByteBuffer custody before emission.
The current compiler still reads history and retirement records; narrowing that
input is the next representation change.

The [self-hosting specification](../../docs/spec/selfhosting.md) describes supported
semantics and the broader design target. [Source framing](../../docs/spec/selfhosting-source-input.md)
and [artifact framing](../../docs/spec/selfhosting-artifact-input.md) define exact
encodings, checked extents and remaining policies. The structural-DAG reader uses
dependency counts and a work queue; duplicate edges are counted consistently and
unprocessed nodes identify cycles.

## Build and verify

From a clean checkout:

```sh
cargo build --release --locked --bin lkjscript
bash tools/check.sh
bash tools/selfhost/verify.sh
```

The routine command builds and runs its suite once. The selfhost command selects
the raw-source fixed point, scalar 42→43 behavior mutation and compiler diagnostic
12→77 mutation. Run the three maintained controls together when their code changes
so they share immutable source, requests and compiler images:

```sh
bash tools/check.sh slow whole_maintained_raw_source_fixed_point \
  selfhost_source_artifact_maintained_native_lifecycle \
  selfhost_source_artifact_maintained_semantic_corpus
```

The lifecycle control repeatedly invokes both C1 and C2 in one process, checking
retained output, cancellation, allocation failure, recovery and cleanup. The
corpus checks scalar, compound, custody, binary-filter, bulk-byte, callable,
nominal, wide-sequence and rejection behavior against stabilized C2. Each control
uses fresh invocations and verifies the original repository's inventory.
[Verification](../../docs/spec/verification.md) describes focused and slow selection.
The [native phase ABI](../../docs/spec/native-phases.md) defines output ownership
and scratch reclamation.

To produce a reusable raw-source filter:

```sh
target/release/lkjscript --project tools/selfhost/project build \
  --native-elf /tmp/lkjscript-compiler --target compile-source-elf
```

Its stdin carries the target, output mode and complete source carrier. Output
bytes go to stdout and diagnostics to stderr. An external launcher publishes the
output file atomically. Select `compile-elf` for normalized backend input.

## Typed compiler input

`typed-input.lkjc` and `typed-dag.lkjc` bootstrap through the Rust evaluator. They
use records, variants, exhaustive matching and generic persistent collections to
read structural types and compute their dependency graph.

| API | Result |
| --- | --- |
| `typed-input::read(carrier)` | Ordered `TypeEntry` catalogue with digest and canonical image |
| `typed-input::decode-image(image)` | Named `TypeKind` and ordered child digests |
| `typed-dag::analyze(entries)` | Child ordinals, reverse parent occurrences and child-before-parent order |
| `typed-dag::read-graph(carrier)` | Catalogue reading followed by graph analysis |

The structural reader consumes canonical images from an admitted snapshot. It
checks envelope framing, canonical integer encodings, identity tags and complete
structural payloads; snapshot admission supplies checksum, digest and semantic
authority. All 35 type constructors have named cases. Scope, effect and declaration
metadata are parsed as metadata; structural child occurrences become graph edges.

A catalogue's list position is its graph ordinal, and analysis checks that each
`TypeEntry.index` agrees. Structured errors identify malformed entries and byte
offsets, incorrect ordinals, both duplicate-digest positions, missing child
occurrences, or the ordinals still blocked by cycles. Duplicate edges remain
present in both directions.

Analysis builds a digest-to-ordinal map, reverse adjacency and pending dependency
counts. A list queue releases each vertex once and visits each reverse edge once.
Persistent AVL maps add logarithmic lookup/update work; lists use 32-way trie
paths. The pass needs no new collection primitive. The cursor DAG instead scans
the complete edge array for each released vertex.

Run the focused controls and the explicit maintained/scale tier with:

```sh
bash tools/check.sh focused typed_compiler_input
bash tools/check.sh slow typed_compiler_input_matches_maintained_cursor_and_type_objects \
  typed_compiler_input_structural_dag_scale
```

Five routine controls cover 49 canonical images spanning all 35 constructors,
malformed payloads and truncations, ordered child occurrences, reverse
multiplicities, and structured failure locations. Generated graphs are compared
with an independent Rust DFS oracle; a reducer removes vertices and edge
occurrences from a disagreement. The maintained comparison also checks the cursor
reader and every canonical Rust type object.

On the maintained 26,868,750-byte carrier, all 98 types and dependency edges matched:

| Evaluator operation | Typed component | Cursor component |
| --- | ---: | ---: |
| Read the complete type catalogue | 4 ms | 9 ms |
| Read and check the structural dependency graph | 43 ms | 2,398 ms |

Geometrically growing probes build canonical carriers, then measure evaluator
reading and graph analysis:

| Types | Chain | Wide graph |
| ---: | ---: | ---: |
| 256 | 50 ms | 57 ms |
| 1,024 | 200 ms | 229 ms |
| 4,096 | 820 ms | 932 ms |

The largest lifecycle checks, including result comparison, cloning and destruction,
took 831 ms and 943 ms. The typed reader is 862 lines, 44,415 bytes and 10,167
`o200k_base` tokens; the DAG pass is 452 lines, 18,585 bytes and 4,123 tokens.
The cursor DAG alone is 326 lines, 15,794 bytes and 3,921 tokens. Reader coverage
differs: the typed decoder handles all constructors, while the cursor child reader
covers the maintained compiler subset.

The implementation passed public authoring and its first focused runtime run.
Concrete notation costs remain: explicit intrinsic declarations, repeated generic
type arguments, reconstruction of unchanged record fields, and repeated matching
to propagate errors. The next compiler step is a typed current-meaning snapshot
with exact dependencies, using this structural pass as its first checked component.
