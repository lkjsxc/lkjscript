# Generic associated type families

A family such as Mode::Output<T> retains its receiver, complete applied trait,
member and ordered family arguments through source definition checks,
normalization and method replay. Implementation and family binders remain
separate. Equal family outputs never prove equal receivers or arguments.
Concrete families lower through the existing checked types and calls.

Unused definitions check rigidly. Each implementation must establish the
declared parameter contract without strengthening it, and its right-hand type
must establish the declared result contract. Missing members, wrong arity,
contradictions, transparent cycles and expanding applications refuse at source
locations. No arbitrary recursion depth limit substitutes for proof.

## Delegated parser inference

The nom-shaped delegated parser exposed implementation parameters appearing only
in prerequisite associated equalities, such as P: Parser<I, Output = O, Error = E>.
Inference first proves that exact prerequisite, then uses its normalized outputs
to bind the right-hand variables. Callable and associated constraints iterate
until no bindings change; all final contracts remain mandatory.

Bound-method headers preserve closure output context for rigid and projected
receivers. The authored delegated-parser case exercises OutputM/PResult,
emitting, discarding and error propagation. It is not an original-module
admission claim.

The first inference candidate failed native self-reproduction: a borrowed slice
target prematurely turned an unknown owned match result into an unsized local.
The repair follows incoming unresolved producers only for borrowed slice/str
coercions, preserving ordinary sized constructor/closure inference. Dedicated
owned-clone/match, wrong-element and cloned-key collection controls remain.

## Questions, edits and boundaries

Family signatures, projections and exact member/binder references survive
queries, renames and checked body edits. Missing-contract edits refuse
atomically; unchanged authenticated admission reuses normally.

Type-parameter families are supported. Family lifetime/const binders, defaults,
type-outlives bounds, written region-bearing arguments or definitions,
nonparameter predicates and deferred input well-formedness remain explicit
boundaries. Uniform generic arguments can carry borrowed values without
inventing a lifetime-specific family witness.

## Validation

Native SHA-256:

    f9dde2462bf4727d7c5796d4d54d592e22acda512846e00b7b96ebbf97a8009b

Seed gzip SHA-256:

    754c709e8ae210bd1a50a50015636ced39e74061d8c756fee07b7038a06ae254

The [bootstrap](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/gat-data/bootstrap.log) bridges from the previous positional
seed and obtains two consecutive byte-identical current-emitter generations.
The [focused proof](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/gat-data/cases-native.log) passes 56 cases / 112 engine
routes. The independent [Rust witness](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/gat-data/cases-rust.log) agrees on
24 values/output cases and 32 verdicts. All family, equality-inference and
method-dispatch [query/edit proofs](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/gat-data/validation.json) pass, as do
41 earlier positional cases / 82 routes and their source-command suite.

The refreshed-seed four-worker [full gate](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/gat-data/full-gate.log) passes in 837.933 seconds, with all 6403 case requests fresh, no reuse and 128 IR mutants with zero findings. The separate [real-program regression](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/gat-data/standalone-real-programs.log) passes eight Levenshtein tests per engine, eighteen provenance controls and all 7,239 native corpus rows. Producer and seed hashes remain unchanged after both checks.

The first full gate stopped after 871.890 seconds on three stale diagnostic
fixtures, while ordinary and optimized 1,273-case conformance runs and both 1,215-case
generated runs passed. The newly supported generic Option closure and owned Vec loop
were independently checked and executed with Rust at opt0/3 and with both
lkjscript engines. Their original function bodies remain byte-identical in
positive conformance cases; the consumer harness retains both the inferred and
annotated Vec versions as executed controls. No compiler or library source
changed for this expectation repair. The [retained failed log](gat-data/failed-gate-01.log.gz)
and [validation](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/gat-data/validation.json) distinguish it from the successful gate.

The twelve fresh original-source observations give nom 17->17, DOM 63->61,
and Sozu 448 classified plus 34 prior-unclassified unchanged, with zero new
unclassified diagnostics. Two DOM parse refusals disappear at document.rs:259
and matcher.rs:219 as the parser now carries family binders. Both definitions
use lifetime families, which remain a semantic boundary masked by later root
refusals. Their disappearance does not establish lifetime-GAT support.

The [current frontier](../docs/real-programs-current.json), exact source-hashed
reviews and [previous frontier](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/gat-data/previous-frontier.json) retain this
distinction. No original large root or nom own test has run under lkjscript.
Exact-head publication gate/witness CI is recorded in the session report after
publication.
