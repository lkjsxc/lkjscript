# Semantic compiler bootstrap

The maintained lkjscript compiler compiles its complete authoritative source
graph to standalone Linux x86_64 ELF and canonical `.lkja` artifacts. Raw-source
Milestone A was accepted at `010ce014ae97e87df17d2f6acbea728f73bf14d1`: native
generations established `C2 = C3` and `Ahost = A1 = A2 = A3`, with source mutations,
independent behavior, complete-inventory negative controls and strict loading.
The [acceptance record](../../docs/decisions/20261101-active-source-acceptance.json)
binds that exact implementation tree; [status](../../docs/status.md) identifies
the current graph, subsequent candidates and current acceptance boundary.

Full `P_self` semantics and repeated same-process native phase reclamation are
separate Milestones B and C. Source reproduction alone does not establish them.
See the [self-hosting contract](../../docs/spec/selfhosting.md) and
[source compilation decision](../../docs/decisions/20261101-active-source-self-compilation.md).

## Current inputs and trust boundaries

| Entry path | Current input | Work performed inside lkjscript |
| --- | --- | --- |
| Normalized backend | `LKJCI1` mode 0/profile 6 | Admit the backend projection and emit standalone ELF |
| Normalized backend | `LKJCI1` mode 1/profile 7 | Admit the backend projection and construct a canonical artifact |
| Raw-source compiler | `LKJSRQ01` profile 2 around complete `LKJSG002` profile 3 | Admit source meaning and custody, resolve the requested target, lower it and emit the selected ELF/artifact output |

Each changed boundary has one current format. Predecessor selectors reject
without fallback. The raw carrier includes the exact source revision binding,
semantic-state claim and complete owner/type inventory. It supplies no inferred
types, lowered entry index, custody certificate, normalized instructions or
cached code. The complete carrier hash also binds unused supplied types that
are not committed by semantic state alone.

The Rust compiler is a legitimate bootstrap producer and independent artifact
oracle. Native self-produced generations consume the unchanged complete source
carrier plus caller-selected TargetId and output mode. The source compiler
constructs its own normalized backend projections; the caller does not supply
Rust-normalized target input to those generations.

The backend profiles use wide offsets and lengths for their private projection
pools. Removed aggregate projection quotas are distinct from remaining canonical
unit/store representation boundaries. The canonical unit map grows through radix
pages with a 64 KiB physical page bound, without an aggregate 700-unit guard.
[Source framing](../../docs/spec/selfhosting-source-input.md) and
[artifact framing](../../docs/spec/selfhosting-artifact-input.md) specify exact
extents, policies and remaining boundaries.

An absent proof policy is unmetered. Every explicitly selected finite u64
allowance remains finite and exact, including zero and `u64::MAX`. Estimated
admission policies, evaluator work, cancellation, passive observations and actual
allocation failure have different contracts. Unmetered admission does not remove
structural checks or justify unchecked arithmetic.

## Source admission and lowering

The source modules compose explicit layers. Object readers decode current
canonical payloads; graph/state/history checks bind their identities; membership,
containment and scope passes reject missing or ambiguous ownership and lexical
references. Namespace, contextual type-binder, structural-DAG and nominal
parameter-flow checks follow. Every supplied owner and type is inspected,
including uncalled functions, untaken branches and unselected targets.

Despite their historical `source-scalar-*` names, the active compiler modules
support the maintained compiler's compound feature closure: Text/StaticText,
Bytes, affine ByteBuffer, List<I64>, structural records/fields and pure function
types, alongside Unit, Bool and I64. Closed generic substitutions used by I64
list operations are admitted explicitly. Expression admission and lowering
cover lexical locals, Let, Sequence, If, lists, records, fields, direct calls,
recursion and the exact active intrinsic signatures. Target selection supports
pure `() -> I64` commands and the pure binary-filter signature
`(Bytes) -> {diagnostic: I64, image: Bytes}`.

