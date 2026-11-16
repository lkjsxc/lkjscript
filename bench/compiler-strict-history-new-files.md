# Static classification of the eleven new-file blockers

This inventory reads Git and source declarations only. **No compiler was
run for these eleven revisions.** It adds no successful replay and changes
none of the [four-probe observations](compiler-strict-history-four.md).

There are **37 added projected paths and zero deleted projected paths**.
Thirty destination directories exist in the parent revision; seven do not.
The proposed first step—checked external-module creation inside existing
directories—addresses useful parts of this population, but **does not by
itself establish a complete language-scope replay for any of the eleven
cases** under the frozen selected-root policy. Every case still has a
missing-directory requirement, an off-root change, or another projection
constraint.

## What the labels mean

[Physical paths](compiler-strict-history-new-files.paths.tsv) are computed
from exact parent and target Git trees. A legacy `.rs` path is projected to
`.lkj` without changing its bytes. Directory existence is checked against
the parent tree; the fresh historical materialization does not invent
untracked empty directories.

[Route annotations](compiler-strict-history-new-files.routes.tsv) are
manual source-level classifications supported by the retained declaration
and reference excerpts. A **conventional module route** means the target
text contains the ordinary `mod name;` chain from the selected root; it
does not mean the historical program was admitted by the current compiler.
A **test exclusion** has `#[cfg(test)]` on the module or an ancestor.
A **separate root** belongs to a different compilation entry.

The current loader resolves ordinary external modules as `name.lkj` or
`name/mod.lkj`; `cfg(test)` is false for these compiler invocations.
Historical `#[path = "...rs"]` mounts are kept distinct from those
conventional routes. A positive `include_bytes!` or `include_str!` read
would likewise not prove a module mount. The one `include!` fragment in
this inventory is recorded as a source include, not an external module.

The [joined table](compiler-strict-history-new-files.classification.tsv)
checks that every annotation belongs to the exact added-path set and
records whether its declaring source existed in the parent. Twenty-nine
declarations are in existing files; seven are in other newly added files;
one added file is an independent root.

## Per-revision classification

“Existing” counts destination parents, not declarations. Every row has zero
deleted paths.

| Original ordinal | Revision | Added | Existing / missing parent | Selected-root route and remaining constraint |
| --- | --- | ---: | --- | --- |
| 106 | `892b46da` | 2 | 2 / 0 | `selfhost_input` is directly test-gated; `selfhost_tests` is beneath test-gated normalized tests and uses a legacy path attribute. Neither is loaded as an ordinary module for this replay root. |
| 108 | `81048410` | 1 | 0 / 1 | `forwarding` is declared by an existing optimizer-test module but belongs to `semantic_main.rs`, not the selected `main.rs`. Its `src/semantic_tests/optimizer/` directory is also absent. |
| 110 | `7b6d333b` | 3 | 2 / 1 | All three have conventional main-root routes from existing declaration files. `src/semantic/` must be created for `references.lkj`. The changed compiler files inspected are on that main module graph; no off-root changed file was found. |
| 115 | `47e98941` | 2 | 2 / 0 | Both are legacy path-bound modules inside compiler tests, excluded by the ancestor test condition. |
| 122 | `52e3e11c` | 1 | 1 / 0 | `typed_compiler_input_fixture_tests` is a legacy path-bound `fixture` module inside the selfhost test chain. An existing directory alone does not bring it into the selected root. |
| 127 | `685764ae` | 4 | 3 / 1 | All four new files have conventional main-root routes, but `src/driver/` is absent and the commit also changes the separate `src/lib.lkj` root. |
| 144 | `16055fc0` | 5 | 5 / 0 | All five are conventional modules directly declared in existing `main.lkj` or `native.lkj`. This is the cleanest existing-directory creation mechanism witness. The commit also changes off-root `src/lib.lkj`, which remains a complete-coverage blocker. |
| 152 | `d6ff7809` | 2 | 2 / 0 | The new `structure_refactor_cases.lkj` is a standalone root; it alone declares the new helper at this revision. Neither new file is wired into the selected main root. |
| 154 | `3556e582` | 2 | 2 / 0 | Both are legacy path-bound test modules. The new profile test module declares the new flow test module, so one declaration owner is itself new. |
| 157 | `acf74027` | 4 | 4 / 0 | Three scalar modules are conventionally declared in existing `elf_x86_64.rs` on the library-root module route. The fourth added file is a path-bound scalar-boundary test beneath the excluded selfhost test chain. |
| 184 | `411886df` | 11 | 7 / 4 | Three production `linear` modules have conventional library-root routes but require new directories; `unit_codec` uses a legacy path mount; seven additions are test-gated, including a source `include!` fragment. Several declaration owners are also new. |

