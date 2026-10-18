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

All integers are little-endian. Magic is `LKJCI001`; only profile `4` is
admitted. Earlier profile readers are removed. The u32 at offset 112 selects
ELF (0) or artifact (1) emission. Both modes retain Rust semantic admission and
IR export as Tier 1 boundaries. The ELF layout follows; the
[artifact layout](selfhosting-artifact-input.md) supplies canonical unit fields
and source objects. Neither is the final semantic graph profile.

| Header offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | Magic |
| 8 | 4 | Profile and flags, exactly 4 |
| 12 | 4 | Root function index |
| 16 | 8 | Actual complete input byte length |
| 24 | 8 | Function count F, 1 through 1024 |
| 32 | 8 | Traversal fuel |
| 40 | 32 | BLAKE3 of bytes `[0,40)` concatenated with `[72,length)` |

The common decoder bounds actual input to 160 through 8,388,608 bytes before
hashing or table access; ELF mode additionally requires at least 2248 bytes.
Header lengths, counts and fuel require zero high words. In ELF mode there are
1 through 32768 instructions; locals, operand depth and record fields are each
bounded by 128. The root is either `() -> I64` or the exact binary-filter
signature `(Bytes) -> {diagnostic: I64, image: Bytes}`; other signatures reject
with code 9.

Let N be instruction count and B blob byte length. Fuel must be at least
`4096 + F + 64*N + 256*B` and at most 2,147,483,647. This conservative work
certificate covers bounded descriptor/state visits, UTF-8 checks, field-name
comparisons and record permutation scans; it is separate from evaluator steps.
All scans advance monotonically. Calls may recurse, but input admission neither
executes nor recursively traverses the call graph.

| Offset | Table | Encoding |
| --- | --- | --- |
| 72 | Directory | Five u32 offset/count pairs: `(112,1)`, `(128,T)`, `(2176,F)`, `(2176+40*F,N)`, `(2176+40*F+32*N,B)` |
| 112 | Module | Four u32 words: first function 0, count F, two zeros |
| 128 | Types | T sixteen-byte rows; T is 7 through 127; reserved space ends at 2176 and all unused words are zero |
| 2176 | Functions | Forty-byte rows |
| 2176+40*F | Instructions | Thirty-two-byte rows |
| 2176+40*F+32*N | Blob | Exactly B bytes, ending at actual input length |

Type IDs are one-based. IDs 1–6 are I64, Bool, Unit, Text, Bytes and ByteBuffer;
their rows are `[id,0,0,0]`. ID 7 is List<I64>, row `[7,1,0,0]`. IDs 8–127 are
structural records, row `[8,field_count,metadata_offset,0]`. Record metadata comes
first in the blob, in type order. Each record has sixteen-byte field rows
`[child_type,name_offset,name_length,0]`, followed by its consecutive field names.
Names are nonempty UTF-8 in strictly increasing byte order. Child types must be
smaller IDs and cannot be ByteBuffer; this excludes cycles and affine aggregates.
Empty records are permitted. Every offset is the next canonical position.

Function rows contain ten u32 words: module (0), result type, parameter count,
local count, first instruction, instruction count, external code, absolute
local-type byte offset, and two zeros. Instruction ranges partition the table in
function order. A code function has external code zero and at least one instruction.
An external has no instructions and exactly as many locals as parameters; its
signature must match its primitive.

After record metadata, each function contributes local-type bytes, followed by
each instruction's optional payload, initialization flags (0 or 1) per local,
and type bytes per incoming operand. Type 134 denotes a borrowed ByteBuffer and
is permitted only in parameter locals and operand stacks. Owned ByteBuffer type 6
parameters consume custody. Other parameter types are unrestricted. Results cannot
be borrowed buffers. Layout checks all descriptors and ranges before transfers.

Instruction rows contain u32 tag, operands A, B and C, an eight-byte immediate,
u32 absolute state offset, and u32 incoming stack depth.

| Tag | Operation | Operands |
| --- | --- | --- |
| 1 | I64 literal | Signed two's-complement immediate |
| 2 | Bool literal | Immediate exactly 0 or 1 |
| 3 | Unit | None |
| 4 | Load local | A local index; B=0 copy, B=1 buffer borrow |
| 5 | Store local | A local index |
| 6 | Drop operand | Explicitly consumes an owned value |
| 7 / 8 | Jump / jump if false | A forward instruction index within the function |
| 9 / 10 | Call / tail call | A function index; B argument count |
| 11 | Return | None |
| 12 | Text | A blob offset; B UTF-8 byte length; payload precedes state |
| 13 | List<I64> | A=7; B element count |
| 14 | Structural record | A type; B field count; C blob offset of B-byte field permutation |
| 15 | Field | A record type; B canonical field index |
| 16 | Move buffer local | A initialized owned-buffer local index |

