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

All integers are little-endian. Magic is `LKJCI001`; only profile `2` is
admitted. Profile 1 and its old reader are removed. This is a Rust-admitted
normalized scalar function closure, an intermediate step toward Milestone D.
It is not the final semantic source-graph profile.

| Header offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | Magic |
| 8 | 4 | Profile and flags, exactly 2 |
| 12 | 4 | Root function index |
| 16 | 8 | Actual complete input byte length |
| 24 | 8 | Function count, 1 through 256 |
| 32 | 8 | Traversal fuel, described below |
| 40 | 32 | BLAKE3 of bytes `[0,40)` concatenated with `[72,length)` |

The decoder bounds the actual input to 248 through 2,097,152 bytes before hashing
or table access. Header lengths, counts and fuel require zero high words before
low-word arithmetic. Total instruction count is 1 through 4096; each function has
at most 128 locals and each instruction has at most 128 incoming stack values.
The root has no parameters and returns I64. Other boundary signatures reject
with code 9. Internal functions may take and return I64, Bool and Unit.

Let F be function count, N instruction count, and B state-blob byte length.
Fuel must be at least `3 + F + 32*N + 6*B` and at most 16,777,216. This covers
bounded descriptor and state-byte visits; it is separate from evaluator steps.
All table scans advance monotonically. Calls may be recursive; input admission
does not recursively execute or traverse the call graph.

| Offset | Table | Encoding |
| --- | --- | --- |
| 72 | Directory | Five `(u32 offset, u32 count)` pairs: `(112,1)`, `(128,3)`, `(176,F)`, `(176+40*F,N)`, `(176+40*F+32*N,B)` |
| 112 | Module | Four u32 words: first function 0, count F, two zeros |
| 128 | Types | Three 16-byte rows: tag 1 (I64), 2 (Bool), 3 (Unit), each followed by three zero words |
| 176 | Functions | 40-byte rows, described below |
| 176+40*F | Instructions | 32-byte rows, described below |
| 176+40*F+32*N | States | Exactly B bytes; ends at the actual input length |

Function rows contain ten u32 words: module (0), result scalar code (type index
plus one), parameter count, local count, first instruction, instruction count,
external code, absolute local-type byte offset, and two zeros. Instruction ranges
partition the whole instruction table in function order. A code function has
external code zero and at least one instruction. An external has zero instructions
and exactly as many locals as parameters. Its signature must match its primitive.

Each function's blob contribution starts with one scalar-code byte per local;
parameter types occupy its first parameter-count bytes. Each instruction then
contributes one initialization byte (0 or 1) per local followed by one scalar-code
byte per incoming operand. All offsets equal the next canonical blob position,
including empty states. Layout admission checks every descriptor and byte range
before control-flow admission follows any cross-reference.

Instruction rows contain u32 tag, operands A and B, a zero reserved word, an
8-byte immediate, u32 absolute state offset and u32 incoming stack depth.

| Tag | Operation | Operands |
| --- | --- | --- |
| 1 | I64 literal | Signed two's-complement immediate |
| 2 | Bool literal | Immediate exactly 0 or 1 |
| 3 | Unit | None |
| 4 / 5 | Load / store local | A is local index |
| 6 | Drop operand | None |
| 7 / 8 | Jump / jump if false | A is a forward instruction index within the function |
| 9 / 10 | Call / tail call | A is function index; B is argument count |
| 11 | Return | None |

Unused operands and immediates are zero. Every instruction, including untaken and
unreachable syntax, must have a supported form and valid transfer. Entry states
have an empty operand stack and initialized parameters only. Local reads require
initialization and push the declared type. Stores require the local's type;
storing Unit to a non-Unit local clears its initialization, matching normalized
lexical cleanup. Branches consume Bool. Calls check arity, ordered argument types
and result type. Return has exactly one result; tail calls have exactly their
arguments and the enclosing result type.

Every successor's operand types equal the transferred stack. Its initialized
locals must be a subset of those established on each incoming edge, including
untaken edges. A store may establish its local; a clear cannot establish it.
Thus certificates are checked evidence, not trusted permission to read a local.
No nonterminal instruction can fall out of a function. Backward jumps reject;
source recursion and self-tail calls remain available. Traversal never follows
attacker-controlled edges recursively.

External codes 1–7 are I64 add, subtract, multiply, divide, equal, less and
less-equal. Codes 8–10 are Bool not, and and or. Arithmetic traps and
operand evaluation order match the existing evaluators.

`src/platform/compiler/selfhost_input.rs` is a test-only Rust exporter. It
projects the admitted reachable function closure in stable normalized-index order,
retains instructions and builds checked control-flow certificates. It performs no
expression evaluation or native instruction selection. Unsupported types, effects,
polymorphism, witnesses and instructions reject with `compiler_input_unsupported`.
This preprocessing boundary must be replaced for Tier 2.

## Compiler component and artifact contract

`tools/selfhost/project` owns accepted meaning. The four `.lkjc` files are
maintained authoring proposals admitted through ordinary public operations.
`ci` supplies byte/encoding helpers, `flow` validates the complete input,
`machine` lowers scalar functions, and `elf` frames the executable and runtime.
`elf::compile-elf` returns `{diagnostic: I64, image: Bytes}`. Zero means success;
all rejections return empty bytes before the binary filter writes stdout.

