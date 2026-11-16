# Object graphs

The form is one owning `Arena<T, Kind>` and typed `Handle<Kind>` values.
Handles store an owner namespace, slot index and generation. They do not borrow
the arena, so nodes may store parent, child, sibling and arbitrary observer links
for lifetimes that do not follow lexical scopes. A lookup yields a short-lived
checked view. Mutations take the owner exclusively; multiple handles do not
manufacture multiple mutable views.

`Kind` is a nominal class marker carried as a zero-length array. It does not
require the stored `T` to be Copy: the affine-value test inserts a Vec and moves
it out once. Distinct arenas of one Kind must use distinct application-assigned
owner IDs while any handle from either can remain. There is no hidden global ID
allocator and no claim of automatically unique namespaces. The arena is not
Clone. Memory safety still comes from ordinary checked indexing and loans.

Removal returns the owned value, increments the generation and links the slot
into a free list. Any old handle returns None, including after slot reuse.
The last u64 generation permanently retires a slot rather than wrapping. The
library's private witness reaches that representation edge directly. Insert,
lookup and removal are constant work, except Vec growth. Storage is proportional
to the maximum occupied slots plus retired generations, not the history of
insertions. Handles survive storage relocation.

## Specimen and gate

The specimen builds a four-way tree of 1,000,000 nodes. Every node carries parent,
first-child, previous/next-sibling links, two numeric attributes and two observer
links. Observers may name arbitrary other nodes; the history deliberately retains
removed handles. A fixed xorshift seed drives 2,000 subtree removals and new
subtree insertions. Traversal verifies every parent/sibling relationship, the
live node count and all observer generations. Traversal/removal use explicit
work stacks, so tree height does not consume the machine stack.

Run `bash tools/programs-graph-test.sh` for the gate. It builds the program
directory, runs two graph unit cases and four exact-library cases in native and
independent interpretation, checks nominal handle rejection and four retained
diagnostics, compares three small seeds across both engines, and runs the
million-node native specimen. Each temporary directory is removed on exit.
The authority is exactly Arguments and Stdout.

Add `--witness` to compare the unchanged arena and graph source with Rust at
opt-level 0 and 3, both with overflow checks. Only the entry wrapper changes
to obtain arguments/output through Rust's host library. Rust is optional and
is never needed by the native gate.

## Measured result

One measured run on 2026-11-15 UTC, seed 6840227782638526189; wall time includes
build-tree, mutations, traversal and destruction, excluding compilation.
The load column is the host's one-minute average before and after each run.

| Engine | Wall ms | User/system s | Peak RSS KiB | Load |
| --- | ---: | ---: | ---: | --- |
| lkjscript native | 986 | 0.89 /0.06 | 254,556 | 5.35 -> 5.35 |
| Rust opt-level 0 | 660 | 0.60 /0.04 | 245,908 | 5.35 -> 5.35 |
| Rust opt-level 3 | 274 | 0.21 /0.05 | 246,256 | 5.35 -> 5.35 |

All three outputs agree: 13,546 nodes removed, 16,960 inserted, 1,003,414
remaining, 40,449 stale observers, 1,008,515 allocated slots and checksum
138421207483169. Native time is 1.49x opt0 and 3.60x opt3 in this sample.
The focused pre-measurement checks took 17.731 s, load 5.07 -> 5.35.

Four diagnostic repairs are retained in tests/diagnostics: receiver argument
borrowing and uninitialized-local inference (expected a Rust form), plus
mutable Arguments and unit-returning Stdout.write (rule with no Rust counterpart).
The typed-handle mismatch is a required rejection, not a repair count.
No new IR operation or checker rule was needed.
