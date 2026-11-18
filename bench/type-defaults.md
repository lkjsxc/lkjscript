# Declaration-scoped type defaults

Named structs, enums and type aliases can declare trailing default type arguments.
An omitted argument is resolved in its declaration's scope and becomes the same
canonical application as the corresponding explicit argument. The existing
checked IR and independent checker remain unchanged.

## Source contracts and limits

Defaults may refer to earlier type parameters. They retain declaration identity
through type interning, alias normalization, inference, coherence and implementation
selection; a caller's same-spelled parameter cannot replace that identity.
Concrete defaults and bounds check even in unused declarations. Parameter-dependent
obligations remain attached to each application, so a default does not silently
prove its own required bound. Self/forward references and nontrailing defaults
refuse; recursive omission cycles refuse while explicit finite applications remain
available.

Authored defaults retain exact signature and rename references. Generated Default
and Debug implementation binders carry no copied default metadata, and strict
origin validation rejects it. Nested declarative captures retain defaults and
$crate/caller namespace distinctions. Generated declarations keep their source
origin and reject unsupported projection edits; an authored template change
invalidates its generated factory's epoch.

Defaults on functions, implementations, traits or associated families, const or
lifetime defaults, combinations with const/lifetime binders or nonparameter where
predicates, dependent defaults needing deferred nested well-formedness, and
bounded or unsized alias defaults remain explicit located boundaries.

## Validation

Native SHA-256:

    ef1f22d49ef4ce473dc49566f38ddbcf24f7000ce19ee86a37cfb1dc69073f92

Seed gzip SHA-256:

    5fc31ce964b2173300b230b28c2d0672a23e964da9723f899c6a572a83b49976

The [bootstrap](type-defaults-data/bootstrap.log) starts from the reproduced combined defaults/replace-call producer, whose source contains the exact public replace-call commit 327fa6c
and reaches byte-identical current-emitter generations without a bridge.
The [focused suite](type-defaults-data/cases-native.log) passes 43 cases /
86 native/interpreter routes. The independent
[Rust witness](type-defaults-data/cases-rust.log) verifies 16
values/output cases and 26 recorded Rust verdicts. These 42 standard cases are supplemented by the exact $crate namespace case at the actual crate root, which passes Rust optimization levels 0 and 3 in the generated-defaults suite. All 43 focused cases therefore have an independently recorded Rust route.

The combined producer passes these source-command proofs:

- [debug default origins](type-defaults-data/debug-query.log)
- [declaration defaults](type-defaults-data/query.log)
- [generated defaults](type-defaults-data/origins-query.log)
- [macro origins](type-defaults-data/macro-query.log)
- [structural rename](type-defaults-data/structural-rename.log)

The combined proof passes declaration queries, exact parameter renames, atomic
capture refusal and generated-source epoch checks. The source-origin suites reject
twelve malformed Default/Debug records and twenty-six malformed macro records,
including substituted valid default TypeIds. The separate actual-root Rust route
is registered in witness/check.sh.

The original fixture failures remain in the external proof: unsupported :ty
captures were corrected to the intended :tt contract, a nonexistent nominal body
epoch was replaced by the generated factory's epoch, and a minimal Default test
provider was supplied its exact canonical helper. None changed compiler behavior.

Canonical None advice also follows the new constructor-annotation diagnostic.
The same primary expression span and resolved library Option::None identity
remain required. Seven additional controls cover the exact suggested edit,
inferred same-spelled local constructors and genuinely unconstrained local
constructors on both engines, with independent Rust classification. The first
wrong local-constructor refusal expectation remains retained without alteration.

The original generic-rename fixture and exact operation now assert equal checked
meaning. Its old blanket refusal was obsolete after declaration-owned nominal
references became available. The transaction changes exactly two source spans,
retains the checked meaning and authority, and preserves value 7 across both
engines in old and current compilers. A separately retained function-owned type
parameter still refuses atomically. The complete structural suite passes 142
cases and its CLI checks; unchanged Rust source and exact-body wrappers agree
at optimization levels 0 and 3. The [failed first full gate](type-defaults-data/first-gate-failure.log) remains
preserved. The [migration receipt](type-defaults-data/rename-migration.json),
[original source](type-defaults-data/rename-before.txt),
[renamed source](type-defaults-data/rename-after.txt) and
[exact transaction](type-defaults-data/rename-transaction.log) retain the evidence.
The [independent preflight review](type-defaults-data/preflight-independent-review.json)
verifies the original and corrected inputs.

The refreshed-seed four-worker [full gate](type-defaults-data/full-gate.log) passes in 1283.412 seconds with 6661 fresh case requests, zero case-request reuse and 128 IR mutants with zero findings. The separate [real-program regression](type-defaults-data/standalone-real-programs.log) passes eight original Levenshtein tests per engine, eighteen provenance controls and all 7,239 native corpus rows. Producer and seed hashes remain unchanged.

Twelve fresh unchanged-source observations give nom 47->45, DOM 39->39 and Sozu 968->968 classified sites, with 34 prior-unclassified Sozu sites and zero new unclassified diagnostics.
No original large root or nom own test admits. The
[current frontier](../docs/real-programs-current.json), exact source-hashed
reviews and [previous frontier](type-defaults-data/previous-frontier.json) retain
original/projected input identity and complete diagnostic accounting.

The checked IR, independent checker, interpreter and runtime are unchanged.
Exact-head gate and witness CI are recorded in the session report after
publication.