These are projection and coverage classifications, not predictions that
the code will pass a new producer. The prior older-producer admission
reasons remain in the [original static assessment](compiler-strict-history-new-file-blockers.tsv).

## Exact directory and declaration dependencies

The seven missing-parent paths are:

- Ordinal 108: `src/semantic_tests/optimizer/forwarding.lkj`.
- Ordinal 110: `src/semantic/references.lkj`.
- Ordinal 127: `src/driver/structure.lkj`.
- Ordinal 184: `src/platform/kernel/codec/linear.lkj`,
  `src/platform/kernel/codec/linear/expression.lkj`,
  `src/platform/kernel/codec/linear/owner.lkj` and
  `src/platform/kernel/codec/linear/tests.lkj`.

The seven declarations owned by another new file are:

- Ordinal 152: the new standalone case root declares `structure_refactor`.
- Ordinal 154: the new profile-test module declares `flow`.
- Ordinal 184: the new `unit_codec` file declares its tests; the new
  sequence-test file declares its preparation module; the new `linear`
  file declares `expression`, `owner` and its test module.

Ordinal 184's `source_capacity_tests.rs` is included with
`include!("source_capacity_tests.rs")` inside an inline
`#[cfg(test)] mod tests` in `package_transport/source.rs`. It must not be
treated as a selected external-module source merely because a historical
source file names it.

## Smallest useful repair witnesses

For the proposed **existing-parent, ordinary external-module** operation,
ordinal 144 is the strongest source-creation witness in this fixed set:
four declarations added in the existing main file and one in the existing
native file, five destinations already inside existing directories, no
deletion and no declaration under another new file. It can validate the
creation mechanism without requiring directory creation or legacy mount
semantics. Its separate library-root edit must still be addressed before
the whole changed-language case can be counted.

Ordinal 157 offers a smaller three-module production subset in one
existing directory, but its added test source remains outside the selected
root and its historical code has separate admission uncertainty. Counting
only the three modules would narrow the original case and must be labeled
as such.

Ordinal 110 is the smallest fixed revision whose added modules all follow
the selected main-root graph without a discovered off-root changed file.
It needs **one new directory**, so it lies beyond the proposed initial
writer scope. This is a concrete next directory-creation case, not a
reason to silently create directories in an existing-parent operation.

The inventory therefore supports a narrowly bounded creation feature; it
does not supply six guaranteed additional complete cases or a route to
thirty by relabeling tests, includes or separate roots.

## Reproduction and evidence

The [inventory script](compiler-strict-history-new-files.pl) reads the
frozen eleven-row input, parent/target trees and manual route annotations.
It verifies declaring-file existence and writes the joined table:

```sh
perl bench/compiler-strict-history-new-files.pl . \
  bench/compiler-strict-history-new-file-blockers.tsv \
  bench/compiler-strict-history-new-files
```

Its [summary](compiler-strict-history-new-files.log) records 22 ordinary
external declarations, 13 legacy path-bound modules, one source include
and one standalone root. Eighteen paths have conventional selected-root
routes, fifteen are test-excluded, three belong to separate roots and one
requires a legacy mount that the current conventional loader does not
follow.

[Declaration excerpts](compiler-strict-history-new-files.declarations.txt)
and [supporting source references](compiler-strict-history-new-files.evidence.txt)
are retained as raw textual evidence. They are not authenticated semantic
query answers. The earlier scoped and four-probe manifests remain unchanged.
