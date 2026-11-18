# Positional trait type arguments

Named trait parameters now form part of the complete proof identity:
Parser<u8> and Parser<bool> remain different applications for the same receiver.
Bounds, implementations, supertraits, associated equalities, qualified members
and opaque results retain that distinction. Blanket implementation parameters
may be constrained through the applied trait as well as its receiver.

Generic definitions check rigidly even when unused. Declared argument bounds
and implicit or relaxed Sized obligations apply before specialization.
Provided bodies substitute trait and implementation parameters independently;
exact source recipes retain the selected method and applied trait.

## Method inference and ownership

The actual nom tuple-macro body exposed a missing transfer between definition
inference and concrete method selection: input types solved from the call were
discarded before checking implementation prerequisites. The correction saves
that solution under the exact expression, implementation and method identities,
then proves candidate bounds again. Ambiguous applications still refuse.

The solved application does not create an authored coercion site. Arguments
first use receiver-derived context; unconstrained direct or nested mutable
references still move. Explicit reborrows remain available.
Eleven direct controls cover nom-shaped tuple aliases/equalities, generic
replay, result-context inference, ambiguity, wrong input, unused-definition
contracts and both mutable-reference ownership routes. These authored controls
do not establish admission of nom's original module tree.

## Source commands and boundaries

Queries retain applied signatures, projections and exact member/method
references. Renames update selected identities and callers; wrong-input and
wrong-body edits refuse atomically without publishing source or admission.

Trait lifetime/const parameters, GATs and explicit inference placeholders
remain separate boundaries. Written reference arguments, lifetime-parameterized
nominal/alias arguments and projections normalizing to views require a
region-aware witness identity and are refused. Uniform type parameters can
carry borrowed values without inventing lifetime-specific witness equality.
Proof cycles and expanding applications refuse without an arbitrary depth cap.
The checked IR, independent checker, runtime and library are unchanged.

## Validation

Native SHA-256:

    3d2fc84dfc880cdd52aee6ca9a16dbef6025500e8453cd7ab75f45f336fda353

Seed gzip SHA-256:

    c1563abbe755c994dc5e9baf2170b384db7b1390ff21dbcc7fe32cbfe27e446a

The [bootstrap](positional-traits-data/bootstrap.log) reproduces itself exactly.
The [focused proof](positional-traits-data/cases-native.log) passes 41 cases /
82 engine routes; the separate [Rust witness](positional-traits-data/cases-rust.log)
agrees on 19 values/output cases and 22 verdicts. Both
[applied-trait](positional-traits-data/query.log) and
[method-dispatch](positional-traits-data/dispatch-query.log) query/edit suites pass.

The refreshed-seed four-worker [full gate](positional-traits-data/full-gate.log) passes in 772.339 seconds. All 6,288 case requests rerun, with no reuse; 128 IR mutants have zero findings. The separate [real-program regression](positional-traits-data/standalone-real-programs.log) passes eight Levenshtein tests per engine, eighteen provenance controls and all 7,239 native corpus rows. The producer and seed remain byte-identical after both checks. Exact-head publication gate/witness CI is recorded
with the session report after publication.

All twelve fresh original-source observations are byte-identical: nom 17,
DOM 63, and Sōzu 448 classified plus 34 prior-unclassified sites. Exact
source-hashed reviews carry forward; no diagnostic is classified by keyword or
disappearance. The [current frontier](../docs/real-programs-current.json)
records producer and library inventory. The [preceding frontier](positional-traits-data/previous-frontier.json)
preserves the full lifetime-where proof and earlier failure/repair history.

The initial isolated fixture copy flattened matching pass/fail basenames and
was incomplete; the final 41-case directory preserves both. Its first rerank
also refused the isolated library command path. Final observations use the
original paths, with no weakened comparison guard. Earlier raw attempts remain
with the session receipts. No large root or nom own test is claimed admitted.
