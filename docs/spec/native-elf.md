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
EOF with a 16 MiB limit, and writes the raw image only after the entry returns
diagnostic zero. Nonzero diagnostics produce `compiler_rejected:N` plus newline
on stderr, empty stdout and exit 65. Read errors or excess input produce
`native_input` and exit 65; argument errors exit 64 and runtime traps/allocation
failure exit 70. Read and write loops retry EINTR and continue partial transfers.
Output write errors exit nonzero (74 for observed syscall errors). The target's
signature selects this boundary at build time; neither input nor output is sniffed.
An external launcher may stage, sync, chmod and atomically publish successful
bytes; the generated program performs no filesystem publication itself.

The initial native instruction set covers Unit, Bool, I64, text constants,
structural and nominal records, lists, fields, branches, ordinary and generic
calls, bound function values, invocation and self tail recursion. It includes
checked I64 arithmetic/comparisons/conversion, boolean operations and list
length/get/append. Immutable byte intermediates support length/get/equality,
construction from octet lists or raw UTF-8 text, copying, slicing and concatenation.
Byte construction and indexing preserve exact octet/index/range traps. Every byte
producer allocates independent bounded storage; JSON string encoding is separate
from the raw UTF-8 view. Unkeyed 32-byte BLAKE3 runs as native integer code, with
bounded chunk-tree recursion and independent block/chunk/tree test vectors.
ByteBuffer empty/push/get/length/freeze/discard preserve admitted exclusive
mutation and scoped read-only borrowing. Growth doubles capacity up to 256 MiB;
freeze copies into independent immutable bytes. Affine custody is established by
complete source and normalized admission. All backing storage belongs to the
process heap, including retired growth allocations and abandoned buffers; it is
reclaimed at process exit, with no per-buffer RSS reclamation claim.
The standard library's pure map and fold run as native calls.
Ordinary entry parameters must be I64; results may be supported scalar values,
lists and records. Binary filters use the separate exact signature above.
External capability requirements, scoped views, implementation witnesses,
F64, other byte entry/results, region operations
and other unsupported instructions produce
`native_elf_unsupported` before output publication. There is no interpreter fallback.

The safe first-party emitter writes the ELF64 header, an RX `PT_LOAD` segment,
a non-executable stack declaration, native x86_64 code and relocation/data bytes
into a bounded `Vec<u8>`. The generated executable uses Linux `read` (0), `write` (1), `mmap`
(9) and `exit` (60), with no loader, Rust VM, system assembler or external crate.
Anonymous mappings contain writable values and remain live until process exit.
Native execution does not implement the VM's configurable work/allocation quotas.
Every direct and indirect call aligns the machine stack to 16 bytes. A saved
argument-base pointer keeps language stack arguments independent of alignment
padding; nested probes verify alignment and argument preservation at 16 depths.

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
