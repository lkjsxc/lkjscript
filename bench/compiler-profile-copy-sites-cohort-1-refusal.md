# Copy-site cohort 1: preflight refusal

The once-only invocation released at `be0b6ad9` stopped in the launcher's
preflight on 2026-11-16 at 13:44:53 UTC. It executed **zero compiler phases**
and created no run directory. All seven declared phases are explicitly not run.
The raw [completion](compiler-profile-copy-sites-cohort-1-refusal-data/phase/completion),
[not-run table](compiler-profile-copy-sites-cohort-1-refusal-data/phase/not-run.tsv)
and exact release remain retained. No construction, execution, decode or
accounting measurements exist for this cohort; elapsed/CPU/RSS/load observations
for those phases are unavailable, not zero.

| Binding | Identity |
| --- | --- |
| Cohort | compiler-copy-sites-20261116-1 |
| Gateway request | compiler-copy-sites-fixed-witness-20261116-once |
| Committed release | be0b6ad9e6ec476f0f1584e01f85528b5faf0538 |
| Release JSON | 7440f30b53c1e0f297caabaaa65c9ccc8763ebcf40ed6405bda56f8c4765bdfa |
| Original launcher | 47d085bc0cf1b0a48be56fcfda8efcec5cba85162dcdf9031f5da5ba7e6140e6 |
| Corrected launcher | abf52e1d5f22370a999e6e7765c4b28fe34671ff16430aa93da8d7112fd71bd6 |

## Cause and correction

The preflight Perl helper `die_if` had no prototype. In the relative-path
predicate, its final positive regular-expression match inherited list context
through a short-circuit expression. For a valid relative path, that failed
match returned an empty list. The call consequently passed only the string
`unsafe relative path`; the helper treated that string as its condition and
then tried to print its absent second argument. The observed diagnostic is
an uninitialized `$_[1]` warning followed by `release_preflight_failed`.

An instrumented read-only extraction reproduces that exact one-argument call.
The repair gives `die_if` a two-scalar `($$)` prototype and an explicit
arity/message diagnostic. All predicates, pin requirements, workload commands,
limits and output policies remain unchanged. Root compared the complete old
and new launchers; that helper definition is the only change.

The [diagnostic bundle](compiler-profile-copy-sites-cohort-1-refusal-data/diagnostic/README.md)
retains the frozen original, stack trace, focused context reproduction, exact
diff, syntax checks and twelve pure Perl controls. Exactly one corrected,
extracted read-only preflight also passes against the original committed
release and original launcher identity, with empty stderr. It validates those
inputs; it does not execute the corrected launcher. Its report is SHA-256
`55c6f01e1d0e024b7114d0b395e0801a57867c48aa26fb03e782cd367937691a`.

The initial generation of the repaired candidate also had a JavaScript
replacement-string escape error, reducing `$$` to `$`. Syntax checking caught
it before invocation. That candidate and failure remain in the diagnostic
bundle; the corrected candidate uses the literal two-scalar prototype.
No compiler, measured image, official profile reader or accounting helper ran
in diagnosis. The 51-record diagnostic manifest is
`54af81ffb4635d6d77738afa6f09e37a6a1c2988b82001d6c7afac6bd432eaf0`.

## Preservation and next cohort

Cohort 1 remains closed. Its original release files are unchanged; its original
launcher and initial preparation report/manifest are retained here as exact
copies as well as in Git. The live twelve-file evidence directory was copied
before any repair. Source, library, runtime, seed and producer identities remain
unchanged, and the passed source gate/CI are still the same evidence.

Any later workload needs a distinct committed cohort and root release with the
corrected launcher hash. It must keep the fixed source, producer, ordinary/profile
order, all bounds and byte oracle. A later successful cohort cannot turn this
preflight refusal into a successful observation. The current preparation manifest
may bind the corrected current launcher; preparation-v1.sha256 preserves the
original manifest and resolves its historical paths at commit 6fc9099c.