| Code | Rejection |
| --- | --- |
| 1 | Magic |
| 2 | Unsupported profile or flags |
| 3 | Input extent/count bound |
| 4 | Content digest |
| 5 | Noncanonical table, index, state extent, reserved word or root |
| 6 | Traversal fuel |
| 7 | Unsupported/malformed instruction form |
| 8 | Invalid stack, local initialization, primitive signature, call or control-flow edge |
| 9 | Unsupported root signature |

Validation order is outer extent, magic, profile, header lengths/counts, digest,
table extents/content, fuel, complete layout, root signature, then every function's
entry and instruction transfers. Offsets use checked I64 arithmetic over bounded
counts. The output is exactly `5120 + 128*(F+N)` bytes, at most 562,176, independently
owned by a ByteBuffer until freeze. Header/data padding is zero and code padding
is NOP. Output contains no paths, timestamps, random IDs or host addresses.

The emitted ELF has an RX load segment, non-executable stack, entry address
`0x401000`, and no interpreter or dynamic dependencies. Each function has a
128-byte entry slot; every instruction has a 128-byte slot. Scalars and local
frames use the process stack. Calls align RSP to 16 bytes before CALL and retain
the original argument base; the callee observes RSP modulo 16 equal to 8. A self-tail
call replaces parameter slots and returns to its first instruction without a new
frame. Other recursion uses ordinary frames. Independent native probes check the
actual call boundary at sixteen operand depths.

Successful output is the I64 JSON value without newline. Integer traps emit the
VM diagnostic to stderr and exit 70 with empty stdout; extra arguments exit 64.
Writes retry EINTR and continue partial transfers; other errors exit nonzero.
See the [Linux write contract](https://man7.org/linux/man-pages/man2/write.2.html).
Native execution is unmetered and subject to process limits. Input bounds and
traversal fuel are separate from VM instruction/allocation quotas.

Compiler conformance normally uses the evaluators' default finite quotas. The
4096-node boundary case uses the existing trusted foreground policy: it observed
14,511,121 VM instructions and 7,903,698 reference expressions, with respective
cumulative modeled allocation charges of 1,599,109,325 and 1,962,239,106 bytes.
Those are work observations, not live heap or RSS measurements. The default
256 MiB cumulative allocation quota rejects that workload. Native execution remains
unmetered; no runtime quota or verifier policy is weakened for this test.

## Dependency inventory and next cutover

| Required by the existing compiler component | Available execution | Native work still required |
| --- | --- | --- |
| I64 arithmetic/comparison, Bool, calls and tail calls | VM, reference, host ELF | Existing native implementation |
| Bytes length/get/slice/concat/equality/from-list/from-text | VM, reference, native ELF | Covered by native compiler tests |
| ByteBuffer empty/push/length/freeze and affine calls | VM, reference, native process-owned heap | Phase-local region reclamation for later compiler tables |
| BLAKE3 digest | VM/reference neutral Rust primitive; native integer implementation | No host hashing during native execution |
| List literals/length/get and result records | VM, reference, native ELF | Covered by the complete current compiler closure |
| Bounded binary input and atomic output staging | Native stdin/stdout filter; external staging harness | Self-produced compiler stages still required |

This first component has no phase-local graph arena: it makes one linear bounded
pass over immutable input and emits into owned buffers. Region-native compiler
tables become necessary as source decoding, type environments, normalization and
relocations grow. It does not claim region-exclusive execution or reduced RSS.
The host can now build this lkjscript component as a standalone native compiler.
Select its `compile-elf` command target. The exact binary-filter signature and
16 MiB transport bound are specified in [native ELF](native-elf.md); profile 2
still enforces its tighter 2 MiB input bound before hashing. Native output
must equal VM/reference output for positive, malformed and mutated-source cases.
Milestone C is accepted; scalar closure extends toward Milestone D of Tier 1: Rust still admits and exports input programs and
builds the initial compiler. The component cannot compile its own implementation.
The next dependency-justified slice is complete backend closure and `.lkja`
emission, followed by semantic admission. Optional optimizers and parallel
compilation are deferred.

## Required final profile and proof

`P_self` must include the complete transitive compiler source closure and its
standard declarations, plus independent programs exercising I64 traps, Bool,
Unit, Text, Bytes, structural/nominal records and choices, sequences/buffers,
lexical region allocation/read/write/freeze, recursion, self-tail calls, closures
and explicit contract/implementation witnesses. Missing features remain known
limitations; profile 2 does not reduce these final requirements. Unsupported
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
tests; they do not establish a C2 mutation proof or any fixed point.

Final acceptance additionally requires complete source S and mutated S' runs,
negative type/affine/borrow/region/effect admission, both output formats, region
phase reclamation, exact stage/environment digests and sizes, all discovered
official changed/fresh-full/release-target gates and a stable verified tree.
Keep forbid(unsafe_code), denied unwrap/expect/panic, no-Python, one current format,
and ownership-aware disk bounds. `docs/status.md` distinguishes current attempts
from accepted work; compact receipts carry observed execution evidence.
