# Region types and storage

Region identity is a distinct kind: an exact region-constrained parameter or a
lexical brand. It is not a runtime value. `region`, `rel-ptr`, `region-slice` and
`region-cap` take that identity; pointer and slice element positions take value
types. Generic, compiler and package admission retain exact binder identities,
including unused types. These types cannot cross ordinary data/JSON boundaries.

| Capability mode | Custody | Transfer | Shared read |
| --- | --- | --- | --- |
| `unique` | Affine owner | Yes | No |
| `(shared SCOPE)` | Scoped borrowed view | No | Within the exact source scope |
| `frozen` | Affine owner | Yes | Yes |
| `pinned` | Affine owner | No | No |

The storage owner reserves the caller's exact capacity before entering the region.
Zero capacity is valid and admits no cells. Its 64-byte-aligned image contains a
64-byte header, a cell-start bitmap and aligned cells. Each cell records its exact
layout key, stride, count and alignment. Relative pointers occupy eight bytes;
slices add an eight-byte count. Access requires a live matching brand and exact
allocation/layout bounds. Freeze moves custody without copying. Release drops one
buffer without traversing cells; cycles have no per-cell reference count.

There is no fixed live-region slot allowance. The arena grows its slot inventory
fallibly and reuses released slots with checked generations; a stale pointer or
capability never revives through reuse. Capacity, layout extent and host allocation
failures remain checked. A region's explicit capacity and a caller's image-byte
allowance remain finite policies with exact zero, exact-fit and one-over behavior.

## Current slab format

The current image begins with `LKJREGN2` and profile 2. Previous slab magic and
profiles are rejected. All integer fields below use little-endian encoding.

| Header offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | `LKJREGN2` magic |
| 8 | 2 | Current canonical schema epoch |
| 10 | 2 | Slab profile 2 |
| 12 | 4 | Zero reserved bytes |
| 16 | 8 | Explicit cell-area capacity |
| 24 | 8 | Aligned cell-area start |
| 32 | 8 | Used image extent |
| 40 | 8 | Allocated cell count |
| 48 | 16 | Zero reserved bytes |

The bitmap occupies `ceil(capacity / 64)` bytes after the header; its bits mark
eight-byte cell positions. The cell area begins at the next 64-byte boundary.
Each cell's 64-byte header immediately precedes its aligned payload: a 32-byte
layout key, then eight-byte stride, count, alignment and zero reserved fields.
Stride and count multiply with checked arithmetic. Even zero-sized or empty arrays
retain a distinct cell position and their exact full-width count. Alignment remains
a physical parameter: a power of two from 1 through 64.

Image and cell layout hashing use the `lkjscript.region.image.v2` and
`lkjscript.region.cell.v2` domains. Pointer-item shape fingerprints use the
`lkjscript.region.type.v2` domain: each node hashes its form tag, framed field
names and child digests, with a distinct tag for the relative region identity.
This framing distinguishes nested record boundaries. Complete image admission checks the header,
bitmap, ordered cell extents, exact metadata, padding and final extent before
exposing any read. High words belong to the full-width fields and are not reserved.

The bytecode VM and canonical interpreter both convert nonnegative I64 region
capacities and offsets to u64. Converting a readable relative offset back to I64
is checked. Allocation also checks the host `usize` and allocation-layout bounds
before fallible reservation; the wire width does not promise physical backing.

## Layout and codec lifecycle

Runtime cell layouts use a flat node inventory with child indexes. Layout
derivation, pointer-layout fingerprints, ordinary cell checking, encoding and
decoding use explicit worklists. Completed type nodes and pointer fingerprints
are memoized by exact type identity so shared type graphs do not expand into
repeated trees. Inline type cycles reject. Pointer edges remain
relative handles and do not introduce recursive ownership of region cells.
Partially decoded ordinary values use the existing iterative raw-value cleanup
owner on quota failure or cancellation. Dropping a layout does not recurse through
nested layout objects.

Kernel region escape checks inspect all actual type arguments and each nominal
member schema without generating an unbounded sequence of generic substitutions.
Region-specific inline-layout checks are iterative in both independent admission
paths. Broader recursive kernel and canonical read-view preparation paths retain
their existing depth guards and proof policies; this format change does not claim
those separate admission families are unbounded.

`region-mmap-save` and `region-mmap-load` name persistence of this relative-offset
slab format. They require first-party safe Rust, without OS memory mappings,
fs-verity or external mapping dependencies. Save writes the exact image bytes with
a cryptographic digest through the existing data store. Load reads those bytes
into an aligned slab, verifies the digest and complete structural layout, and
exposes scoped reads without deserializing individual nodes.

Both the checked bytecode VM and the independent canonical interpreter execute
`region-with`, `region-alloc`, `region-read`, `region-write`, `region-freeze`,
`region-resolve`, `region-offset`, `region-mmap-save` and `region-mmap-load`.
Lexical cleanup runs on success, trap and cancellation. Compiler control-flow
admission rejects edges across a region lifetime. Runtime arena and generation
checks reject stale pointers and foreign custody. Cells currently contain Unit,
Bool, I64, same-region relative pointers and structural records of those values.
The slice type and storage array API do not yet have language array expressions.
The native backend still needs region execution and compiler allocation domains;
this slab-format change does not establish native phase reclamation.

Frozen capabilities lend read access to joined parallel children. Unique custody
can move into a child without copying the slab. The scoped executor holds its
process worker reservation until the child joins. Ordinary function calls can
return unique custody within the lexical lifetime. Raw data/JSON boundaries cannot
export a capability or pointer.

Persistence requires an exact DataStore get/put grant and an active transaction.
Save stages the unchanged image and returns its BLAKE3 digest; the transaction owns
commit. Load verifies that digest, copies once into an aligned buffer and admits
the complete header, bitmap and cell layouts before exposing reads. Neither load
nor cleanup reconstructs individual nodes. A failed condition or invalid image
does not publish a data revision.

The additive [planner witness](../../examples/dependency-plan/region-witness.lkjc)
stores a condensed cyclic component and its stages using relative pointers,
freezes the slab and reads it from parallel tasks. Its companion
[store witness](../../examples/dependency-plan/region-store-witness.lkjc) persists
the cyclic stage slab and reloads its root. These bounded witnesses leave the
existing planner's IndexedData/BlockRead entrypoints and expectations unchanged.
[Status](../status.md) records verification.
