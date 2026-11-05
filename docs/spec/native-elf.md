# Native Linux x86_64 commands

`lkjscript build --native-elf OUTPUT [--target NAME]` derives a standalone static
ELF executable from an admitted `NormalizedProgram`. Select a command target by
name when the project has more than one. Compilation admits the whole reachable
function closure, including untaken branches, before create-new publication.
The executable is published atomically with mode `0755`; existing outputs are
never replaced or chmodded.

An I64-entry program accepts one decimal command-line argument per I64 entry
parameter. For example, an arithmetic target with two I64 parameters runs as
`./arithmetic 7 3`. Successful output is exactly the typed result's JSON bytes,
without a trailing newline. Invalid arguments exit 64. Checked integer and index
traps exit 70 with the VM diagnostic code on stderr and no result on stdout.

A selected entry with the exact signature `(Bytes) -> {diagnostic: I64, image:
Bytes}` is a binary filter. It accepts no command-line arguments, reads stdin to
EOF without a default transport quota, and writes the raw image only after the
entry returns diagnostic zero. Nonzero diagnostics produce `compiler_rejected:N`
plus newline on stderr, empty stdout and exit 65. Read errors produce
`native_input` and exit 65; argument errors exit 64 and runtime traps or reported
allocation failures exit 70. Read and write loops retry EINTR and continue partial transfers.
Output write errors exit nonzero (74 for observed syscall errors). The target's
signature selects this boundary at build time; neither input nor output is sniffed.
An external launcher may stage, sync, chmod and atomically publish successful
bytes; the generated program performs no filesystem publication itself.

