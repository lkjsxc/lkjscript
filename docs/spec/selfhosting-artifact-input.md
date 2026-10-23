# Tier 1 artifact input and emission

This is the artifact mode of the current [LKJCI1 profile](selfhosting.md).
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

All scalar fields are little-endian; IDs and digests are raw bytes. The common
magic is `LKJCI001`, profile at offset 8 is exactly 5, and offset 112 is exactly 1.
Mode 0 retains profile 4; a mode-1 profile-4 input rejects deterministically.
The default aggregate 8 MiB quota is removed. The current offset representation
admits actual lengths from 160 through 4,294,967,295 bytes before hashing. This is
a representation domain, not a promise that every such input fits the native
runtime: its separate 256 MiB per-value limit and host allocation failures still
apply. The header is 160 bytes, with this layout:

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | Magic |
| 8 | 4 | Profile 5 |
| 12 | 4 | Zero |
| 16 | 8 | Complete byte length L |
| 24 | 8 | Unit count U, 1 through 700 |
| 32 | 8 | Zero for unmetered admission, otherwise a positive u64 allowance |
| 40 | 32 | BLAKE3 of `[0,40)` concatenated with `[72,L)` |
| 72 | 8 | Unit directory offset 160 and count U, two u32 words |
| 80 | 8 | Source-object row offset `160+8*U` and count O, 1 through 100000 |
| 88 | 8 | Metadata offset `160+8*U+48*O` and byte length |
| 96 | 16 | Zero |
| 112 | 4 | Artifact selector 1 |
| 116 | 44 | Zero |

Lengths and unit counts have zero high words. The allowance uses all 64 bits:
zero explicitly selects unmetered admission; every positive value must be at
least `4096 + 128*L + 64*(U*U + O)`. There is no default allowance ceiling and no
signed narrowing. The Rust projection API represents this choice as
`Option<NonZeroU64>` and preserves a selected positive value exactly. The native
checker compares the high and low u32 halves lexicographically. A value one below
the required estimate rejects with diagnostic 6; equality succeeds.

This estimate is an optional admission policy, separate from execution steps,
allocation limits and observations. The passes do not decrement it. Their
termination comes from finite, advancing cursors: unit and object record indexes
increase to their validated counts, spans partition the finite input, UTF-8
cursors advance, and metadata searches decrease a bounded binary-search interval.
Calls in the compiled program are not executed during admission.

Before computing the estimate, layout requires at least 336 bytes per unit,
validated metadata extent, and the retained object/owner counts. Even the weaker
bounds `L <= 2^32-1`, `U <= floor(L/336)` and `O <= floor(L/48)` bound the formula
by 10,457,892,146,095,296, below `2^54` and signed I64. The actual 700-unit and
100,000-object guards make this bound more conservative. Generic extent checks
require nonnegative offset/count and positive stride, establish `offset <= L`,
and compare `count <= (L-offset)/stride` before multiplying or adding. A zero
count at the end is valid; an out-of-range offset, zero stride or overflowing
extent rejects deterministically.

Directory rows are `(offset:u32,length:u32)`. Metadata, unit blobs in directory
order and source-object blobs in row order fill the input consecutively with no
overlap, padding or trailing bytes. Unit pointers are local to their unit blob;
outer pointers are absolute. Unit identities are strictly ordered by OwnerKey.

Source rows are 48 bytes: domain u32, zero pool selector u32, offset u32, length
u32, digest32. Keys are strictly increasing in current ObjectKey enum order,
which differs from numeric wire tags: 1, 2, 3, 5, 25, 23. Digests use BLAKE3
derive-key with the domain context and `length:u64be || object_bytes`:

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
map pages are excluded. The source objects retain their ordinary current encodings.

Metadata occupies `312+56*R` bytes, where R is the runtime-owner count:

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 / 16 | 16 each | Nonzero repository / package ID |
| 32 / 64 / 96 / 128 | 32 each | Package revision / semantic revision / semantic state / interface digest |
| 160 / 232 | 72 each | Interface-owner / reference-owner map root |
| 304 | 4 | R, 0 through 100000 |
| 308 | 4 | Zero |
| 312 | 56*R | Runtime-owner rows |

A map root is page digest32, entry count u32 (at most 100000), zero u32, content
digest32. Runtime-owner rows contain OwnerKey ordinal u32, OwnerKind ordinal u32,
nonzero ID16 and object digest32. Rows are strictly ordered by OwnerKey and each
digest names an included Owner object. Supported ordinal pairs are declaration
1 with kinds 4/5/8, type parameter 2 with kind 10, parameter 6 with kind 14, port 10
with kind 18, and target 11 with kind 19. The package revision and both root pages
must exist in the source-object table. The logical interface identity is computed
from the package ID and interface map's content digest/count, using the current
`LKJPIFI1` envelope and `lkjscript.kernel.package-interface.v1` digest context.
It is distinct from the individual interface-owner object identities. Each unit has the metadata's
package ID. If its identity also appears in the runtime-owner table, the kinds
must agree. Runtime metadata is a selected subset, not the complete unit inventory;
a program with no parameters or resource boundary can have no runtime-owner rows.

## Unit projection

Each projected unit retains its independent 336 through 8,388,608-byte extent
restriction. The canonical Rust unit reader independently retains its 8 MiB
encoded-unit boundary. Lifting aggregate input size does not lift either unit
boundary. Every unit starts with a 256-byte descriptor:

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 / 4 / 8 | 4 each | OwnerKey ordinal, OwnerKind ordinal, OwnerKind wire tag (ordinal+1) |
| 12 | 4 | Zero |
| 16 / 32 | 16 each | Nonzero package / owner ID |
| 48 / 80 / 112 / 144 / 176 / 208 | 32 each | Semantic interface, implementation, type, effect, capability, validation-dependency digests |
| 240 | 4 | Table descriptor offset, exactly 256 |
| 244 | 4 | Signature offset, or zero for component/target |
| 248 / 252 | 4 each | Payload tag and offset |

