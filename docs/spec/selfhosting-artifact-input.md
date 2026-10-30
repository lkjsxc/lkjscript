# Tier 1 artifact input and emission

This is Mode 1, profile 7 of the current [LKJCI1 contract](selfhosting.md).
Rust admits the source and projects canonical compilation-unit fields. The
lkjscript compiler checks framing, supported unit layouts, source-object hashes
and required metadata references, then constructs current `.lkja` bytes. It does
not yet decode or semantically admit the source objects. Matching unit IR to
source meaning remains a host precondition and an independent strict-loader
check. Structural checks alone do not establish type, ownership or effect safety.

The test-only exporter `src/platform/compiler/selfhost_artifact_input.rs` reads
one currently admitted package. It projects fields and instructions, without
serialized units, unit keys, compilation manifests, derived map pages, container
bytes or executable templates. All output identities and envelopes are derived
inside lkjscript. Unsupported payloads, multiple packages, witnesses, regions,
effects, constrained type parameters and test summaries reject at export.

## Complete input

All scalar fields are little-endian; IDs and digests are raw bytes. Magic is
`LKJCI001`, the u32 profile at offset 8 is exactly 7, and the u32 emission selector
at offset 112 is exactly 1. Mode 0 retains profile 6. Each mode rejects every
other profile; there is no previous-profile reader.

Transport counts, offsets and lengths occupy eight bytes. The selfhost reader
admits decoded counts and byte-indexing values only in the nonnegative I64 domain,
then proves their actual buffer extents before arithmetic or indexing. A high-bit
word returns an invalid sentinel and cannot become an empty count. The Rust
producer also checks host `usize` and wire-u64 representability. These are checked
representation boundaries, not default logical quotas. The finite allowance uses
two u32 limbs and preserves the entire u64 range independently of byte indexes.

The header is exactly 160 bytes:

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | Magic `LKJCI001` |
| 8 | 4 | Profile 7 |
| 12 | 4 | Policy tag: 0 = unbounded, 1 = finite; other tags reject |
| 16 | 8 | Complete actual byte length L |
| 24 | 8 | Positive unit count U |
| 32 | 8 | Exact finite allowance A; zero required with tag 0 |
| 40 | 32 | BLAKE3 of `[0,40)` concatenated with `[72,L)` |
| 72 | 8 | Unit directory offset, exactly 160 |
| 80 | 8 | Unit count, exactly U |
| 88 | 8 | Source-object directory offset, exactly `160 + 16*U` |
| 96 | 8 | Positive source-object count O |
| 104 | 8 | Metadata offset, exactly `160 + 16*U + 56*O` |
| 112 | 4 | Artifact selector 1 |
| 116 | 4 | Reserved zero |
| 120 | 8 | Metadata byte length |
| 128 | 32 | Reserved zeros |

The policy is `Option<u64>` in Rust. Tag 0 means `None`, bypasses the finite
estimate and requires all eight allowance bytes to be zero. Tag 1 means
`Some(A)`, including `Some(0)` and `Some(u64::MAX)`. Finite zero never means
unmetered, and the largest finite value is never converted to `None` or clamped.
The deterministic estimated proof-admission requirement is

`F = 4096 + 128*L + 64*(U*U + O) = 64*(64 + 2*L + U*U + O)`.

Finite admission requires `A >= F`. The comparison does not form a potentially
overflowing square or saturate a requirement. It starts with
`B = floor(A/64) = high32*67108864 + floor(low32/64)`, at most `2^58 - 1`.
It requires and subtracts 64, requires `L <= floor(B/2)` and subtracts `2*L`,
requires and subtracts O, then requires positive U and `U <= floor(B/U)`.
Every intermediate therefore fits nonnegative I64, and the comparison preserves
exact, one-short, nonmultiple, `2^63` and `u64::MAX` allowances. A requirement above
`u64::MAX` rejects every finite allowance. Insufficient finite admission returns
diagnostic 6 before carrier hashing, deep validation or generated-output work.

This is an up-front estimated proof-admission threshold. The passes do not
consume it as an execution meter. Evaluator instructions, allocation policies,
cancellation and passive observations are separate contracts. Saturated
observations cannot admit finite work or prove an extent. Host artifact admission,
source-map verification, decoding, inventory construction and sorting precede
projection measurement and are outside the embedded consumer policy. After that
preprocessing, the Rust producer uses the same emitter to measure and fill the
projection. Measurement allocates no output byte buffer; exact finite admission
precedes its single fallible output reservation, and filling must agree with the
measured extent.

