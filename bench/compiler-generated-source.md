# Generated Default projections have explicit origins

The repair separates authored declarations from the compiler's Default expansion.
Generated functions and implementations retain their semantic identities, and
checked instances keep their source mappings. Their struct span is now a
labelled definition anchor: it is never offered as an editable function body.
This is a frontend source-claim repair. It adds no IR trust, AST encoding,
frontend checkpoint format, or native optimization.

The original historical scoped case 3 is **not rerun** here. The retained
original 0/6 campaign, scoped 5/6 campaign and their named-horizon union of
29 strict cases remain unchanged. The new successful replay below is a small
regression fixture, not a thirtieth historical proof.

## Representation and edit contract

The positive declaration relation covers `Program.items`, every
`Module.items`, and every block `Stmt::Item`. Functions belong either to a
listed free declaration or to methods of a listed trait/implementation.
An unlisted implementation must match the exact Default expansion of a listed
struct; unexplained functions and implementations fail `E_IDENTITY_ORIGIN`.
This does not infer universal module authorship: the loader also creates module
declarations.

The Default classifier checks the resolved struct target, definition and field
spans, one derive request, one method with the exact implementation owner, its
signature and block, and the named/tuple/unit constructor shape. It resolves
fresh declaration-free paths for `std::default::Default` and
`std::default::__lkjscript_default` in the recorded scopes, then compares their
canonical declaration identities and names. Re-exports remain valid; no
`prelude` owner is hardcoded. A pre-existing declaration field on a malformed
path cannot bypass this comparison.

Only freshly allocated bound, generic-argument and target types, the exact
generated expressions, and their block are classified as synthetic. In
particular, the helper's `args=[field.ty]` and copied original generic bounds
still refer to authored nodes and remain ordinary editable references.

The public catalog additions are `Item.generated_from: String` and
`Catalog.generated_references: Vec<Reference>`. Generated edges are separate
from editable source-spelling references and carry their actual generated owner
and defining identity. Generated Items are excluded from physical span ownership.
The classifier runs on the fresh resolved AST, including when ordinary frontend
checking resumes from a checkpoint.

Generated query signatures are logical descriptors with origin labels. Text
queries return `text-unavailable`; each generated item metadata row marks its
span as a definition anchor. Generated calls have
`FUNCTION::derived:EPOCH::call:N` identities, with a domain-separated
`lkjscript-derived-default-body-1` epoch over the defining identity and
token-normalized declaration. Call metadata inherits that epoch. These call
Items exist in both full and declaration-only catalogs. The previous erroneous
source-body aliases are not retained.

The common edit validation refuses generated targets and insertion parents with
`E_EDIT_GENERATED_SOURCE`, naming the defining struct. Authored function
operations that would rewrite a generated call refuse
`E_EDIT_GENERATED_REFERENCE`. Replay selection and file correspondence use
authored declarations. Replacing or changing the defining struct regenerates
the expansion and checks the complete candidate. Historical authenticated
snapshots retain their producing compiler's old facts.

## Checked operation chain

Base: `f31949f5e3b09125378365871b4fd15ebe00b44a`.
All compiler commands used one worker, a 4 GiB external address-space ceiling
and a 900-second timeout. The signing key was outside both source and explicit
library grants. Normal output and every failed attempt were retained.

The immutable baseline executable was copied from the gated producer:

```
ab2fa80787261e2fec6d872d989099d3b1dd0984c26ac59bf89c7237d9e97504
```

