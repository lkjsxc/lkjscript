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

## Typed input prototype

`typed-input.lkjc` runs on the evaluator and builds a type catalogue with records,
variants, exhaustive matching, generic lists and structured errors. Its comparison
checks every digest and canonical image against Rust objects and the cursor reader:

```sh
bash tools/check.sh slow typed_compiler_input_matches_maintained_cursor_and_type_objects
```

The prototype keeps canonical type bodies as opaque bytes and currently scans the
list for lookup. Its next step is structural-child decoding and a typed DAG pass;
collection update cost and intrinsic-heavy notation are measured design inputs.

On the maintained 26,868,750-byte carrier, all 98 types matched. The typed source
is 139 lines and 10,394 bytes (2,528 o200k_base tokens); the comparable 21-declaration
cursor count/data/lookup closure is 10,247 bytes (2,486 tokens). Building the typed
catalogue took 3 ms; counting and reading all 98 cursor entries took 9 ms. Explicit
intrinsic wrappers account for much of the notation cost. The evaluator provides
the generic records/variants/list support used here; the native compiler subset
will grow with the next typed pass.
