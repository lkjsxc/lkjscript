const fs = require('fs');
const path = 'docs/spec/selfhosting.md';
let s = fs.readFileSync(path, 'utf8');
const begin = s.indexOf('All integers are little-endian.');
const end = s.indexOf('## Compiler component and artifact contract');
s = s.slice(0,begin) + `All integers are little-endian. Magic is \`LKJCI001\`; only profile \`3\` is
admitted. Earlier profile readers are removed. This is a Rust-admitted normalized
byte/aggregate function closure for Tier 1, not the final semantic graph profile.

| Header offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | Magic |
| 8 | 4 | Profile and flags, exactly 3 |
| 12 | 4 | Root function index |
| 16 | 8 | Actual complete input byte length |
| 24 | 8 | Function count F, 1 through 1024 |
| 32 | 8 | Traversal fuel |
| 40 | 32 | BLAKE3 of bytes \`[0,40)\` concatenated with \`[72,length)\` |

The decoder bounds actual input to 2248 through 8,388,608 bytes before hashing or
table access. Header lengths, counts and fuel require zero high words. There are
1 through 32768 instructions; locals, operand depth and record fields are each
bounded by 128. The root is either \`() -> I64\` or the exact binary-filter
signature \`(Bytes) -> {diagnostic: I64, image: Bytes}\`; other signatures reject
with code 9.

Let N be instruction count and B blob byte length. Fuel must be at least
\`4096 + F + 64*N + 256*B\` and at most 2,147,483,647. This conservative work
certificate covers bounded descriptor/state visits, UTF-8 checks, field-name
comparisons and record permutation scans; it is separate from evaluator steps.
All scans advance monotonically. Calls may recurse, but input admission neither
executes nor recursively traverses the call graph.

| Offset | Table | Encoding |
| --- | --- | --- |
| 72 | Directory | Five u32 offset/count pairs: \`(112,1)\`, \`(128,T)\`, \`(2176,F)\`, \`(2176+40*F,N)\`, \`(2176+40*F+32*N,B)\` |
| 112 | Module | Four u32 words: first function 0, count F, two zeros |
| 128 | Types | T sixteen-byte rows; T is 7 through 127; reserved space ends at 2176 and all unused words are zero |
| 2176 | Functions | Forty-byte rows |
| 2176+40*F | Instructions | Thirty-two-byte rows |
| 2176+40*F+32*N | Blob | Exactly B bytes, ending at actual input length |

Type IDs are one-based. IDs 1–6 are I64, Bool, Unit, Text, Bytes and ByteBuffer;
their rows are \`[id,0,0,0]\`. ID 7 is List<I64>, row \`[7,1,0,0]\`. IDs 8–127 are
structural records, row \`[8,field_count,metadata_offset,0]\`. Record metadata comes
first in the blob, in type order. Each record has sixteen-byte field rows
\`[child_type,name_offset,name_length,0]\`, followed by its consecutive field names.
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
Signatures, consumption and argument order match the registered core primitives.
Arithmetic traps and operand evaluation order match the existing evaluators.

\`src/platform/compiler/selfhost_input.rs\` is a test-only Rust exporter. It
projects the admitted reachable closure in stable normalized-index order, retains
instructions and builds checked flow certificates. It performs no expression
evaluation or native instruction selection. The current projection accepts one
explicit closed type instantiation per declaration, needed for generic list
primitives. Inconsistent instantiations, unsupported types, effects, witnesses,
regions and instructions reject with \`compiler_input_unsupported\`. Rust still
owns source semantic admission and normalization; Tier 2 must replace that boundary.

` + s.slice(end);
s = s.replace('The four `.lkjc` files are\nmaintained authoring proposals admitted through ordinary public operations.\n`ci` supplies byte/encoding helpers, `flow` validates the complete input,\n`machine` lowers scalar functions, and `elf` frames the executable and runtime.', 'The `.lkjc` files are maintained\nauthoring proposals admitted through ordinary public operations. `ci`, `flow`,\n`types`, `custody`, `profile` and `values` validate and decode input. `asm`,\n`machine` and `aggregate` select native instructions; `heap`, `objects`, `hash`,\n`stream`, `runtime` and `elf` emit the runtime and executable framing.');
s = s.replace('Invalid stack, local initialization, primitive signature, call or control-flow edge', 'Invalid types, UTF-8 literal, stack, custody, primitive signature, call or edge');
const outBegin = s.indexOf('counts. The output is exactly');
const outEnd = s.indexOf('## Dependency inventory and next cutover');
s = s.slice(0,outBegin) + `counts. Output is exactly \`65536 + 512*F + 128*N + B\` bytes. A ByteBuffer owns
it until freeze. Header/data padding is zero and code padding is NOP. It contains
no paths, timestamps, random IDs or host addresses. Literal bytes and record
permutations are copied from the canonical blob as read-only data; this is data,
not a precompiled program or an executable template.

The ELF has an RX load segment, non-executable stack, entry address \`0x401000\`,
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
\`compiler_rejected:<code>\\n\` with empty stdout and exits 65. Usage exits 64;
native traps exit 70. The profile's tighter 8 MiB bound applies before hashing.
Read/write loops handle partial transfers and retry EINTR; other I/O errors exit
nonzero. An external launcher owns atomic file publication. Native execution is
unmetered and subject to process limits. Input bounds and traversal fuel differ
from VM instruction/allocation quotas.

Compiler workload measurements use the existing trusted foreground evaluator
policy where needed. Cumulative modeled allocation charges measure work, not
live heap or RSS. No runtime quota or verifier policy is raised for these tests.

` + s.slice(outEnd);
s = s.replace('Self-produced compiler stages still required', 'Native reproduction is a Tier 1 verification obligation');
const nextBegin = s.indexOf('Select its `compile-elf` command target.');
const nextEnd = s.indexOf('## Required final profile and proof');
s = s.slice(0,nextBegin) + `Select its \`compile-elf\` command target. The exact binary-filter signature and
16 MiB transport bound are specified in [native ELF](native-elf.md). Native output
must equal VM/reference output for positive, malformed and mutated-source cases.
Milestone C and the scalar closure step are accepted. The current byte/aggregate
extension targets complete native backend closure. Milestone D additionally
requires current \`.lkja\` emission; E/F require semantic admission, phase-local
regions and the native source-graph fixed point. Optimizers and parallel compilation
remain deferred.

` + s.slice(nextEnd);
s = s.replace('profile 2 does not reduce', 'profile 3 does not reduce');
fs.writeFileSync(path,s);