Cheap framing independently requires the complete unit/source directories,
metadata span, all runtime-owner rows and at least 432 bytes for each projected
unit. Generic extent checks require nonnegative offset/count and positive stride,
establish `offset <= L`, and compare `count <= (L-offset)/stride` before
multiplication or addition. A zero count at the end is valid; an out-of-range
offset, zero stride or nonrepresentable extent rejects. There is no 4 GiB carrier
ceiling or projection-only object/unit inventory ceiling.

Directory rows are `(offset:u64,length:u64)`, 16 bytes each. Metadata, unit blobs
in directory order and source-object blobs in row order fill the input
consecutively, without overlap, padding or trailing bytes. Unit pointers are
relative to their own unit blob; outer pointers are absolute. Unit identities
are strictly ordered by OwnerKey. For R runtime owners, the exact layout is:

```text
unit_directory = 160
source_directory = 160 + 16*U
metadata = source_directory + 56*O
metadata_bytes = 312 + 56*R
first_unit = metadata + metadata_bytes
unit_start[0] = first_unit
unit_start[i+1] = unit_start[i] + unit_bytes[i]
first_source = unit_start[U]
source_start[0] = first_source
source_start[j+1] = source_start[j] + source_bytes[j]
L = source_start[O]
```

These equations are formed only after their constituent extents are proved.
Admission terminates through advancing byte and row cursors, decreasing remaining
element counts, strictly forward instruction edges and binary-search interval
halving with `lo + (hi-lo)/2`. No default step count proves termination, and
admission does not execute calls from the compiled program. The generated-row
selection sort remains quadratic; wider framing does not establish scalable
large-inventory performance.

### Admission order

The ordinary `ci::validate` entry first requires its 128-byte dispatch extent,
then checks magic and emission selector. The Mode-1 reader requires the complete
160-byte header and profile 7. Its remaining checks have this order:

| Check | Failure diagnostic |
| --- | --- |
| Exact total length, positive common unit count, policy tag and zero `None` payload | 3 |
| Cheap directory/metadata/runtime-owner framing, duplicate unit count, reserved words and minimum unit extent | 10 |
| Exact finite estimated proof admission | 6 |
| Complete carrier digest | 4 |
| Every projected unit, including unused units | 10 |
| Every source object's domain, order, extent and digest | 11 |
| Metadata identities, required references and runtime owners | 10 |

Wrong magic returns 1; unsupported profile or selector returns 2. An earlier
rejection precedes later work. In particular, malformed cheap framing precedes
finite refusal, and finite refusal precedes hashing. A negative fixture targeting
a later layer must preserve every earlier commitment and layout requirement.

### Source objects and metadata

Source rows are 56 bytes: domain u32 at 0, zero source selector u32 at 4, absolute
offset u64 at 8, length u64 at 16, and digest32 at 24. Keys increase strictly in
current ObjectKey enum order, which differs from numeric wire tags: 1, 2, 3, 5,
25, 23. Digests use BLAKE3 derive-key with the domain context and
`length:u64be || object_bytes`:

| Tag | Object | Context |
| --- | --- | --- |
| 1 | Owner | `lkjscript.kernel.owner-object.v14` |
| 2 | Type | `lkjscript.kernel.type-object.v10` |
| 3 | Blob | `lkjscript.kernel.blob-object.v5` |
| 5 | Source map page | `lkjscript.persistent-map-page.v2` |
| 25 | Package revision | `lkjscript.kernel.package-revision.v1` |
| 23 | Package interface owner | `lkjscript.package-interface-owner.v7` |

Only pages reachable from the package's interface-owner and reference-owner maps
are supplied. Host-produced compiler units, compilation manifests and compilation
map pages are excluded. Source objects retain their ordinary current encodings.

Metadata occupies `312 + 56*R` bytes:

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 / 16 | 16 each | Nonzero repository / package ID |
| 32 / 64 / 96 / 128 | 32 each | Package revision / semantic revision / semantic state / interface digest |
| 160 / 232 | 72 each | Interface-owner / reference-owner map root |
| 304 | 8 | Runtime-owner count R |
| 312 | `56*R` | Runtime-owner rows |