Unused operands and immediates are zero. Record permutations contain each field
index exactly once; values are evaluated in source order and placed at canonical
field positions. Lists require I64 elements. Field access requires its record type.
Text rejects invalid UTF-8, including overlong sequences, surrogates and truncation.

Every instruction, including untaken and unreachable syntax, needs a supported
form and valid transfer. Entry stacks are empty and only parameters are initialized.
Reads require initialization; owned buffers require an explicit move or borrow.
A move clears custody and cannot occur while any borrowed operand remains live.
Stores require the local's type; Unit clears a non-Unit local for lexical cleanup.
An initialized owned buffer cannot be overwritten; explicit cleanup/drop requires
no live borrowed operand. Borrowed values cannot be stored, aggregated or returned.

Successor stacks equal the transferred types. Copyable local initialization can
be forgotten; owned-buffer initialization must exactly preserve, establish or
clear custody according to the instruction. Branches consume Bool and check both
edges. Calls check arity and ordered types. Return has exactly its one result;
tail calls have exactly their arguments and the enclosing result type. Neither
may abandon initialized owned-buffer locals. These are checked certificates, not
trusted permission. No instruction falls out of a function. Backward jumps reject;
recursion and self-tail calls remain available.

External codes 1–7 are I64 add, subtract, multiply, divide, equal, less and
less-equal; 8–10 are Bool not, and and or. Codes 11–19 are Bytes length, get,
from-list, from-text, copy, slice, concat, equal and BLAKE3. Codes 20–25 are buffer
empty, push, length, get, freeze and discard; 26–27 are List<I64> length and get.
Code 28 is `core.bytes.blake3-derive-key(Text context, Bytes material) -> Bytes`.
It returns 32 domain-derived bytes; this is BLAKE3 derive-key mode, not a context
prefix passed to unkeyed hashing. It supplies the current `.lkja` object, unit and
container identity dependency. Unknown codes fail layout admission.
Signatures, consumption and argument order match the registered core primitives.
Arithmetic traps and operand evaluation order match the existing evaluators.

`src/platform/compiler/selfhost_input.rs` is a test-only Rust exporter. It
projects the admitted reachable closure in stable normalized-index order, retains
instructions and builds checked flow certificates. A derived tail-dispatch hint
falls back to its original call/cleanup continuation when an owned local remains
live, preserving borrowed calls and lexical cleanup. It performs no expression
evaluation or native instruction selection. The current projection accepts one
explicit closed type instantiation per declaration, needed for generic list
primitives. Inconsistent instantiations, unsupported types, effects, witnesses,
regions and instructions reject with `compiler_input_unsupported`. Rust still
owns source semantic admission and normalization; Tier 2 must replace that boundary.

## Compiler component and artifact contract

`tools/selfhost/project` owns accepted meaning. The `.lkjc` files are maintained
authoring proposals admitted through ordinary public operations. `ci`, `flow`,
`types`, `custody`, `profile` and `values` validate and decode input. `asm`,
`machine` and `aggregate` select native instructions; `heap`, `objects`, `hash`,
`stream`, `runtime` and `elf` emit the runtime and executable framing.
`artifact-input` and `artifact-check` validate artifact projection structure;
`wire`, `unit-wire`, `unit-map`, `pack`, `artifact-order`, `artifact-manifest` and
`artifact` construct current units, maps, manifests, packs and the container.
`elf::compile-elf` returns `{diagnostic: I64, image: Bytes}`. Zero means success;
the explicit input selector chooses ELF or `.lkja` output. All rejections return
empty bytes before the binary filter writes stdout.

| Code | Rejection |
| --- | --- |
| 1 | Magic |
| 2 | Unsupported profile or flags |
| 3 | Input extent/count bound |
| 4 | Content digest |
| 5 | Noncanonical table, index, state extent, reserved word or root |
| 6 | Traversal fuel |
| 7 | Unsupported/malformed instruction form |
| 8 | Invalid types, UTF-8 literal, stack, custody, primitive signature, call or edge |
| 9 | Unsupported root signature |
| 10 | Artifact projection layout, unit form, owner metadata or required reference |
| 11 | Artifact source-object domain, order, extent or digest |

ELF validation order is outer extent, magic, profile/selector, header lengths/counts, digest,
directory/module extents and root index, fuel, types and complete layout, root
signature, then every function's
entry and instruction transfers. Offsets use checked I64 arithmetic over bounded
counts. ELF output is exactly `65536 + 512*F + 128*N + B` bytes. A ByteBuffer owns
it until freeze. Header/data padding is zero and code padding is NOP. It contains
no paths, timestamps, random IDs or host addresses. Literal bytes and record
permutations are copied from the canonical blob as read-only data; this is data,
not a precompiled program or an executable template.

