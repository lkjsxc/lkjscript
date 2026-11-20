# Namespace-aware catalog identities

The catalog admits legal type/value declaration pairs, including the compiler's module and function both named `syntax::macros::expand`. A checked transaction selects the function as `value@syntax::macros::expand`; the module remains `type@syntax::macros::expand`. The candidate is `7e08803ba`, based on public `7dd89b8fb`. Its namespace, crate-root, selector and incomplete-reference repairs are separate commits. Lead published the integrated candidate as `589e11b5a`.

## Contract and consumer audit

The [selection contract](../docs/structure.md#semantic-identities-and-selection) defines the full behavior:

- Resolver type declarations use `type@PATH`; free functions and constants use `value@PATH`. Descendants retain their owner's complete identity. Constructors reserve their value slot while keeping one authored type identity.
- External-library declarations use `std::…`; an application module named `std` uses `crate::std::…`. Conversion accepts proven declaration/scope origins and preserves provenance inside associated-method, implementation and IR stems.
- Bare selectors consider literal exact, `type@`, `value@` and `builtin@` candidates with equal precedence. Every metadata candidate participates before applicability is checked. Explicit keys are exact; operation kind and `expect` never disambiguate.
- Displayed types retain source spelling. Refactors and joins retain separate canonical type evidence. Builtin families/concretes are queryable as `builtin@Vec` and `builtin@Vec<i64>` without supplying editable source items.

The consumer audit followed the complete identity flow:

| Consumer | Join or selection checked |
|---|---|
| Catalog and transaction selection | Resolver namespace ownership, crate provenance and the complete metadata set; primary and secondary selectors share the rule. |
| References and generated origins | Full owner keys, already-qualified defining declarations, and both dependencies of shared imports. |
| Facts and authenticated queries | Shared source/builtin/checked-type classification; exact equality between published metadata keys and catalog candidates. |
| Type-sensitive refactors | Canonical capture and alias evidence, canonical trait cohorts, separate display values. |
| Projection and replay | Exact semantic parents and full item/module identities; physical path decoding only at the projection boundary. |
| Checked comparison | Literal selection over the before/after metadata union before detail applicability. |
| Units and checked IR | Proven function/method ownership, nominal crate provenance, one-time normalization, preserved unmatched intrinsic names. |

The independent checker, authenticated store format, semantic transaction postconditions and atomic writer are unchanged. Historical pins retain their original stored facts; canonical cursor selection remains authenticated.

## Checked compiler edit

The three-line baseline example checks and returns 42 on both engines, but editing refuses `E_IDENTITY duplicate source identity "shared"`. The baseline real compiler-root request similarly refuses at `syntax::macros::expand` in 32.18 seconds, without publication changes.

The successful operation is:

```json
{"version":1,"operations":[{"op":"replace-body","id":"value@syntax::macros::expand","source":"{ let result = expand::run(program); result }"}]}
```

It replaces `{ expand::run(program) }`. The final producer repeats dry-run, apply, build and reproduction on a fresh baseline archive. Dry-run preserves source and publication state. Apply freshly checks all 16,640 instances, reports no authority delta, and changes only `src/syntax/macros.lkj` among 1,949 archived source files. An independently constructed expected file preserves its module declaration and every byte outside the function body; all 294 compiler/library source files match the expected archive.

The edited compiler reproduces byte for byte and returns 42 through both engines on the retained constant-function-call specimen. Its inline-64 executable is identical to the original baseline executable, consistent with eliminating the added local binding. The unoptimized checked-meaning digest changes as expected; this is a checked body replacement, not a claim that its conservative IR digest stays equal.

All implementation changes were authored through guarded edits and receive **zero operation-produced credit**. Only the disposable body replacements were produced by checked operations.

The [reproduction harness](compiler-catalog-data/exercise-compiler-edit.sh) creates its own key, archive, request and independent expected file:

```sh
bash bench/compiler-catalog-data/exercise-compiler-edit.sh target/lkjscript
```

The harness completed dry-run, apply, rebuild, reproduction and both execution routes on the final producer; every stage exited successfully.

## Verification

The [namespace](../tools/catalog-namespace-test.sh) and [crate-root](../tools/catalog-roots-test.sh) suites pass on `7e08803ba`. They cover module/function, generic struct/function and alias/constant collisions; true duplicates and constructor reservations; wrong, malformed and stale selectors; unchanged source/generation after refusal; rename/move, shared imports, same-name enum variants, replay, projection, historical snapshots and cursors.

Root controls cover both crates, identical children, insertion and cross-root anchors, associated stems, literal `$std` refusal and builtin family/concrete selection. Bare `Vec` refuses consistently in query, comparison and edit `id`/`before`/`parent`, for dry-run and apply, without pending intent. Display/alias controls preserve canonical references and layouts; the alias-backed enum refactor executes as 42, then 7 when its new variant is selected, through both engines.

The 143-case refactor planner suite plus CLI controls, projection suite, strengthened Debug suite and bootstrap fixed point pass. The existing query/type/generated-source consumers also passed during integration.

The full four-worker gate passes in **1,361.762 seconds**, rerunning **6,840 case requests with zero test-request reuse** and all host suites. IR mutation completes 128 mutants with zero findings. Ordinary and optimized conformance each retain their two existing pending route entries. The gate's rebuilt compiler is byte-identical to the frozen producer used above. Raw results are retained in `target/catalog-data/report-evidence/full-gate.log`, `full-gate.time` and `ir-mutation.log`.

## Focused paired latency

Baseline `7dd89b8fb` and repaired `7e08803ba` index identical source and checked IR from retained revision `62fa76f4f863a169b906094f0dd9998c8421e77a`. The unchanged [50-question cohort](compiler-query-current-questions.tsv) uses its original bare selectors, depths, 96-fact limits and 32,768-byte limits. One untimed warmup precedes three alternating matched rounds under the shared gate lock after this stream's other compiler/test workers stop.

The unchanged [Number::doubled request](compiler-call-data/call.request.json) supplies the change control. Each sample starts from the same source; exact resulting text and both-engine result 43 are checked outside timing. The [archived measurement harness](https://github.com/lkjsxc/lkjscript/blob/evidence-archive-20261120/bench/compiler-catalog-data/measure-latency.pl) rejects mismatched source/IR and violated query bounds.

| Observation | Baseline | Repaired |
|---|---:|---:|
| Query samples | 150 | 150 |
| Query median | 3.445 ms | 5.952 ms |
| Query p95 | 44.650 ms | 51.511 ms |
| Total query wall time | 1.248 s | 1.645 s |
| Change dry-run median, 3 samples | 9.954 ms | 7.693 ms |
| Change apply median, 3 samples | 24.280 ms | 22.128 ms |
| Separate index setup | 134.85 s | 144.68 s |

All 300 queries and 12 timed changes pass. Candidate maximum is 76.223 ms. Returned facts are 2,532→2,535, bytes 746,115→757,215, and logical page reads 4,956→6,936. Only question 36, `slice source::load_report`, changes fact count, 95→96 within unchanged bounds.

Equal-precedence selection adds authenticated lookups. Query median rises 2.507 ms and total query time 31.8% in this trial. The increase is broad: 141 of 150 matched requests are slower, and each reads 7–17 extra pages. This fits common selection overhead; profiling the four metadata-candidate probes is the focused follow-up, without relaxing selection. Three change samples do not establish equivalence. Host background services were uncontrolled, and index setup overlapped validation, so these results provide no general performance bound or controlled setup-speed comparison.

## Decisions and remaining frontier

Claude chose stable namespace-prefixed keys and unique literal bare selection. Neon's initial alternatives supported namespace qualification and highlighted parent traversal, constructors, generated origins and type/IR joins. The final Neon attempt stopped during prompt preparation; the owner replaced it with one Codex Sol review.

Sol found and reproduced crate-root collapse, missing builtin-family selectors and normalized bare keys, then two closure defects: edit/query candidate disagreement and canonical spelling in display facts. The repairs preserve the complete metadata set and separate canonical evidence from display. Sol independently reran those controls, including a checked-IR primitive collision, and cleared those findings on `04087f19c`. The full gate then found a remaining text-based ancestry check that lost the implicit-capture rename refusal. `7e08803ba` uses canonical parent relationships for known targets. Its Debug controls verify partial metadata, unchanged bytes/generation on bare and explicit refusal, and independent renaming of a same-named module. Sol independently verified the repair, sibling modules and mixed crate roots and cleared `7e08803ba`.

The demonstrated compiler-root blocker is removed. Full catalog and whole-program input retention remain open. Order-sensitive meaning comparison can conservatively refuse relocation; repeated authored use aliases retain their located refusal. Authored `mod crate` and coincident builtin/source associated stems refuse rather than inventing ambiguous identities. An attempted `Vec` source alias also exposed an earlier elaboration limitation outside this patch.

Raw logs, source manifests and superseded attempts remain in `target/catalog-data`; compact extracts and individual final timing observations are in `target/catalog-data/report-evidence`. No seed refresh, ten-million-line rerun, new application feature or deployment belongs to this slice.