A map root is page digest32, entry count u64 and content digest32. All eight bytes
of counts at metadata offsets 192, 264 and 304 participate in their respective
consumers. The selfhost profile requires nonnegative I64 decoded counts; a Rust
serialization control retaining all u64 bits does not establish runtime admission
of a bit-63 count. Runtime-owner rows retain their 56-byte representation:
OwnerKey ordinal u32, OwnerKind ordinal u32, nonzero ID16 and object digest32.
Rows are strictly ordered by OwnerKey, and each digest names an included Owner
object. Supported ordinal pairs are declaration 1 with kinds 4/5/8, type parameter
2 with kind 10, parameter 6 with kind 14, port 10 with kind 18, and target 11 with
kind 19.

The package revision and both root pages must exist in the source-object table.
The logical interface identity uses the package ID and interface map's exact
content digest/count, the current `LKJPIFI1` envelope and
`lkjscript.kernel.package-interface.v1` digest context. It is distinct from
individual interface-owner object identities. Each unit has the metadata's
package ID; if its identity also appears in the runtime-owner table, the kinds
must agree. Runtime metadata is a selected subset of the unit inventory. A
program without parameters or a resource boundary may have no runtime-owner rows.

The reference-map count at metadata offset 264 does not drive a local allocation
or iteration and has no parallel interface-identity comparison in this admission
layer. It is passed to the emitted artifact manifest. No synthetic count cap or
count-versus-carrier-length constraint substitutes for root verification. A
hash-consistent small reference-count mismatch can reach emission and must be
rejected by the strict artifact loader's root-summary check. The metadata's
interface count instead participates in the interface identity and can fail
metadata admission with diagnostic 10. Controls must preserve that distinction.

## Unit projection

A projected unit has a minimum extent of 432 bytes and no independent 8 MiB
ceiling. The canonical Rust unit reader retains its separate encoded-unit
boundary; this transport change does not remove it. Every unit starts with a
272-byte descriptor:

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 / 4 / 8 | 4 each | OwnerKey ordinal, OwnerKind ordinal, OwnerKind wire tag (ordinal+1) |
| 12 | 4 | Reserved zero |
| 16 / 32 | 16 each | Nonzero package / owner ID |
| 48 / 80 / 112 / 144 / 176 / 208 | 32 each | Semantic interface, implementation, type, effect, capability, validation-dependency digests |
| 240 | 8 | Table directory offset, exactly 272 |
| 248 | 8 | Signature offset, or zero for component/target |
| 256 | 4 | Payload tag |
| 260 | 4 | Reserved zero |
| 264 | 8 | Payload offset |

The 160-byte table directory has ten `(offset:u64,count:u64)` pairs. Tables 0–5
hold package16 plus typed-identity16 references for declarations, fields, cases,
requirements, operations and ports. Table 6 holds type digests32. Tables 7 and 9
hold UTF-8 name/text spans; table 8, read-view fields, is exactly `(0,0)`.
Every other table, empty or nonempty, starts at the next canonical position.
String tables place their span rows before their consecutive raw bytes. Each
span is two u64 words, offset and length; strings have no encoded-length prefix.
Literal text is inline; blob-backed text remains outside this projection.

Function/external signatures occupy 40 bytes: a 16-byte type-parameter span, a
16-byte parameter span, and a u64 result type index. Type parameters are 16-byte
IDs without constraints. Parameters occupy 48-byte rows: ID16, name span16,
type index u64 and use ordinal u64 (0 unrestricted, 1 borrow, 2 consume).
Parameter names follow the complete row array. Signatures are pure, with value
results and no implementation, requirement, effect, read-scope or resource
arguments. Type and reference indices are zero-based. There are no independent
128-parameter or 32,768-table-entry projection guards.

| Owner ordinal | Kind ordinal | Payload tag | Payload |
| --- | --- | --- | --- |
| 1 | 4 | 6 | Signature plus implementation-name span16 |
| 1 | 5 | 7 | Signature plus code descriptor32 |
| 1 | 8 | 9 | Port span16 followed by port rows32 |
| 11 | 19 | 11 | Component index, port presence, port index, command runner zero; four u64 words |

External names are the 28 primitives supported by the ELF input profile.
Component ports contain four u64 fields: port index, function-type index,
function-implementation tag zero and declaration index. Components have no
requirements; targets have no routes. All indices, enum values and presence tags
must be valid. Widening the projection does not add new supported language forms.