The ELF has an RX load segment, non-executable stack, entry address `0x401000`,
and no interpreter or dynamic dependencies. Runtime code occupies the first
65536 bytes; each function has a 512-byte header and each instruction a 128-byte
slot. Calls align RSP to 16 before CALL and preserve the original argument base;
callees observe RSP modulo 16 equal to 8. Self-tail calls replace parameters and
jump to the first instruction without growing the stack. Other recursion uses
ordinary frames.

I64, Bool and Unit are unboxed. Bytes/Text use a length and data pointer;
ByteBuffer adds capacity. Lists and records use a count followed by scalar or
pointer values. The process-owned native heap uses aligned bump allocation into
RW mmap chunks, with a minimum chunk size of 65536 bytes. Buffer growth retains
old backing storage until process exit; freeze and byte-producing operations make
owned copies. Byte payloads have a 256 MiB per-value limit. This runtime does not
claim lexical region reclamation, bounded process RSS or whole-program zero-copy.

A scalar root prints I64 without a newline. A binary-filter root reads up to
16 MiB from stdin, then writes the successful image as raw bytes; rejection writes
`compiler_rejected:<code>\n` with empty stdout and exits 65. Usage exits 64;
native traps exit 70. The profile's tighter 8 MiB bound applies before hashing.
Read/write loops handle partial transfers and retry EINTR; other I/O errors exit
nonzero. An external launcher owns atomic file publication. Native execution is
unmetered and subject to process limits. Input bounds and traversal fuel differ
from VM instruction/allocation quotas.

Compiler workload measurements use the existing trusted foreground evaluator
policy where needed. Cumulative modeled allocation charges measure work, not
live heap or RSS. No runtime quota or verifier policy is raised for these tests.

At accepted commit `108906b5`, profile-4 execution exported 244 reachable functions
and 16,111 instructions in 794,236 bytes. VM, reference and Rust-built native compiler execution
emitted the same 2,519,420-byte ELF; C2 and C3 are byte-identical. Artifact projection
contains 247 canonical units and 4,126,413 input bytes. A1, A2 and A3 are identical
4,349,511-byte bundles, also matching VM/reference emission. Every unit matches
the current canonical encoding; the unchanged strict loader admits the complete
artifact and both evaluators execute its compiler. Independent programs cover
branches, recursion, tail calls, integer limits, overflow, division by zero and
trap order. Native stages also emit hash programs matching 35 published vectors.
Both stages compile a source mutation into ELF and artifact; the resulting
compiler emits programs with the deliberately changed argument diagnostic.

ELF emission observed 105,866,005 VM instructions and 59,048,940 reference
expressions, with cumulative modeled allocation of 10,275,753,351 and
13,326,093,983 bytes. Artifact emission observed 296,112,311 VM instructions and
153,493,373 reference expressions, with cumulative modeled allocation of
31,909,160,503 and 36,294,774,728 bytes. Artifact scans use tail calls; the complete
compiler run has maximum call depth 19 on VM and 20 on reference. These are
work and call-depth observations, not live heap or RSS measurements. This is
Tier 1 reproduction from Rust-admitted input, not semantic self-hosting.

## Dependency inventory and next cutover

| Required by the existing compiler component | Available execution | Native work still required |
| --- | --- | --- |
| I64 arithmetic/comparison, Bool, calls and tail calls | VM, reference, host ELF | Existing native implementation |
| Bytes length/get/slice/concat/equality/from-list/from-text | VM, reference, native ELF | Covered by native compiler tests |
| ByteBuffer empty/push/length/freeze and affine calls | VM, reference, native process-owned heap | Phase-local region reclamation for later compiler tables |
| BLAKE3 unkeyed and domain-derived digest | VM/reference neutral Rust primitives; native integer implementation | No host hashing during native execution |
| List literals/length/get and result records | VM, reference, native ELF | Covered by the complete current compiler closure |
| Bounded binary input and atomic output staging | Native stdin/stdout filter; external staging harness | Native reproduction is a Tier 1 verification obligation |

This component has no phase-local graph arena: it scans immutable input and
constructs derived rows and output in owned buffers. Artifact emission also sorts
derived object rows and builds the compilation map's logical radix digest.
Region-native compiler tables become necessary as source decoding, type environments, normalization and
relocations grow. It does not claim region-exclusive execution or reduced RSS.
The host can now build this lkjscript component as a standalone native compiler.
Select its `compile-elf` command target. The exact binary-filter signature and
16 MiB transport bound are specified in [native ELF](native-elf.md). Native output
must equal VM/reference output for positive, malformed and mutated-source cases.
Milestone D both-output backend closure is accepted at `108906b5`; its exact
official receipt records the fresh gate matrix and remaining Rust boundaries.
E/F require semantic admission, phase-local
regions and the native source-graph fixed point. Optimizers and parallel compilation
remain deferred.

