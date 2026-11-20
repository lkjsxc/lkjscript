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

The [bootstrap](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/type-outlives-data/bootstrap.log) starts from the unchanged published type-outlives seed at 739c8cde2
and reaches byte-identical current-emitter generations without a bridge.
The [focused suite](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/type-outlives-data/cases-native.log) passes 31 cases /
62 native/interpreter routes. The independent
[Rust witness](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/type-outlives-data/cases-rust.log) verifies 14
values/output cases and 17 recorded Rust verdicts.

The combined producer passes these source-command proofs:

- [lifetime outlives](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/type-outlives-data/region-query.log)
- [type outlives](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/type-outlives-data/query.log)

The integrated source includes the published TemporaryFiles, Unix and TcpPort
runtime, corrected GAT inference, Debug and operational macros with transactional
diagnostic recovery. Type-outlives and lifetime-outlives queries and checked edits
pass. The earlier private type-outlives proof and its separate 210-case regression
population remain retained and are not attributed to this producer.

The original type-outlives step changes the classified nom frontier from 49 to 47
sites. This harness correction repeats all twelve original-source observations
with the identical compiler and leaves every diagnostic stream unchanged.
Inherited source identities and recovery parents are revalidated against the
exact original files; no new unclassified diagnostics remain.

The complete local Rust witness passes 499 values/output cases and 652 verdicts,
including exact compiler byte identity and all registered runtime, macro and
original Levenshtein proofs. Those source, query and Rust receipts are retained
with their original identities because their compiler and inputs are unchanged.

The original exact-commit CI 44332902680 failed at the macro-origin host suite's
600-second watchdog; its witness passed. That failed run remains failed.
The preceding successful CI host suite took 580.842 seconds. A controlled
same-source local pair takes 288.54 seconds with the old producer and 289.42
seconds with this producer; all 25 malformed-origin contracts execute on both
engines. The corrective harness keeps every assertion, uses a finite 1800-second
watchdog and reports phases/status while removing default fixtures and stripping
keys before explicit retention. Its full gate, bootstrap, final phase proof and
twelve original-source observations are separately fresh.

The refreshed-seed four-worker [full gate](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/type-outlives-data/full-gate.log) passes in 1213.535 seconds with 6575 fresh case requests, zero reuse and 128 IR mutants with zero findings. The separate [real-program regression](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/type-outlives-data/standalone-real-programs.log) passes eight original Levenshtein tests per engine, eighteen provenance controls and all 7,239 native corpus rows. Producer and seed hashes remain unchanged.

Twelve fresh unchanged-source observations give nom 47->47, DOM 39->39 and Sozu 968->968 classified sites, with 34 prior-unclassified Sozu sites and zero new unclassified diagnostics.
No original large root or nom own test admits. The
[current frontier](../docs/real-programs-current.json), exact source-hashed
reviews and [previous frontier](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/type-outlives-data/previous-frontier.json) retain
original/projected input identity and complete diagnostic accounting.

The checked IR, independent checker, interpreter and runtime are unchanged.
Exact-head gate and witness CI are recorded in the session report after
publication.
