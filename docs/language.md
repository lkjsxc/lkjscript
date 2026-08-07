# Current language semantics

This document summarizes meaning implemented by the compiler and tests. The physical text notation
is provisional and may be replaced without compatibility support.

## Programs

A package contains modules and declarations. Imports name exact package-relative module paths and
explicit declarations. Resolution is static and deterministic; duplicate declarations, unresolved
names, invalid imports, and import cycles are errors.

Functions have typed inputs, results, and parameters. Calls are statically resolved, including the
implemented generic and trait-dispatch subsets. Local bindings may be immutable or explicitly
mutable. Control expressions include `if`, `do`, loops, `while`, `break`, `continue`, `return`, and
exhaustive enum matching. `never` represents non-returning control.

## Values and types

Implemented foundational values include `unit`, `bool`, `i64`, `f64`, symbols, text/bytes,
byte vectors and borrows, lists, nominal products, nominal enums, capabilities, and typed host
resources. `Option` and `Result` are ordinary generic enums. Products and enums are nominal rather
than structurally interchangeable.

Integer arithmetic and conversions are checked where specified by their operations. Floating-point
values retain IEEE-754 bit behavior tested by the differential suites. Equality is type-directed.
Pattern matching is checked for type correctness and exhaustiveness. Usefulness specialization
work, pattern depth, and canonical witness bytes are not semantic limits; allocation or host-width
failure is a host failure, while every completed nonexhaustive diagnostic carries its complete
deterministically ordered witness.

## Effects, capabilities, and ownership

Host effects require capabilities supplied by the package and runtime provider. Source code does
not receive ambient filesystem, network, database, terminal, or process authority.

Ordinary source does not expose raw pointers, retain/release, a general `free`, tracing controls,
or named implementation lifetimes. Copy values may be duplicated. Affine resources and unique
storage move or borrow under compiler checks. Supported aggregate representations carry a verified
memory plan and deterministic cleanup behavior.

## Incomplete programs

Typed holes are supported by the current Semantic Source editing service. A snapshot containing a
reachable hole is a valid editing state but is rejected for executable compilation. The future
semantic model will also represent unresolved references, ambiguity, conflicts, and parse-import
errors without requiring a valid full text file.

## Text projection

The current notation uses one atom or open/close marker per physical line, with forms such as
`name/` and `/name`. Only `.lkjscript` is accepted. Exact syntax examples live under
`src/examples/`; parser and compiler tests own accepted and rejected forms.

Formatting, comments, spans, and file locations are not intended to become semantic identity.
A future concise renderer may replace this projection.

## Bounds and failure

Type compatibility, ownership legality, capability authority, exhaustive matching, valid control
flow, and artifact well-formedness are semantic laws. A declaration or expression count is not.

The lexer-token, children-per-form, top-level-form, source-nesting, 16 MiB per-source, 256 MiB
aggregate-source, and 65,536 source-unit ceilings have been removed. Trusted source validation,
loading, package analysis, and compilation are unrestricted by source-byte, source-unit, or source
nesting policy. Parsing and the ordinary deep-expression compiler path grow stack storage on the
heap and report parser reservation failure as host failure; depth does not grant or deny language
validity. Ownership analysis and HIR memory planning do not reject an aggregate HIR expression, function,
use, loan, call, obligation, witness parameter/argument, destination, borrow scope, drop path,
type-fact, type-edge, witness, region-product, structural operation-reference, or type-SCC work
count. Memory-plan producer telemetry uses checked observational `u64` arithmetic, and the
independent verifier reconstructs and requires exact totals without comparing them with admission
maxima. Compiler and SSA type verification do not use depth or work fuel. Auto traits are
solved over memoized canonical obligations with coinductive nominal cycles: a cycle holds unless a
reachable constituent disproves the trait. An untrusted
Semantic Source request may apply an explicit aggregate source-byte policy;
exhausting that policy is a typed host resource failure and does not make the unchanged program
invalid. SSA ownership verification has no project-selected work or retained-state-cell quota;
wide states remain subject to the same exact predecessor joins, affine availability, active
place/owner/drop facts, loop invariance, unreachable ownership rejection, borrow locality, and
cleanup semantics as smaller states.

