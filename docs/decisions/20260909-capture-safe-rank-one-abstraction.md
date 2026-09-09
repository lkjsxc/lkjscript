# Explicit capture-safe rank-one abstraction

The accepted graph now gives a type parameter an explicit empty or capture-safe constraint set.
This revises the bare-parameter limitation in
[explicit pure function binding](20260909-explicit-pure-function-binding.md). Unconstrained
parameters still cannot supply stored runtime environments. Function signatures remain capture
leaves; neither equality, serialization, task closures nor resource polymorphism is implied.

The closed set is represented by explicit stable tags 0 and 1, with strict JSON sets `[]` and
`["capture-safe"]`. This represents exactly the selected sets without a heap allocation per owner
or inferred side table. The parameter's existing semantic identity and declaration remain its
sole authority. The public setter is an interface edit that validates the complete final candidate.
Ordinary scope, arity, effect and type checks stay independent of entailment. All stored nominal
fields/cases and container element types participate even when no value occupies them.

Static entailment uses the existing bounded stored-type traversal. A TypeParameter requires the
exact in-scope pure declaration's bound; a foreign owner does not import an assumption. The compiler
retains constraints in ordered signatures. Strict artifact loading checks their shape and compares
parameter metadata against exact canonical owners. An envelope checksum certifies neither fact.

Production preparation derives a disposable capture-safe set from compiled layouts and completed
instantiated type closure, with bounded per-root visited traversal. Canonical reference preparation
uses a separate greatest-fixed-point elimination over accepted layouts, starting with ordinary
forms and eliminating aggregates that store unsafe children. Function signatures are leaves in
both. This independent algorithm also handles already admitted nominal cycles. No new recursive
type admission is introduced. Proofs belong only to an exact validated preparation; reconstruction
consults current constraints, nominal fields/cases and package closure. There is no persistent
constraint catalog keyed by a TypeObject digest. Repeated invocation uses the proof and checked
immutable values; raw inputs and actual hidden environments still cross full admission.

The standard `function-constant<Value: capture-safe, Argument>` binds a private ordinary generic
helper whose parameters are unconstrained. The separately authored configure factory binds runtime
Env and step, and can return a private helper through an exported library. This is a new callback
family proving safe abstract runtime environments across generic library boundaries, not evidence
of prior application adoption. lkjournal receives only the generation/dependency cutover.

| Owner | Selected contract |
| --- | --- |
| Graph owner/root/dependency/retirement/change codecs and domains | 12, replacing 11 |
| TypeObject magic, encoding and digest domains | 10, unchanged |
| Compiled unit envelope/signature and compilation-key domain | 7, replacing 6 |
| Package interface owner envelope | 8, replacing 7 |
| Summary record/envelope; interface dimension | 7; other dimension domains remain 6 |
| Validator | 12, replacing 11 |
| Validation-witness representation | 7, unchanged; validator binding changes |
| Compact change / authored intent | 16 / 13 |
| Registry / CLI / definition projection / query | 15 / 29 / 4 / 7 |
| Bytecode / artifact / package container | 4 / 16 / 3, unchanged framing |
| Pure-tail / offline-package acceptance receipts | 5 / 3 |

Existing immutable ObjectKey framing remains stable (compiler-unit domain v1, package-interface-owner
object domain v7). The new versioned envelope bytes participate in those keys. Operational data,
queue, backup, deployment, runtime value and immutable flat prefix representations are unchanged.
Repository/package/declaration/parameter/member/expression identities and retirements survive the
isolated GraphRepository migration; only the deliberately added standard family has new identities.

Fixed public expectations, canonical-reference behavior, corruption fixtures and restored-scan
faults must be able to falsify these decisions. Shared type/owner codecs could conceal a common
encoding bug, so predecessor-produced exact TypeObject/data byte fixtures and explicit constraint-tag
expectations remain separate. A rule accepting unconstrained parameters must fail the negative
oracle; a compiled metadata erasure must fail strict canonical comparison. Failure of these
boundaries requires fixing or revising the derivation, not retaining a second authority, relaxing
raw admission, adding an intrinsic factory or changing unchanged data formats.