| Request | Result | Operations | Wall s | Load 1m before → after | Purpose |
|---|---|---:|---:|---|---|
| 1 | Refused, source unchanged | 15 uncredited | 46.52 | 4.25 → 5.96 | Initial closure-context and unavailable Option-method forms |
| 2 | Refused, source unchanged | 15 uncredited | 45.12 | 8.70 → 7.57 | Remaining borrowed Option closure context |
| 3 | Applied | 15 | 42.85 | 5.60 → 5.79 | Origin classifier, catalog/facts, central edit and replay guards |
| 4 | Applied | 6 | 52.90 | 5.14 → 6.69 | Resolve exact generated expression paths through the shared Resolver |
| 5 | Applied | 2 | 46.18 | 7.49 → 7.38 | Exact method owner and call metadata epoch |
| 6 | Applied | 1 | 44.77 | 7.20 → 6.35 | Generated calls in declaration-only catalogs |
| 7 | Applied | 2 | 41.79 | 5.95 → 5.82 | Canonical helper and trait declaration comparisons |

An earlier preparation command exceeded the gateway's 64 KiB argument bound
before execution; its rejection is also recorded. Large request files were
then written in separate bounded chunks before invocation.

The five successful requests contain **26 operations and 11 file facts**.
Every file fact's before/after hashes chains to the parent Git blob and final
source file. The initial new helper's bytes also match the operation's
projection source exactly. Neither refused request changed a file.
[audit.pl](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-generated-source-data/audit.pl) and
[audit.txt](compiler-generated-source-data/audit.txt) verify this chain;
the request JSON, status, stdout, stderr and before/after manifests are retained.

Final source hashes:

| File | SHA-256 |
|---|---|
| `src/semantic.lkj` | `f85b5d25336462e58505a07d4426c94489e9df849306174c3719b8de5a8a91d0` |
| `src/semantic/origins.lkj` | `b1922fef4fb8100c64caf936ffb237653e74174143b44f706ec3351c990df2d7` |
| `src/semantic_facts.lkj` | `013429fb13d9fc43c6304a838356603d7c41107b96e8c68261a0e68b58edafa9` |
| `src/structure_edit.lkj` | `8ff0523e958c4f71f4b0322eabd52ff89ad58c1690dac7c0bf177766a8dcd5e4` |
| `src/structure_replay.lkj` | `aa2af86f2cee2393b3be2ae5b67e3738e0f493162d0f64a9ee0a75c20b23d47f` |

### Retained-line adoption

The denominator is added plus deleted lines in retained `.lkj` files, not
cumulative transaction sizes. Copied specimens keep their original filenames
and bytes and receive no operation credit.

| Scope | Added | Deleted | Total | Operation-backed |
|---|---:|---:|---:|---:|
| Production `src/` and `library/` | 467 | 17 | 484 | 484 |
| Twelve retained original/contract specimen files | 438 | 0 | 438 | 0 |
| All retained `.lkj` files | 905 | 17 | 922 | 484 (52.5%) |

Thus the production subset is 484/484; the inclusive lower bound is 484/922.
[retained-lkj-lines.tsv](compiler-generated-source-data/retained-lkj-lines.tsv)
lists every file, and [adoption.tsv](compiler-generated-source-data/adoption.tsv)
records both scopes. Shell fixtures, documentation and JSON requests are outside
this specifically named `.lkj` denominator.

## Validation and retained failures

The final inline64 candidate is:

```
408e30e5679ca684d4a66e501d16854a41ea6ae6d62a7c07a415b1e7df8c1b3a
target/compiler-generated-source-candidate4
```

Its build passed in **33.02 s**, load **3.33 → 3.46**, peak **1,215,096 KiB**.
This is an isolated candidate, not a seed or fixed-point claim.

The complete focused suite passed in **20.14 s**, load **3.46 → 3.72**, peak
**128,176 KiB**. It covers generated method/implementation/call facts, unavailable
text, epoch changes and inherited call epochs; six direct generated-method
operations; a generated insertion parent and generated call target; a generated
call's authored helper signature; a new authored-struct replay; named field-type
and generic-bound renames; positive derived-field regeneration; and historical
pinned facts after a later edit.

