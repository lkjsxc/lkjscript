# Share exact incoming callable inputs within one proof

Decision: during [demanded recursive provenance](demanded-callable-provenance.md),
load one immutable input projection for each demanded incoming call. Reuse that
projection for its distinct requested witness paths. Complete discovery and exact
callable-context cycle selection still precede this phase; neither is cached away.

## Boundary and representation

An incoming call retains its exact source context, target context and optional
expression identity. Contexts retain their declaration, selected method and ordered
implementation selections. An expression reused under a different context receives
a separate entry. Equal-shaped witnesses and separate ordered paths never share
provenance merely because they share source operand bytes.

The first demand reads the original expression or selected implementation mapping,
checks its exact target, method, selection and arities, then retains only the ordinary
type arguments and implementation operands needed for transfer. An unrelated method
inventory or expression's ordinary argument syntax is not retained in this projection;
complete source admission has already checked it. Later demands still traverse their
own exact operand suffix, resolve lexical source scope and construct their own weighted
type-flow edges. Source locations and argument positions remain attached to those edges.

A single-threaded reference-counted handle permits recursive demand insertion while
sharing the immutable projection. These handles are an internal checking mechanism,
not a language ownership policy. The catalogue exists only inside one `connect`
operation over one immutable source reader. It has no global root, persistent entry,
revision lookup or route into a later publication attempt. A changed source must be
admitted again. Successful loading is not a reusable semantic-validity certificate.

Loading is lazy: an acyclic or otherwise undemanded incoming mapping allocates no
projection. Every variable-sized operand and type vector is accounted before the
projection becomes reusable. Traversal uses a charged explicit stack; the existing
work and metadata limits remain unchanged. Every demand, including a cache hit,
retains cancellation and work checkpoints. A failed read, target check, reservation
or cancellation exposes no partial entry. All operation-local handles are released
on completion or failure.

## Independent evidence

The maintained two-package canonical fixture defines a generic function with one,
four, sixteen or sixty-four explicit implementation parameters. Each selected method
maps back to the function, forcing distinct incoming witness paths through one mapping.
The independent source reader counts actual mapping-declaration reads only during
this proof phase, rather than inferring them from elapsed time or reference counts.
The same fixture and test-only phase marker run against the preserved predecessor;
no predecessor production code is changed. Original failure and comparison logs live
under `.artifacts/20261007-call-transfer-inputs/`.

A constructor inserted at each separate witness position must still reject as type
expansion, including the last otherwise equal-shaped operand. Acyclic inputs remain
lazy. Exact/N-1 work and early, intermediate and late cancellation must preserve their
failure classes and allow a healthy new admission. Separate low-level tests retain
one-load identity, distinct call/proof entries, refusal without partial reuse and
last-handle reclamation. Existing independent weighted-cycle, package, reference,
ownership and finalized-public-harness obligations remain unchanged.

[Measurements](../performance.md#operation-local-callable-transfer-inputs) separate
read counts, modeled proof work and cumulative reserved metadata. The single-use
case pays bookkeeping overhead. Reusing input projections does not bound the number
of exact contexts or genuinely demanded paths. Lexical declaration lookups, type-object
reads, complete validation and concrete preparation remain separate costs. No general
polynomial bound, whole-program zero-copy behavior, execution speedup, lower RSS or
API-cost saving follows from this change.

Canonical meaning, source/package encodings, compiler/artifact generations, CLI
observation and shared-runtime observation remain unchanged. This derived proof
representation adds no syntax, inferred witness, grant, execution authority or data
migration. [Status](../status.md) owns accepted source and binary availability.