Code descriptors are 32 bytes: parameter count, local count, instruction offset
and instruction count, all u64. Parameter count matches the signature. There is
no independent 128-local or 32,768-instruction projection guard. Instructions are
48 bytes: tag/A/B u64 at offsets 0/8/16, signed I64 immediate at 24, and u64 extra
offset/count at 32/40. Extra arrays use eight-byte numeric elements and follow the
complete instruction array in instruction order. The supported canonical
pre-tail IR tags are:

| Tag | Instruction | Fields |
| --- | --- | --- |
| 2 / 3 / 4 | Unit / Bool / I64 | Bool A is 0 or 1; I64 uses immediate |
| 5 / 6 | Text / static text | A text index |
| 7 / 8 | Load / store local | A local index; load B use ordinal |
| 9 | Drop | No operands |
| 10 / 11 | Conditional / unconditional jump | A strictly forward instruction index |
| 12 | Call | A declaration index, B argument count; extras are type indices |
| 15 | Structural record | Extras are structural-name indices |
| 17 | Structural field | A structural-name index |
| 18 | List | A type index, B item count |
| 27 | Return | No operands; only the last instruction |

Unused operands and immediate bytes are zero. Projected canonical tag 10 is
`JumpIfFalse`; it is not Mode-0 tag 10's tail-call operation. The canonical IR
still represents type, local, reference, branch, argument and item fields as
u32. Their widened projection fields must fit that exact type before canonical
emission, rather than being truncated. Vector/table extents follow actual input
bounds independently of their elements' semantic index types. Unsupported
opcodes reject even in untaken code. Shape admission checks bounds and layout,
not abstract stack execution or correspondence with the raw source graph.

## Current artifact construction and validation

`unit-wire` encodes the current canonical bincode fields, typed ID/digest tags,
unit key, `LKJCUNIT` envelope and object identity. It does not copy an encoded
unit from input. `unit-map` builds the canonical logical radix content digest
and an expandable physical radix hierarchy. Its physical page bound remains
exactly 65,536 bytes; there is no aggregate 700-unit guard. Logical keys are
17 bytes, values are 65 bytes, and sorted unique bindings precede emission.

For U bindings with common-prefix length p, a leaf occupies
`107 + p + U*(88-p)` bytes. The encoder compares U against
`floor((65429-p)/(88-p))` before choosing a leaf. An oversized range branches
on the next key byte. Child ranges are nonempty and emitted in increasing edge
order. A branch has no terminal value for these fixed-width keys and occupies
`106 + p + 82*C` bytes for C children; C is at most 256. Every child descriptor
contains its edge, physical digest, entry count, logical byte count and logical
content commitment. Prefix length increases strictly on descent, bounding the
walk by the 17-byte key representation. The child cursor advances strictly;
its buffer-threading loop uses the existing tail-call mechanism.

The encoder returns the root's image, physical digest and logical commitment,
plus every page in deterministic postorder as `length:u64le || page` frames.
The root is last and appears once. Empty input produces one empty leaf; for
U>0 every branch has at least two children and the hierarchy has at most `2*U-1`
pages. This framing is compiler scratch representation, not a new canonical
map or artifact format. The artifact assembler emits a domain-5 object row for
every frame, then sorts all source and generated objects canonically. It does
not append the root separately. Physical partitioning preserves the logical
content commitment.

Generated object rows use the same 56-byte layout as source rows, with a selector
for the appropriate byte source. Generated pool offsets and lengths use u64.
A unit fact is 138 bytes: the object row56 followed by the unchanged map binding82,
comprising the 17-byte key and 65-byte value. Fact extraction must preserve all
82 binding bytes independently of the object-row width.

Before each frame, the compiler proves that accumulated bytes, its eight-byte
length prefix and payload fit nonnegative I64. It likewise checks the combined
unit frames, map frames and compilation manifest before serializing pool offsets.
The page-frame reader checks the full length word, positive page size, the 64 KiB
physical page bound and remaining payload extent, and rejects trailing fragments.
Exact-fit host extents succeed; negative, bit-63 or one-beyond extents return
diagnostic 3 with empty output. There is no aggregate u32 pool ceiling.

