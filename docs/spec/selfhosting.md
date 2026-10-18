# Semantic compiler self-hosting contract

The acceptance target is Tier 2: an lkjscript compiler consumes its complete
authoritative source-graph closure, owns structural and semantic admission and
normalization, and emits both current `.lkja` and standalone Linux x86_64 ELF.
Tier 1 consumes Rust-admitted normalized input. Tier 3 additionally replaces text
authoring, repository packs and package transport. Neither Tier 1 nor Tier 3 is a
substitute for Tier 2. This document does not claim Tier 2 completion.

## Verified starting inventory

The clean initial checkout and fetched origin/main were
`b83b6dd5dc235daaef0483d10342988f7712085c`. The existing safe Rust ELF emitter
supported checked I64, Bool, Unit, text constants, records, lists, calls, closures
and self-tail calls. It rejected Bytes, ByteBuffer, regions and implementation
witnesses. VM and reference execution already supported byte access, BLAKE3,
affine byte buffers and lexical regions. No new language primitive is required
for the initial compiler slice. The checker registry currently defines 29 full
gates; a fresh execution receipt, not that count, establishes acceptance.

Initial `target` usage was 8.5 GiB with 623 GiB free on the filesystem. Cargo-owned
debug and stale package artifacts were cleaned using Cargo while no other build
was active. Use one checkout and `$PWD/target`; preserve unrelated storage and
retain compact evidence before removing task-owned stage binaries.

## Current canonical input: LKJCI1

All integers are fixed-width little-endian. Magic is `LKJCI001`; the only currently
admitted profile is `1`, a normalized I64 stack function. Unknown profiles reject,
with no fallback reader. Profile 1 explicitly retains Rust semantic preprocessing.
It is an end-to-end backend slice, not the eventual supported language profile.

| Header offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | Magic |
| 8 | 4 | Profile and flags, exactly 1 |
| 12 | 4 | Root declaration index, exactly 0 |
| 16 | 8 | Actual complete input byte length |
| 24 | 8 | Declaration count, exactly 1 |
| 32 | 8 | Traversal fuel, at least node count + 3 and at most 4096 |
| 40 | 32 | BLAKE3 of bytes `[0,40)` concatenated with `[72,length)` |

The digest binds all header fields except itself, all tables and all nodes. The
decoder bounds the actual input before hashing or table access. Header lengths,
counts and fuel require zero high words before low-word arithmetic. The only
valid extent is `168 + 16 * node_count`, for `1 <= node_count <= 1024`.

| Offset | Table | Encoding |
| --- | --- | --- |
| 72 | Directory | Four `(u32 offset, u32 count)` pairs: `(104,1)`, `(120,1)`, `(136,1)`, `(168,node_count)` |
| 104 | Module | Four u32 words: first declaration 0, count 1, two zero reserved words |
| 120 | Type | Four u32 words: tag 1 (I64), three zero reserved words |
| 136 | Declaration | Eight u32 words: module 0, result type 0, parameter count 0, local count 0, first node 0, node count, two zeros |
| 168 | Nodes | 16-byte rows: u32 tag, zero u32 reserved, 8-byte immediate |

Node tags are 1 (signed two's-complement I64 literal), 2 (checked add), 3 (checked
subtract), 4 (checked multiply), 5 (checked divide), 6 (return). Only literals
have nonzero immediates. Binary operators consume the two latest stack values
in source order. Return must be the final node with exactly one stack value.
There are no branch or cyclic references in this profile. Every node, including
trailing unreachable content, is checked. The bounded tail traversal visits each
node once and charges the three descriptor rows; attacker fuel cannot raise the
implementation bound. Canonical singleton tables avoid identifier-order choices.

`src/platform/compiler/selfhost_input.rs` is a test-only Rust exporter from the
admitted `NormalizedProgram`. It translates literal/arithmetic/return instructions,
without evaluating expressions or emitting machine code. Unsupported source
operations reject with `compiler_input_unsupported`. The eventual graph profile
must replace this preprocessing boundary, retain complete declaration/type/callable
closures, and enforce semantic types, affine use, loans, brands and effects.

## Compiler component and artifact contract

`tools/selfhost/project` is the accepted meaning authority. The `.lkjc` files are
maintained authoring proposals, admitted through ordinary public operations.
`ci::validate` decodes LKJCI1. `elf::compile-elf` returns a structural record with
`diagnostic: I64` and `image: Bytes`. Zero means success. Rejections always return
empty image bytes; no output artifact is published by the component.

| Code | Rejection |
| --- | --- |
| 1 | Magic |
| 2 | Unsupported profile or flags |
| 3 | Input extent/count bound |
| 4 | Content digest |
| 5 | Noncanonical table, index, reserved word or root |
| 6 | Traversal fuel |
| 7 | Unsupported/malformed instruction |
| 8 | Invalid stack or terminal return |

