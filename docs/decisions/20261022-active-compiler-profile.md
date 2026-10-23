# Active compiler source profile

Date: 2026-10-22 (UTC). This inventory describes the authoritative maintained
compiler after the mode-1 byte-capacity transaction. It is an observation of
accepted source, not source-driven E2 admission, E3 lowering or a custody proof.

## Reproduction and identity

The nonignored test
selfhost_source_profile_inventories_every_owner_and_type reconstructs the complete
authoritative snapshot and visits every supplied owner and type. It does not
select a backend entrypoint or scan proposal text to determine membership.
With an absent output path, reproduce the complete JSON report using:

~~~sh
LKJSCRIPT_SELFHOST_PROFILE_OUTPUT=/tmp/lkjscript-active-profile.json \
  cargo test --release --locked --lib \
  selfhost_source_profile_inventories_every_owner_and_type -- --nocapture
~~~

Run under the repository's shared Cargo-target lease. The output path uses
create_new; omit the variable during ordinary verification. The test also prints
the summary and report digest without writing a file. The canonical project
remains the authority; the report is derived evidence.

| Identity | Value |
| --- | --- |
| Revision | rev_dc8bb4c50291e16e49a520baf65b2c93dc039d1305fc0f9c0e2a961767b03c70 |
| Semantic state | semantic_state_db9cdb516c961897effbf694efb3530f047cab7de3faa46dffae067760576b0a |
| Complete LKJSG001 bytes | 6,985,830 |
| Complete carrier SHA-256 | 6a29a86a8d10164f5d302c6abea284e071bbe1c426e0e3c7df088eecfb5948b0 |
| Report bytes | 10,771,886 |
| Report SHA-256 | b685d5f90ffa7a886df5ffcb1ca55a09c59a051d2a041914c5062f11a9154f7d |

The report contains the exact module/declaration records, ordered parameters and
use modes, type descriptors, bindings, ports, targets, intrinsic declaration
templates and ordered substitutions. All identities remain canonical. Complete
carrier identity is required because unused supplied types need not change the
semantic-state digest.

## Complete supplied inventory

| Owner family | Count |
| --- | ---: |
| Modules | 62 |
| Pure functions | 442 |
| External intrinsic declarations | 23 |
| Component / port / target | 1 / 1 / 1 |
| Function parameters | 1,522 |
| Type parameters | 2 |
| Let bindings | 2,040 |
| Expressions | 24,490 |
| All live owners | 28,584 |
| Retirement records | 10,586 |
| Supplied structural type objects | 27 |

There are no dependency packages, derived dependency interfaces/types,
representation dependencies or external blobs. All 466 declarations are included:
442 pure functions, 23 externals and one component. The component has no
requirements. Pure functions have no implementation, requirement, effect or
read-scope binders, no read-scope sources, and ordinary value results. Calls have
no requirement, effect or read-scope applications. The inventory does not treat
an empty task row as purity; no task callable occurs in this maintained profile.

The 62 included modules are:

~~~text
aggregate artifact artifact-check artifact-input artifact-manifest artifact-order
asm ci custody elf flow hash heap machine objects pack profile runtime
source-capability source-capability-shape source-containment source-edges
source-effect source-effect-shape source-expr source-fields source-forest
source-function source-graph source-history source-index source-link-expr
source-link-owner source-link-ref source-links source-map source-members
source-namespace source-namespace-key source-namespace-sort source-nominal
source-nominal-cycle source-nominal-flow source-nominal-queue source-object
source-owner source-read source-scope source-shape source-sort source-state
source-table source-type-dag source-type-scope source-type-table source-type-uses
stream types unit-map unit-wire values wire
~~~

### Expressions and types

| Expression constructor | Count |
| --- | ---: |
| Local | 8,807 |
| Call | 5,996 |
| I64 | 5,934 |
| If | 1,711 |
| Bool | 774 |
| Let | 713 |
| Field | 246 |
| Text | 123 |
| Record | 97 |
| List | 89 |

Local values name exact function parameters or ordinary lexical bindings.
All records are structural and all field selectors are structural. There are
no nominal constructors, Match, closure construction/invoke, Region operations,
task applications or explicit source return constructors in this closure.
Function return values arise from expression results. Intrinsic failure and
evaluation order still matter even without an explicit return expression.

The supplied type forms are one each of Bool, I64, Bytes, ByteBuffer and Text;
three List descriptors; sixteen structural records; one pure Function descriptor;
and two distinct TypeParameter descriptors. The three List descriptors are the
two generic declaration templates and List<I64>. No nominal or owned type,
TaskFunction, read view, associated type or Region type occurs. Unit and StaticText
are accepted by the inventory's expansion guard but are absent from this report.

The sixteen record field sets, with exact canonical field order, are:

~~~text
{order:I64,work:I64}
{declaration:I64,work:I64}
{rows:Bytes,work:I64}
{digest:Bytes,image:Bytes,key:Bytes}
{at:I64,count:I64}
{body:I64,effect:I64,effects:I64,parameters:I64,requirements:I64,result:I64,types:I64}
{code:I64,next:I64,value:I64}
{found:Bool,work:I64}
{diagnostic:I64,image:Bytes}
{code:I64,rows:Bytes}
{height:I64,work:I64}
{digest:Bytes,image:Bytes}
{content:Bytes,digest:Bytes,image:Bytes}
{edges:Bytes,work:I64}
{code:I64,high:I64,low:I64,next:I64}
{code:I64,length:I64,next:I64,start:I64}
~~~