ByteBuffer custody checks consumption, synchronous borrow qualification,
branch-balanced disposal, reverse lexical cleanup and returned-owner transfer.
A live owned local prevents unsafe tail transfer; lowering uses call, cleanup
and return instead. Complete semantic and custody admission precedes emission.
Unsupported forms reject explicitly rather than bypassing their semantics.

The `source-scalar-plan`, cursor/state/trace/emit helpers and packing modules
build typed backend input inside lkjscript. Source artifact construction includes
canonical units, summaries, interfaces, maps and provenance. Closed bulk-byte
intrinsics avoid per-octet temporary values and are supported by both evaluators
and both native compiler paths. Capability support must precede adoption of a
new feature into the maintained compiler. Refresh the active-source inventory
and raw-source fixed point after any closure expansion.

## Native phase ownership

The native phase candidate gives a pure binary-filter invocation independent
scratch and output allocation contexts. It promotes the complete surviving
result into output storage before disposing scratch, and allows callers to
retain independent output handles across later invocations. Cancellation,
allocation failure, runtime rejection and cleanup failure have distinct native
statuses, separate from a completed compiler diagnostic. Caller input, handle
and cancellation storage have explicit lifetime and exclusivity requirements.

The [native phase ABI](../../docs/spec/native-phases.md) specifies discovery,
handle layout, invocation, release and recovery. Source modules `phase-asm`,
`phase-code` and `phase` emit and check the runtime; corresponding Rust native
owners implement the same external contract. Bulk copies and extent-scaled
helper loops contain cancellation safe points. Internal phase contexts do not
establish language-level lexical Regions or eliminate the native CALL stack.

Production Milestone C acceptance requires repeated actual compilation inside
one process, valid surviving output after scratch unmapping, failure/cancellation
recovery and independent mapping observations. A modified-binary prototype,
process exit or zeroed counters alone is insufficient. The current tested tree
and terminal evidence belong in status; this description is not a new seal.

## Authoritative graph and public adoption

`project/` owns accepted program meaning. The `.lkjc` files in this directory are
maintained authoring proposals. Adopt changes through supported public
transactions and confirm public readback; editing a proposal does not update
the authoritative compiler by itself.

Tests reconstruct the compiler through ordinary public transactions: typed
interfaces first, then validated function bodies. Complete proposal replay
submits one public transaction on a private copy of the accepted graph and
compares every canonical compilation unit and the complete native backend
input. The replay and incremental-reconstruction paths use ordinary unmetered
memory repositories and normal publication admission. Their publication
identities may differ while canonical units agree.

Independent mutation fixtures establish expected behavior or diagnostics rather
than relying only on compiler-to-compiler equality. Tests execute the same
compiler through VM, the disjoint canonical reference interpreter and native
ELF. Every emitted artifact must pass strict loading and provenance checks.
Source tests bind the exact graph, carrier, request, profiles, target, compiler
bytes and deterministic execution context.

## Development and verification

Run the focused harness from the repository root under the shared Cargo target
lease:

```sh
RUST_TEST_THREADS=8 bash tools/selfhost/verify.sh
```

The focused harness is not full acceptance. Before integrating a dependency-
closed milestone, finish public proposal adoption, generated discovery and
status/specification edits; stage and freeze the exact candidate; bootstrap the
verifier for its source/tree binding; run the complete selfhost selection,
named maintained-native-closure, whole-owner/type inventory and raw-source
fixed-point controls; then pass all fresh full gates and release-target checks.
The [verification contract](../../docs/spec/verification.md) owns the exact
protocol and evidence boundaries. Preserve working-tree/index quiescence
throughout the seal and commit that exact tested tree.

Build the normalized native filter with:

```sh
lkjscript --project tools/selfhost/project build --native-elf COMPILER --target compile-elf
```

It reads one admitted `LKJCI1` request from stdin and emits complete ELF or
artifact bytes on stdout, with diagnostics on stderr. Select
`--target compile-source-elf` for the raw-source filter. Its stdin is a current
`LKJSRQ01` request carrying the selected TargetId, output mode and complete
`LKJSG002` carrier. The target's historical name does not restrict the request
to ELF output. An external launcher owns atomic output-file publication.