Validation precedence is extent, magic, profile, header lengths, exact node extent,
digest, tables, fuel, then nodes in order. Arithmetic used for offsets is checked
I64 over already bounded counts. Output is at most 70,656 bytes, entirely owned
by a ByteBuffer until freeze. Header padding is zero and instruction slots use
explicit NOP padding. There are no paths, timestamps, random IDs or host addresses
in output bytes.

The emitted ELF has an RX load segment, non-executable stack, entry address
`0x401000`, and no interpreter or dynamic dependencies. Each input instruction
generates runtime machine code in a 64-byte slot; arithmetic is not evaluated at
compile time. The emitted runtime uses the process stack for at most 1024 scalar
operands, decimal formatting and Linux write/exit syscalls. It has no language
calls, so no System V call-frame alignment obligation is introduced in this slice.
Successful output is the I64 JSON value without newline. Integer traps emit the
VM diagnostic to stderr and exit 70 with empty stdout; extra arguments exit 64.
Writes retry EINTR and continue partial transfers; other errors exit nonzero.
See the [Linux write contract](https://man7.org/linux/man-pages/man2/write.2.html).
Native execution is unmetered, subject to process limits; input traversal bounds
are separate from VM instruction/allocation quotas.

## Dependency inventory and next cutover

| Required by the existing compiler component | Available execution | Native work still required |
| --- | --- | --- |
| I64 arithmetic/comparison, Bool, calls and tail calls | VM, reference, host ELF | Existing native implementation |
| Bytes length/get/slice/concat/equality/from-list/from-text | VM, reference | Native owned byte representation and checked operations |
| ByteBuffer empty/push/length/freeze, affine calls and cleanup | VM, reference | Native bounded storage, loans/moves and reclamation |
| BLAKE3 digest | VM/reference neutral Rust primitive | Native implementation or lkjscript implementation before C1 self-produced stages |
| List literals/length/get and result records | VM, reference, host ELF | Bytes-bearing result/entry support |
| Bounded binary input and atomic output staging | Host harness | Native stdin loop; explicit output selector, diagnostics and exits |

This first component has no phase-local graph arena: it makes one linear bounded
pass over immutable input and emits into owned buffers. Region-native compiler
tables become necessary as source decoding, type environments, normalization and
relocations grow. It does not claim region-exclusive execution or reduced RSS.
The next dependency-justified slice is native byte/buffer execution and binary
filter I/O for this exact compiler workload, followed by backend closure and
semantic admission. Optional optimizers and parallel compilation are deferred.

## Required final profile and proof

`P_self` must include the complete transitive compiler source closure and its
standard declarations, plus independent programs exercising I64 traps, Bool,
Unit, Text, Bytes, structural/nominal records and choices, sequences/buffers,
lexical region allocation/read/write/freeze, recursion, self-tail calls, closures
and explicit contract/implementation witnesses. Missing features remain known
limitations; profile 1 does not reduce these final requirements. Unsupported
broader constructs, including F64 and service capabilities, must reject before
publication with deterministic diagnostics.

Let S be the immutable source-graph closure, E the declared deterministic target
and flags, C0 the Rust bootstrap, C1=C0(S,E), C2=C1(S,E), C3=C2(S,E). Required native
identity is bytes(C2)==bytes(C3). With Ai=Ci(S,lkja), require A2==A3 and A1==A2 for
the same compiler source. C1==C2 is optional host-backend parity. The self-produced
native stages may use neither Rust semantic preprocessing nor an external VM.
The exporter, initial Rust bootstrap and external atomic launcher are separately
recorded boundaries. Digests prove identity, not universal correctness.

Every slice requires VM/reference compiler output identity, later native identity,
strict `.lkja`/ELF admission, and independent execution of produced programs.
Preserve minimized mismatches before fixes. The current tests cover malformed
framing/tables/fuel/opcodes/stack states, signed limits and arithmetic trap order,
deterministic compilation and a changed compiler diagnostic. Those are Tier 1
tests; they do not establish a C2 mutation proof or any fixed point.

Final acceptance additionally requires complete source S and mutated S' runs,
negative type/affine/borrow/region/effect admission, both output formats, region
phase reclamation, exact stage/environment digests and sizes, all discovered
official changed/fresh-full/release-target gates and a stable verified tree.
Keep forbid(unsafe_code), denied unwrap/expect/panic, no-Python, one current format,
and ownership-aware disk bounds. `docs/status.md` distinguishes current attempts
from accepted work; compact receipts carry observed execution evidence.