The object byte subtotal is exact checked arithmetic over complete 56-byte rows;
a negative decoded length, partial row or sum overflow rejects. Before allocating
a pack, the emitter proves `pack_length = 144 + object_bytes + 89*object_count`.
Before allocating the final container, it proves
`container_length = 140 + manifest_length + pack_length`. Division and remaining
extent checks precede multiplication and addition. These output sizes are not
passive telemetry. Failure branches consume live builders through
`core.buffer.discard` before returning, preserving affine branch-state admission;
that does not establish native mapping reclamation.

The compiler derives the compilation manifest, combines source and generated
objects in ObjectKey order, derives the artifact closure and manifest, and emits
one raw pack in the current `LKJARTFC` container. Pack entry/index/full checksums,
nonce, pack ID, container checksum and all envelopes retain their existing
contexts and encodings. The 89-byte pack index rows already encode canonical
64-bit offsets and lengths; changing private projection/pool addresses must not
change canonical pack bytes for identical ordered objects. The strict Rust
unit/map/pack codecs and artifact loader retain their independent contracts.

Artifact probe controls compare production VM, disjoint reference,
Rust-bootstrap ELF and selfhost-emitted ELF bytes with independent Rust unit,
map and pack codecs. The selfhost-emitted probe executable is built through the
Mode-0 native backend and cached by its exact emission-payload hash. The controls
strictly load emitted artifacts and execute programs with fixed expected values
and trap codes. Native stage controls compare A2/A3 with the same
VM/reference/C1 output. A successful Profile-7 execution record must bind
the actual adopted compiler and inputs; an earlier Profile-5 record cannot prove
the changed path. All such projected-input results retain Tier 1 scope.

## Capacity, allocation and evidence boundaries

Profile 7 removes projection-only 4 GiB extent, 8 MiB unit, 100,000-object/owner,
32,768-table/instruction and 128-parameter/local/operand guards. It does not widen
canonical u32 semantic indices, remove the canonical unit codec's 8 MiB boundary,
remove the generic packed codec's independent 128 MiB configuration ceiling, or
remove the manifest/cache/query chain's 10,000,000-unit guards. The outer storage
domain allowances also remain 4 MiB for CompilerUnit objects and 64 KiB for
CompilationManifest objects. Canonical kernel objects, persistent-map values,
storage packs and loaders retain their separate admission contracts. Those cuts
require their own progressive decoding,
checked allocation, lifecycle and complete caller proofs.

Mode 0 remains profile 6 with its existing wide framing and explicit policy.
`LKJSG002`, canonical unit/map/pack/`.lkja` bytes and schema epoch 1 are unchanged
by this private transport cut. The native runtime's former 256 MiB per-value
quota was removed in the preceding capacity cut; checked signed-I64 extents,
physical host limits and its process-lifetime bump-allocation behavior remain.
No bounded-RSS or phase-local reclamation claim follows from wider projection.

The Rust exporter checks complete extents before reserving the measured output
with `try_reserve_exact`. Measurement and fill share the same emission path,
including pointers relative to each unit's nonzero origin inside a bundle.
Inventory vectors grow fallibly; unique unit owners use an in-place unstable
sort with unchanged accepted order. Failures return
`compiler_artifact_input_unsupported`. Earlier artifact decoding, source-map
reads and other host structures are outside the embedded proof policy; this
slice does not claim all preprocessing allocations are fallible or metered.

The accepted Profile-5 suite included a publicly authored package with 800 units,
799 unused functions carrying distinct 6,144-byte inline text literals, and input
over 8 MiB. It checked the complete unit identity/text associations, exact source
and unit objects after strict loading, multiple reachable map pages, equal
C1/C2/C3 artifacts, a hash-consistent malformed unused unit reaching diagnostic
10 beyond the former 700-unit boundary, and entry result 42 in both evaluators.
Independent map controls covered empty/singleton maps, 752/753 and 896/897 leaf
boundaries, and a three-level 11-page tree with independent physical/logical
oracles. These historical outcomes are not relabeled as Profile-7 evidence.

The Profile-7 controls retain those expectations and add exact tagged-policy,
wide field/pool, canonical-u32, metadata-root, partial-row and checked-sum cases.
Synthetic extent or root-count tests do not claim a physically allocated >4 GiB
carrier or a valid ten-million-unit closure. A projected code body above 8 MiB
does not establish that the canonical unit codec or public loader admits such a
unit. The [cutover decision](../decisions/20261030-artifact-profile7.md) records
implementation, observed verification, adoption and remaining proof separately.
Source-derived canonical artifacts, complete semantic admission and native phase
reclamation remain Tier 2 obligations.