### Intrinsics and closed substitutions

All 23 external declarations belong to ci. Unmarked parameters below are
unrestricted; borrow and consume remain separate parameter-use facts.

| Declaration | Instantiated signature | Implementation |
| --- | --- | --- |
| add | (I64, I64) -> I64 | core.i64.add |
| sub | (I64, I64) -> I64 | core.i64.subtract |
| mul | (I64, I64) -> I64 | core.i64.multiply |
| div | (I64, I64) -> I64 | core.i64.divide |
| eq | (I64, I64) -> Bool | core.i64.equal |
| lt | (I64, I64) -> Bool | core.i64.less |
| both | (Bool, Bool) -> Bool | core.bool.and |
| either | (Bool, Bool) -> Bool | core.bool.or |
| length | (Bytes) -> I64 | core.bytes.length |
| get | (Bytes, I64) -> I64 | core.bytes.get |
| concat | (Bytes, Bytes) -> Bytes | core.bytes.concat |
| same | (Bytes, Bytes) -> Bool | core.bytes.equal |
| slice | (Bytes, I64, I64) -> Bytes | core.bytes.slice |
| text-bytes | (Text) -> Bytes | core.bytes.from-text |
| octets | (List<I64>) -> Bytes | core.bytes.from-list |
| digest | (Bytes) -> Bytes | core.bytes.blake3 |
| derive | (Text, Bytes) -> Bytes | core.bytes.blake3-derive-key |
| empty | () -> ByteBuffer | core.buffer.empty |
| push | (I64, consume ByteBuffer) -> ByteBuffer | core.buffer.push |
| freeze | (consume ByteBuffer) -> Bytes | core.buffer.freeze |
| size | (borrow ByteBuffer) -> I64 | core.buffer.length |
| list-get<I64> | (List<I64>, I64) -> I64 | core.list.get |
| list-size<I64> | (List<I64>) -> I64 | core.list.length |

There are exactly twelve generic application sites: nine list-get and three
list-size calls. Each substitutes I64 for that declaration's own type parameter;
their binder identities are distinct. There are no generic pure-function
applications in this closure. The report retains every call-site identity,
the complete declaration template and its ordered substitution, so the
instantiated signature is reproducible without flattening binder identity.

Of all 1,522 parameters, 1,386 are unrestricted, 135 consume and one borrows.
The single borrowed parameter belongs to ci::size. The consume parameters occur
in intrinsic and ordinary buffer-threading functions. These counts do not infer
local liveness, borrow provenance or permission to retain a borrowed result.

### Lexical exits, recursion and cleanup obligations

The contextual syntax traversal starts at all 442 pure-function bodies and
includes every Let initializer and every branch. It proves inventory coverage
for all 24,490 expression owners, all 5,996 Call owners and all 2,040 bindings.
There are 713 lexical Let scopes and at most 61 simultaneous lexical bindings
in this source traversal; this is not a normalized local-slot count.

The declared direct-call graph includes all 465 callable vertices, including
isolated vertices, and 2,989 distinct directed declaration edges. An iterative
strongly connected component pass finds 463 components, of which 135 are recursive
and contain 137 declarations. There are 133 single-function recursive components
and two mutual pairs: unit-map::content/children and source-map::content/children.

There are 585 syntactically tail-position call contexts. The report retains their
enclosing function, lexical scopes and bindings. It deliberately does not infer
that those calls are eligible for normalized TailCall. Surviving owned buffers
may require an ordinary Call, reverse-lexical cleanup and Return.

A source-driven E3 implementation must still derive each binding's instantiated
type and custody, balance asymmetric branch consumption with explicit disposal,
clean every surviving unreturned owned local at scope/function exit, and keep
borrowed call operands live until the call finishes. Returned owned values move
out rather than being discarded. Trap/evaluation order and cleanup order need
independent controls. The report supplies complete syntax and exact signatures
for that work; it supplies no precomputed custody certificate.

## Boundary to source-driven and full Tier 2 acceptance

This active profile determines the maintained compiler's eventual source-driven
vertical closure. A smaller scalar E2/E3 path can first check complete supplied
carriers and lower Unit/Bool/I64 functions, Let, If and direct calls, including
uncalled and untaken syntax. It must reject the remaining profile explicitly.
All ports/targets and unused types must be checked; root selection cannot filter
semantic admission. Empty-row TaskFunction remains task kind and rejects where
tasks are unsupported.

The maintained compiler additionally needs Bytes/Text, structural records, closed
List substitutions, intrinsic resolution and complete ByteBuffer custody lowering.
Those are executable dependencies of its source-driven fixed points, not reasons
to postpone a smaller sound semantic path.

Full P_self Tier 2 further requires nominal owned records/choices and Match;
closures and invocation; OwnedContract implementations and witnesses; associated
types, views and read-effect isolation; lexical Regions; whole-profile behavioral
programs; source-carrier-bound ELF and canonical artifact fixed points; and
measured same-process native reclamation. E1 shape recognition for a broader
family does not supply those E2/E3 semantics. The
[self-hosting specification](../spec/selfhosting.md) owns the final contract.