Canonical verified-SSA, validated-bytecode, and prepared-program identities are incrementally
hashed without canonical-byte, append-count, prepared-descriptor-byte, or ordered-closure-entry
ceilings. Prepared closure ordering, uniqueness, nonempty/nonzero rules, and checked `u64`
canonical length encoding remain validation laws or wire representation boundaries. Executable
identity bytes changed with the active bytecode format, and prepared identity distinguishes an
available native transport-specialization identity from no such specialization; it never substitutes
the semantic SSA identity as a claim of native support.

Function parameter counts, call argument counts, lexical local slots, owner slots, and executable
places are not limited by a byte-sized language rule. HIR and code generation retain host indexes;
SSA frame metadata retains `u64` slots. The active bytecode uses fixed little-endian `u64`
branch-target, constant, global, closure, local, argument-count, witness-ordinal, and place
operands, with separate place and local operands when an instruction names both. Constant/global
IDs, prototype references, and their runtime values and metadata remain `u64` until checked host
indexing. Decoding rejects truncation and host-index overflow before indexing. The generic VM
executes 300-parameter/argument programs with slot 299 and more than 255 simultaneously live lexical
locals; larger generated cases execute 1,024 scalar and 1,024 owned parameters and arguments. The
owned case emits and validates 115,754 bytes in main, executes exact cleanup, and uses the generic
VM when automatic native preflight declines the expansion. A generated function with a jump target
beyond 65,535 bytes executes both branch paths. Native ABI or machine-plan ineligibility is
specialization status: automatic mode uses the VM, and a forced native diagnostic may reject the
signature or shape.

Source positions, spans, binding/place/loan IDs, and snapshot-local node indexes remain `u32`,
creating separate addressable representation boundaries. HIR function, expression, entry, use,
constant, call, obligation, and type-fact IDs and SSA function, block, value, binding, trait,
implementation, place, and loan IDs likewise retain `u32` representations. HIR destination,
borrow-scope, drop-path, and drop-glue IDs, memory-witness parameters/bindings/group ordinals, local
witness dependency targets, and executable structural destination IDs use `u64` with checked host
indexing. The remaining compact native witness and aggregate fields are specialization eligibility:
a declined native shape retains valid VM execution. The seal dependency/release thresholds only
choose the total generic placement fallback and cannot reject validity.

Trusted bytecode validation is unrestricted by encoded-byte, table-entry, metadata-byte,
constant-data-byte, cleanup-node/range, region-product, witness, structural-destination, and
structural-operation-reference counts. An untrusted artifact caller may explicitly select
`ValidationPolicy::Limited { max_total_bytes }`; this checks only one checked-arithmetic total-byte
observation, reports byte-policy exhaustion separately from malformed bytecode, and does not alter
validity under `Unrestricted`. Runtime `ExecutionPolicy` is explicit and non-defaultable: trusted
local execution selects `Unrestricted`, while untrusted process work selects a coarse `Limited`
policy. List
value equality is complete and iterative for the implemented acyclic immutable list
representation; it has no independent step quota. Byte vectors, static-byte clones, and bytes
literals have no project-selected per-buffer size rule: decoding and runtime storage use checked,
fallible reservation and explicit heap/allocation policy. Bulk file/socket views have no 64 KiB
operation-validity rule. Private chunks are implementation tuning, partial-count operations report
the exact partial count, full-transfer operations iterate, and output/deadline/cancellation policy
is enforced between chunks. Validator-synthetic owner identity preserves instruction offsets and parameter
indexes at host width. Some pre-existing core structural type/layout/representation lookup helpers
still turn host-width overflow into a failed sentinel lookup, and recursive semantic operation,
runtime structural-value, and specialization paths retain incomplete stack-safety coverage. These
representation and recursion gaps are known defects, not permanent language rules. New work must
repair algorithms or representations rather than publish larger numbers. Real host exhaustion,
cancellation, checked representation overflow, and explicit untrusted-request policy must report
typed failures without partial publication.
