# Named type-outlives contracts

A named type-outlives predicate constrains the whole source type, including rigid
dependencies that do not store a reference in their concrete representation.
Function definitions and calls prove the predicate using the existing directed
region relationships; the checked IR remains unchanged.

## Source contracts and limits

Function-owned type parameters accept named lifetime bounds in inline and where
syntax. Proof follows owned containers, closure captures, opaque forwarding and
fixed callable regions. Call-site obligations retain the type's definition
identity rather than replacing it with a convenient concrete layout.

Queries retain exact bound references and direction. Checked renames and body
changes preserve those edges, while removing a required edge refuses atomically.
Generated macro items keep their authored/generated guard before bound references
are collected.

Trait, implementation-header and associated-family ownership of these predicates,
static/anonymous bounds, unsupported invariant nominal/function-pointer regions,
and the located ordinary-result/parser-inference combinations remain boundaries.
They are source-proof limits, not claims that new trusted semantics are needed.

## Validation

Native SHA-256:

    a2d0d8581c73ceb937addb5ea986ba708fa06c2505161d8745353cf123a9bc8f

Seed gzip SHA-256:

    9327f64a3c9a5c37146ca6c91a6926a8707d9fef3d9fd67398125909f4b5f612

The [bootstrap](type-outlives-data/bootstrap.log) starts from the published operational-macro seed at 5fe0ba9e4
and reaches byte-identical current-emitter generations without a bridge.
The [focused suite](type-outlives-data/cases-native.log) passes 31 cases /
62 native/interpreter routes. The independent
[Rust witness](type-outlives-data/cases-rust.log) verifies 14
values/output cases and 17 recorded Rust verdicts.

The combined producer passes these source-command proofs:

- [lifetime outlives](type-outlives-data/region-query.log)
- [type outlives](type-outlives-data/query.log)

The combined preflight includes the published TemporaryFiles/Unix/TcpPort runtime,
corrected GAT inference, Debug and operational macros with transactional diagnostic
recovery. Both type-outlives and existing lifetime-outlives query/edit suites pass.
The earlier private type-outlives proof and its 210-case regression population are
retained separately; they are not attributed to this producer.

All twelve original-source observations were repeated with this producer. Every
inherited source identity and recovery parent was revalidated against the exact
original files; no new unclassified diagnostics remain.

The complete local Rust witness passes 499 values/output cases and 652 verdicts,
including exact compiler byte identity and all registered runtime, macro and
original Levenshtein proofs. Both reviewed CI harness corrections are retained.

The refreshed-seed four-worker full gate and separate real-program regression are pending; the focused proofs and fresh source census are complete.

Twelve fresh unchanged-source observations give nom 49->47, DOM 39->39 and Sozu 968->968 classified sites, with 34 prior-unclassified Sozu sites and zero new unclassified diagnostics.
No original large root or nom own test admits. The
[current frontier](../docs/real-programs-current.json), exact source-hashed
reviews and [previous frontier](type-outlives-data/previous-frontier.json) retain
original/projected input identity and complete diagnostic accounting.

The checked IR, independent checker, interpreter and runtime are unchanged.
Exact-head gate and witness CI are recorded in the session report after
publication.
