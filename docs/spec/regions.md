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

The storage owner reserves one bounded allocation before growth. Its 64-byte-aligned
image contains a 64-byte header, a cell-start bitmap and aligned cells. Each cell
records its exact layout key, stride, count and alignment. Relative pointers occupy
four bytes; slices add a four-byte count. Access requires a live matching brand and
exact allocation/layout bounds. Freeze moves custody without copying. Release
drops one buffer without traversing cells; cycles have no per-cell reference count.

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