A stored body transaction actually resumed **792 frontend definitions**, reused
**17 checked proofs**, and freshly checked one of 18 instances. This exercises
the declaration-only path rather than merely warming a store and assuming reuse.
The exact parser/resolver/origin sources are also copied into a standalone
contract root with only the reporting-only `BlockedRegion` data declaration
stubbed. It checks authored root/nested/external functions and explicit trait
implementations, shared authored types/bounds, and seven malformed cases:
missing free/impl Items, a contradictory positive Item with Free ownership,
wrong body, wrong target, unrelated helper declaration and unrelated trait
declaration. Real library re-exports and the direct stub definitions both pass.

The new small replay proves equal selected-root canonical checked meaning; it
does not change the historical proof count. The Spare field rename also preserves
checked meaning while its derived constructor is regenerated. A Record field
whose authored receiver lacks exact typed-member evidence remains conservatively
refused; this repair does not invent that missing evidence.

The compiler's own source is separately exercised through a **fresh no-store,
declaration-only, no-op body-edit dry-run** over this complete compiler source,
including `ast::Program`. It passed in **41.50 s**, load **3.72 → 3.31**, peak
**1,269,596 KiB**, with **12,757 fresh checked instances**. Source and checked
meaning remained identical, and authority delta was none. The actual request,
both source manifests and full report are retained as `actual-compiler.*`.

All development attempts remain present:

| Focused attempt | Result / concrete issue | Wall s | Load 1m before → after |
|---|---|---:|---|
| 1 | Baseline fake method text captured; candidate refused unresolved generated Expr::Path | 5.06 | 7.43 → 7.55 |
| 2 | Fixture put --no-store after its key, clearing the option; E_EDIT_KEY | 4.03 | 7.52 → 7.15 |
| 3 | Declaration-only catalog omitted the generated call Item | 7.13 | 6.45 → 6.55 |
| 4 | Authored Record receiver lacked exact typed-field evidence | 13.09 | 5.49 → 6.51 |
| 5 | Same receiver limitation with a unique field spelling | 12.80 | 2.70 → 3.99 |
| 6 | Spare's generic field spelling code collided with an unresolved library local; refused | 13.74 | 1.83 → 2.93 |
| 7 | Isolated AST fixture used unsupported Filter::count and array.to_vec forms | 20.17 | 3.97 → 3.34 |
| 8 | Isolated AST fixture's retain closure needed unavailable inference context | 20.25 | 3.36 → 3.52 |
| 9 | All final focused controls passed | 20.14 | 3.46 → 3.72 |

The positive Spare test uses an explicitly unique field spelling; the actual
Record receiver refusal is retained and asserted in the final suite. No
production reference exemption was added for these fixture limits.

The already-dispatched validation shell then attempted the existing query test
script directly instead of via bash: it exited **126** in **0.00 s**, load
**3.31 → 3.31**, with Permission denied and **zero test cases executed**.
It stopped before the replay suite. Neither suite is rerun here; they remain
part of the following root-owned combined full gate.
`tools/check.sh` now adds the new generated-source suite as a one-worker step.

[measurements.tsv](compiler-generated-source-data/measurements.tsv) retains every
operation, construction and test row, including all four candidate builds,
full 1/5/15-minute loads, CPU and RSS. These are development validations of
changing fixtures/source, not speedup observations. Earlier executable hashes
and all raw output remain separate.

## Review and cleanup state

The semantic-query reviewer independently checked positive Item ownership,
fresh versus shared nodes, expression resolution, and the exact owner/epoch
follow-up. Root and its independent review also checked the final canonical
helper/trait repair. The full gate, integration, seed and CI are root-owned.

Each focused shell removed its generated test tree, key, store and images after
copying textual evidence. No such test directory remains and no compiler job is
running. Pending root's exact-artifact audit, this worktree intentionally retains
the immutable baseline, four candidate executables and the private operation key
outside the source roots (its path locator is in target). The inventory is
[retained-fixtures.txt](compiler-generated-source-data/retained-fixtures.txt).
They are not committed. Their later guarded cleanup will be recorded separately.