## Source-record decoding groundwork

The source readers consume current kernel object bytes, before a semantic graph
profile replaces the host-normalized LKJCI1 input. They do not change profile 4 or
make its `compile-elf` entrypoint a semantic compiler. `source-shape::type` and
`source-owner::owner` return 0 for supported record shapes, 12 for malformed
encoding and 13 for a recognized unsupported form. Rejection produces no artifact.
A supported shape is not permission to execute or a proof of semantic validity.

`source-read` checks ranges before access, minimal bincode integers, complete
signed I64 zigzag values, typed nonzero 16-byte identities, typed 32-byte digests,
bounded arrays, UTF-8 strings, canonical names and optional references. Current
envelopes require exact magic, epoch, length and domain-derived checksum. Type
payloads have a 1 MiB bound and owner payloads a 4 MiB bound. Array counts are at
most 100,000, names have 1 through 128 bytes and inline text at most 65,536 bytes.
Negative indices and overflow-sized extents reject before arithmetic or access.

The type reader covers Unit, Bool, I64, Bytes, Text, StaticText, ByteBuffer, type
parameters, named/applied types, structural records, lists and function types.
It checks nonempty strictly ordered structural fields and positive Applied arity.
The owner reader checks headers and the supported module, declaration, parameter,
type-parameter, binding, expression, field, case, port and target record shapes.
It retains package-qualified references and ordered expression operands. Pure
functions, externals, constants, records/variants, components and tests have
structural readers; other effects, borrowed results, witness parameters and
read-scope metadata currently reject explicitly. Expression forms 0 through 16
cover literals, locals, branches, lets, calls/function values/invocation, records,
variants, fields and lists. Known broader forms reject with code 13 before their
unsupported payload is interpreted; this does not classify that payload as valid.

Tests use the current Rust codecs as an independent structural oracle and also
construct payloads directly, including malformed payloads with valid checksums.
The test-only batch framing is repeated `(kind:u8, length:u32LE, object:Bytes)`
rows, where kind 0 means type and 1 means owner. It is not a public compiler-input
schema. All current source records are tested together on VM, reference and native
execution, including records outside the backend entrypoint's reachable closure.

These helpers do not establish object-to-map binding, dependency closure,
reference existence, unique ownership, array identity uniqueness, graph acyclicity,
expression typing, affine/borrow/region/effect rules or witness summaries. Those
checks and normalization remain Milestone E work. Decoding currently scans bytes
and returns scalar cursors; no phase-local graph arena or region-native memory
claim is made. The final graph profile still needs bounded traversal work and
lexical Region tables, followed by the Milestone F source-graph fixed point.

## Required final profile and proof

`P_self` must include the complete transitive compiler source closure and its
standard declarations, plus independent programs exercising I64 traps, Bool,
Unit, Text, Bytes, structural/nominal records and choices, sequences/buffers,
lexical region allocation/read/write/freeze, recursion, self-tail calls, closures
and explicit contract/implementation witnesses. Missing features remain known
limitations; profile 4 does not reduce these final requirements. Unsupported
broader constructs, including F64 and service capabilities, must reject before
publication with deterministic diagnostics.

Let S be the immutable source-graph closure, E the declared deterministic target
and flags, C0 the Rust bootstrap, C1=C0(S,E), C2=C1(S,E), C3=C2(S,E). Required native
identity is bytes(C2)==bytes(C3). With Ai=Ci(S,lkja), require A2==A3 and A1==A2 for
the same compiler source. C1==C2 is optional host-backend parity. The self-produced
native stages may use neither Rust semantic preprocessing nor an external VM.
The exporter, initial Rust bootstrap and external atomic launcher are separately
recorded boundaries. Digests prove identity, not universal correctness.

Every slice requires VM/reference/native compiler output identity,
strict `.lkja`/ELF admission, and independent execution of produced programs.
Preserve minimized mismatches before fixes. The current tests cover malformed
framing/tables/fuel/opcodes/states, signed limits, arithmetic trap order, calls,
branches, recursive and self-tail calls, initialization and stack alignment,
deterministic compilation and a changed compiler diagnostic. Those are Tier 1
tests. Native reproduction from normalized input is a Tier 1 fixed point; it does
not establish the required semantic source-graph fixed point. Artifact closure
additionally requires strict loading and A2/A3 byte equality.

Final acceptance additionally requires complete source S and mutated S' runs,
negative type/affine/borrow/region/effect admission, both output formats, region
phase reclamation, exact stage/environment digests and sizes, all discovered
official changed/fresh-full/release-target gates and a stable verified tree.
Keep forbid(unsafe_code), denied unwrap/expect/panic, no-Python, one current format,
and ownership-aware disk bounds. `docs/status.md` distinguishes current attempts
from accepted work; compact receipts carry observed execution evidence.