The 80-byte table descriptor consists of ten offset/count pairs. Tables 0–5 hold
package16 plus typed-identity16 references for declarations, fields, cases,
requirements, operations and ports. Table 6 holds type digests32. Tables 7 and 9
hold UTF-8 name/text spans; table 8 (read-view fields) is exactly `(0,0)`.
Each nonempty or empty table starts at the next canonical position. String tables
place all span rows before their consecutive raw bytes. A span is two u32 words,
offset and length; strings have no encoded-length prefix. Table counts are at
most 32768. Literal text is inline; blob-backed text is outside this projection.

Function/external signatures occupy 20 bytes: type-parameter offset/count,
parameter offset/count, result type index. Type parameters are 16-byte
IDs with no constraints. Parameters are 32-byte rows: ID16, name span8, type
index u32, use ordinal u32 (0 unrestricted, 1 borrow, 2 consume). Parameter strings
follow the whole array. Both counts are at most 128. Signatures are pure, with
value results and no implementation, requirement, effect, read-scope or resource
arguments. Type and reference indices are zero-based.

| Owner ordinal | Kind ordinal | Payload tag | Payload |
| --- | --- | --- | --- |
| 1 | 4 | 6 | Signature plus implementation-name span8 |
| 1 | 5 | 7 | Signature plus code descriptor16 |
| 1 | 8 | 9 | Port offset/count8, then port rows16 |
| 11 | 19 | 11 | Component index, port presence, port index, command runner zero; four u32 words |

External names are the 28 primitives supported by the ELF input profile.
Component ports contain port index, function-type index, function-implementation
tag zero and declaration index. Components have no requirements; targets have
no routes. All indices and presence tags must be valid.

Code descriptors contain parameter count, local count, instruction offset and
instruction count. Counts are bounded by 128 locals and 32768 instructions;
parameter counts match the signature. Instructions occupy 32 bytes: tag, A, B,
C (u32 each), signed I64 immediate, extra offset/count (u32 each). Extras follow
the complete instruction array in instruction order. The supported canonical
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

Unused operands and immediate bytes are zero. Unsupported opcodes reject even
in untaken code. This shape admission checks bounds and layout, not abstract
stack execution or correspondence with the raw source graph.

## Current artifact construction and validation

`unit-wire` encodes the current canonical bincode fields, typed ID/digest tags,
unit key, `LKJCUNIT` envelope and object identity. It does not copy an encoded
unit from input. `unit-map` builds the canonical logical radix content digest
and one valid physical leaf, supporting at most 700 bindings. This count is an
implementation guard for the current flat emitter, not a language limit. With U
bindings, 17-byte keys, 65-byte values and common-prefix length p, its exact page
size is `107 + p + U*(88-p)`. At U=700 the worst case is 61,707 bytes, within the
canonical map reader's 65,536-byte page boundary. Removing the guard requires
hierarchical page emission and inclusion of every derived child page in the
artifact's objects and pack. A different valid physical partition may have the
same logical content digest.

The compiler derives the compilation manifest, combines source and generated
objects in ObjectKey order, derives the artifact closure and manifest, and emits
one raw pack in the current `LKJARTFC` container. Pack entry/index/full checksums,
nonce, pack ID, container checksum and all envelopes use the existing contexts
and encodings. The strict Rust codecs and artifact loader are unchanged.

Compiler tests compare VM/reference/native bytes, compare canonical unit and pack
encodings with independent Rust codecs, load emitted artifacts strictly, and
execute independent programs with fixed expected values and trap codes. Native
stage tests compare A2/A3 against the same VM/reference/C1 output. These establish
Tier 1 backend behavior only; source semantics, lexical phase arenas and semantic
source-graph reproduction remain required for Tier 2.

## Capacity and allocation boundaries

The mode-1 aggregate byte quota is the only product capacity ceiling removed by
profile 5. The 700-unit flat-map guard, 100,000 source objects and metadata owners,
8 MiB projected units, per-table/instruction limits and 128-local limit remain.
Mode 0 retains its existing profile, byte bound, type representation and fuel.
No `LKJSG001`, canonical unit, map, pack or `.lkja` envelope changes are coupled
to this cutover.

The Rust exporter checks every projection extent and serialized u32 conversion.
Projection buffers reserve through `Vec::try_reserve` before resizing. These
failures return `compiler_artifact_input_unsupported`; unrelated standard Rust
allocations in decoding, unit/reference collection and sorting retain their host
allocation behavior. The exporter also rejects the retained unit, source-object,
metadata-owner, table, signature, local, instruction and operand count bounds.
These checks do not replace native framing or whole supplied-unit validation.

Capacity evidence uses a publicly authored package with 700 units, including 699
unused functions with distinct 6,144-byte inline text literals. Tests require the
actual mode-1 input to exceed 8 MiB, preserve the complete expected unit identity
set and each text association, and compare exact source/unit object sets and
bytes after independent strict loading. C1, C2 and C3 consume the same large
input and produce identical artifacts. The small entry evaluates to 42 in both
engines. These are Tier 1 capacity controls; they do not establish source-driven
E2/E3, native phase reclamation, or capacity up to the u32 representation maximum.