The input reader owns one separate anonymous mapping, initially 64 KiB. This
initial capacity is a tuning choice. Checked geometric growth uses Linux
[`mremap(MREMAP_MAYMOVE)`](https://man7.org/linux/man-pages/man2/mremap.2.html)
before any language descriptor or view refers to the input. A full mapping is
first probed with one scratch byte: EOF completes without further allocation;
a successful probe is appended after growth. The reader keeps the
only base pointer and a relative length, updates the base after every remap, and
creates the immutable `Bytes` descriptor at EOF. The mapping then stays stable
through execution and output, including outputs that alias or slice the input.
No old growth mapping is retained. This input lifetime rule is separate from the
[pure phase interface](native-phases.md), which receives caller-owned immutable
input and materializes an independent output before reclaiming scratch.

The length ABI remains a nonnegative I64. Capacities are positive powers of two
from 64 KiB through 2^62; doubling is checked before the signed representation
would overflow. Reads always have positive available capacity, retry EINTR, and
advance by the returned count. Reported mapping failure or unrepresentable growth
produces `native_allocation`, empty stdout and exit 70. Linux address-space and
caller-selected OS limits still apply. The separate byte-operation, generated
image and compiler-format bounds remain unchanged by this transport contract.

The initial native instruction set covers Unit, Bool, I64, text constants,
structural and nominal records, lists, fields, branches, ordinary and generic
calls, bound function values, invocation and graph tail transfer. The ordinary
nominal extension adds closed nongeneric acyclic copyable Variant construction
and exhaustive Match inside the existing entry/result boundaries. It includes
checked I64 arithmetic/comparisons/conversion, boolean operations and list
length/get/append. Immutable byte intermediates support length/get/equality,
construction from octet lists or raw UTF-8 text, copying, slicing and concatenation.
Byte construction and indexing preserve exact octet/index/range traps. Byte construction
intrinsics allocate independent bounded storage; JSON string encoding is separate
from the raw UTF-8 view. Unkeyed and domain-derived 32-byte BLAKE3 run as native
integer code, with bounded chunk-tree recursion and independent block/chunk/tree
test vectors. `core.bytes.blake3-derive-key(Text, Bytes) -> Bytes` hashes the exact
UTF-8 context with DERIVE_KEY_CONTEXT, then hashes the material with the context
key and DERIVE_KEY_MATERIAL. The intermediate key uses 32 bytes of lexical stack
storage; both inputs use checked nonnegative I64 byte extents. This neutral operation is
required by current artifact and object identities.
ByteBuffer empty/push/append-slice/append-i64-le/get/length/freeze/discard preserve
admitted exclusive mutation and scoped read-only borrowing. Growth uses checked
geometric capacity or the complete required contiguous extent; no per-buffer logical
byte quota is selected by these operations. Bulk append checks the complete source
window and new length before reservation, then copies bytes without boxing each
octet. The unaligned `core.bytes.read-i64-le` and paired word append use signed
little-endian eight-byte words with exact range checks. Native freeze copies into
independent immutable bytes. Affine custody is established by
complete source and normalized admission. Immutable native I64 values -1 through 255 and both Bool values share read-only
scalar objects; all other integers retain ordinary checked allocation. This
physical lookup table does not restrict supported scalar values or change affine
custody. It does not reclaim general scratch allocations.
Dynamically allocated backing storage, including retired growth allocations and
abandoned buffers, belongs to the active native allocation context. Ordinary
command execution retains its context until process exit. A pure binary filter
also exports the current [LKJPH001 phase interface](native-phases.md): each call
owns separate scratch and output slab chains, promotes the complete result and
reclaims scratch before return. Releasing the caller's output handle unmaps its
remaining slabs. This is invocation-level reclamation, not per-buffer collection
or language-level Region support.
The standard library's pure map and fold run as native calls.
Ordinary entry parameters must be I64; results may be supported scalar values,
lists and records. Binary filters use the separate exact signature above.
External capability requirements, scoped views, implementation witnesses,
F64, other byte entry/results, region operations
and other unsupported instructions produce
`native_elf_unsupported` before output publication. There is no interpreter fallback.

The ordinary Variant/Match backend subset requires the complete reachable
nominal type closure to be copyable and acyclic, including payload types of
unselected cases. Exact prepared origin and declaration/layout identity are
checked; equal-shaped nominal definitions are not interchangeable. Constructors
preserve payload absence versus present Unit. Match requires complete canonical
case coverage, forward in-function arm targets and distinct fresh nonparameter
payload locals. Only the selected arm initializes its binder. Ordinary Record
evaluation preserves its admitted canonical selector order. Existing ByteBuffer
cleanup and live-owner tail fallback apply around every arm.

This extension does not admit OwnedRecord/OwnedChoice, resource or borrowed
payloads, generic/recursive nominal values, witnesses or lexical Regions. Its
internal nominal values do not change the public binary-filter signature: an
equal-shaped nominal Reply cannot replace the structural protocol record. The
Rust native Variant layout and the source-native nominal layout are private to
their respective backends; neither is a public wire or phase ABI. Variant shape
traps use the existing native rejection/phase recovery path, and surviving phase
results remain governed by the unchanged structural Bytes-output contract. The
[ordinary nominal decision proposal](../decisions/20261103-closed-ordinary-nominal.md)
records the publicly adopted candidate, passed C0 and focused source controls,
and pending renewed compiler-generation/frozen-tree acceptance.

The safe first-party emitter writes the ELF64 header, an RX `PT_LOAD` segment,
a non-executable stack declaration, native x86_64 code and relocation/data bytes
into a fallibly grown `Vec<u8>` with checked contiguous representation extents.
The generated executable uses Linux `read` (0), `write` (1), `mmap` (9),
`munmap` (11), `mremap` (25) and `exit` (60), with no Rust VM, system assembler or
external runtime helper. Anonymous mappings contain writable values; their owner
is the command context or a phase handle. The phase API returns distinct
completion, cancellation, allocation, runtime-trap, cleanup and invalid-handle
statuses. It is exported only when the complete selected callable closure is
pure; an empty task effect row does not qualify.
Native execution does not implement the VM's configurable work/allocation quotas.
Every direct and indirect call aligns the machine stack to 16 bytes. A saved
argument-base pointer keeps language stack arguments independent of alignment
padding; nested probes verify alignment and argument preservation at 16 depths.

Callable values use immutable target metadata and a flat captured prefix. An
empty Bind evaluates its callee, checkpoints, and retains that same value without
an environment allocation. Nonempty binding checks complete prefix/layout
extents before allocation, preserves previous captures and appends the newly
evaluated arguments in source order. The complete outgoing argument window is
reserved before ordinary Invoke calls copying or cancellation helpers.

Eligible graph TailCall/TailInvoke reuses RBP and the original caller return
record. Explicit argument slots move with memmove ordering before any immutable
captured prefix is installed. Target nonparameter locals start empty, and the
saved BX/R12/R13 slots plus R14/R15 phase state retain their contracts. A live
owned local takes ordinary Call/Invoke and its retained cleanup/Return
continuation. Consumed/moved locals are cleared before ownership scans; borrowed
parameters are distinguished from owned locals. Registered external targets
retain their ordinary argument ABI and bounded terminal call fallback.

Every graph tail transfer checkpoints even at zero arity with zero locals.
Tail transfer prevents accumulating activations; ordinary non-tail recursion and
very wide native frames remain subject to physical OS stack availability.
Native private metadata is compiler-constructed, not an arbitrary-pointer
sandbox. The [profile-9 contract](selfhosting.md#closed-ordinary-nominal-transfer)
defines the selfhost projection separately from this Rust backend's private
callable representation.

The deterministic algebraic pass is a separate native lowering plan over pure
normalized expression trees. It leaves the admitted program and metered bytecode
unchanged, preserving exact VM quota checkpoints. Proof is bounded by eight
saturation iterations, 1,024 expression nodes and 65,536 input instructions.
Integer folding uses checked operations; identities preserve every possibly
trapping operand. Literal map/fold fusion requires finite successful callback
proofs, and runtime identity maps fuse into folds without changing reducer trap
order. Combinators are recognized only from the exact built-in package revision.

Direct-execution tests compare emitted ELF output with both bytecode and canonical
interpretation, including runtime arguments, closures, recursive loops, structured
JSON, integer limits, trap cases and map/fold pipelines. Separate public CLI tests
exercise detached execution and create-new publication. See [verification](verification.md)
for the complete acceptance gates.
