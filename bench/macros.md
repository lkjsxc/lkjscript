# Operational declarative macros

Declarative macros expand into ordinary checked source. Matching retains expr, ident
and tt captures, opaque expression forwarding, repetitions and lexical definition
scope. Introduced value bindings and captured values keep separate identities;
Rust-compatible type-parameter capture remains observable.

## Source contracts and limits

Module shadowing, exported macros, $crate, local_inner_macros and include retain
exact definition and source-authority identities. Captured expressions expand only
when used, so a discarded expression cannot resolve or read an include path.
Literal spelling survives expansion. Ambiguous repetition and exact invocation
cycles refuse; no arbitrary expansion-depth limit substitutes for this check.

Source queries and checked edits replay source tokens, include inputs and generated
topology. Template changes require rechecking. Macro-owned dependency edits refuse
unless an authenticated authored reconstruction exists; Default and Debug retain
their stricter generated-origin checks. Macro-expanded files conservatively bypass
frontend delta checkpoints while ordinary checked/unit cache validation remains.

Expansion may reorder or repeat source spans. Optional early borrow cleanup now
uses semantic occurrence order; uncertain/shared block occurrences disable that
optimization. Original diagnostic spans are preserved. This changes frontend
scheduling, not the checked IR or ownership rules. Arbitrary procedural code
execution and unsupported fragment forms remain boundaries.

A failed expansion restores the complete original program before diagnostic
recovery. Each failed occurrence rolls back its generated arenas, modules,
origins and captures, and independent errors continue. The original failure is
retained once; recovery leaves the program unexpanded and is never an admission
path. Single-expression/item expansion is the implemented transcriber contract;
arbitrary statement lists and empty terminal expression expansions still refuse.

## Validation

Native SHA-256:

    77695785a3c15da46bd9a3a794e81a2edf4c0155a891ff0cef2084d7bc3659d8

Seed gzip SHA-256:

    590f0863bf80a78506f080825364cc33b557087b517e4c8752ec96cac2b3bb04

The [bootstrap](macros-data/bootstrap.log) starts from the published TemporaryFiles seed at a013e5727
and reaches byte-identical current-emitter generations without a bridge.
The [focused suite](macros-data/cases-native.log) passes 24 cases /
48 native/interpreter routes. The independent
[Rust witness](macros-data/cases-rust.log) verifies 3
values/output cases and 17 recorded Rust verdicts. Two further exact output cases, declarative_macros and nom_tuple_macros, pass separate actual-root Rust comparisons in the declarative-source suite, preserving $crate. Two explicit unsupported producer/fragment boundaries remain unpaired. A separate module/include fixture also has an independent Rust route and is not added to the 24-case cohort.

The combined producer passes these source-command proofs:

- [debug origins](macros-data/debug-query.log)
- [declarative source](macros-data/declarative-query.log)
- [default reconstruction](macros-data/default-query.log)
- [diagnostic recovery](macros-data/semantic-structure.log)
- [macro origins](macros-data/origin-query.log)

The declarative proof retains exact original nom macro-block hashes, a
twenty-one-element tuple expansion and Mode::Output/PResult contracts. It compares
Rust behavior for scoped/module cases and keeps cold/warm units byte-identical.
The origin suite refuses twenty-five malformed controls, including source-token,
include and generated-topology substitutions; Debug adds ten malformed-origin
controls. Authored Default reconstruction and generated/dependency refusals
remain intact. These contracts do not claim admission of nom's actual module tree.

The [first full-gate failure](macros-data/failed-gate-01.log.gz) is retained. Its
obsolete macro-rule refusal was promoted with the original declaration/probe
body unchanged (SHA-256 76f2cacec6dd0762bc26100f950edd89b89e6ee087257f19d22cc205a8bac81d).
The promoted case passes both engines and independent Rust. The actual recovery
regression was fixed in source: all existing semantic/source contracts and the full diagnostic suite pass.
The earlier repair producer passed 735 semantic contracts and 78 diagnostic
cases / 122 routes, with 34 preexisting pending cases; the integrated gate
reruns those suites with the published TemporaryFiles contracts included: 736 semantic contracts and the same 78 diagnostic cases / 122 routes, with 34 preexisting pending cases.
Recovery controls cover malformed rules, repeated/cyclic calls, generated modules,
includes, discarded captures and invalid generated definitions.

Beyond the earlier macro observation, all 685 newly visible exact identities
were checked against original file hashes and source positions: 667 are primary
barriers; eighteen command-local Sōzu logging calls are recovery diagnostics
only because their exact provider-file lexical error precedes every occurrence.
The original @ lexical boundary remains explicit. The final accounting validates
those parent relations and leaves zero new unclassified identities.

The accounting tool maps absolute macro-origin locations only through the
authenticated baseline source-dir and tracked original aliases. Its
[38 offline controls](macros-data/rerank-path-controls.log) and the inherited-review
helper's [nine path controls](macros-data/prune-path-controls.log) reject forged
roots, traversal, untracked files and evidence tampering. Raw diagnostic streams
and complete original-file hashes are retained.

The published TemporaryFiles runtime was merged before the current seed and
fixed point were rebuilt. Its six-grant Weston/TcpPort proof remains a separate
runtime landing. The independent Rust-hosted compiler built from the same
combined source reproduces the native compiler bytes exactly.

The refreshed-seed four-worker [full gate](macros-data/full-gate.log) passes in 1166.061 seconds with 6515 fresh case requests, zero reuse and 128 IR mutants with zero findings. The separate [real-program regression](macros-data/standalone-real-programs.log) passes eight original Levenshtein tests per engine, eighteen provenance controls and all 7,239 native corpus rows. Producer and seed hashes remain unchanged.

Twelve fresh unchanged-source observations give nom 17->49, DOM 37->39 and Sozu 427->968 classified sites, with 34 prior-unclassified Sozu sites and zero new unclassified diagnostics.
No original large root or nom own test admits. The
[current frontier](../docs/real-programs-current.json), exact source-hashed
reviews and [previous frontier](macros-data/previous-frontier.json) retain
original/projected input identity and complete diagnostic accounting.

The checked IR, independent checker, interpreter and runtime are unchanged.
Exact-head gate and witness CI are recorded in the session report after
publication.
