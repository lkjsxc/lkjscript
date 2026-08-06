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
Pattern matching is checked for type correctness and exhaustiveness.

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
validity. Ownership analysis and HIR memory planning do not reject an aggregate HIR expression
count; memory-plan expression work remains checked observational `u64` telemetry. An untrusted
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
SSA frame metadata retains `u64` slots. The active bytecode uses fixed little-endian `u64` branch-target, local, argument-count,
witness-ordinal, and place operands, with separate place and local operands when an instruction
names both. Decoding rejects truncation and host-index overflow before indexing. The generic VM
executes 300-parameter/argument programs with slot 299 and more than 255 simultaneously live lexical
locals; larger generated cases execute 1,024 scalar and 1,024 owned parameters and arguments. The
owned case emits and validates 115,754 bytes in main, executes exact cleanup, and uses the generic
VM when automatic native preflight declines the expansion. A generated function with a jump target
beyond 65,535 bytes executes both branch paths. Native ABI or machine-plan ineligibility is
specialization status: automatic mode uses the VM, and a forced native diagnostic may reject the
signature or shape.

Source positions, spans, and snapshot-local node indexes remain `u32`, creating separate
addressable representation boundaries. HIR memory planning still has table, shape, and
verifier-work quotas. Trusted bytecode validation is unrestricted by encoded-byte, table-entry,
metadata-byte, constant-data-byte, and cleanup-node/range counts. An untrusted artifact caller may
explicitly select `ValidationPolicy::Limited { max_total_bytes }`; this checks only one
checked-arithmetic total-byte observation, reports byte-policy exhaustion separately from malformed
bytecode, and does not alter validity under `Unrestricted`. Bytes literals have no project-selected
size rule: decoding validates hexadecimal syntax and reserves storage fallibly. `u16` constants,
globals, and product/enum/structural tables remain, while product fields and enum substitutions retain byte-sized
representations. HIR and SSA place identities still use `u32`, a separate representation gap above
that range. Validator-synthetic owner identity preserves instruction offsets and parameter indexes
at host width. Other recursive type/trait/enum and structural-value ceilings also remain. These
inherited ceilings are known defects, not permanent language rules. New work must remove the
checks and repair the algorithms or representations rather than publish larger numbers. Real host exhaustion, cancellation, checked representation
overflow, and explicit untrusted-request policy must report typed failures without partial
publication.
