# Copy-site cohort-1 preflight diagnosis

Original launcher: 47d085bc0cf1b0a48be56fcfda8efcec5cba85162dcdf9031f5da5ba7e6140e6.
Original release: 7440f30b53c1e0f297caabaaa65c9ccc8763ebcf40ed6405bda56f8c4765bdfa.
Original release HEAD: be0b6ad9e6ec476f0f1584e01f85528b5faf0538.
Cohort 1 remains consumed; it ran zero compiler phases and created no run directory.

The unprototyped die_if call in rel() evaluated its final positive regex match
in list context, propagated through the short-circuit predicate. A valid
relative path makes that regex fail, returning an empty list rather than one
scalar false value. The call therefore supplied only "unsafe relative path".
That string became the condition and the message argument was absent, causing
the observed uninitialized $_[1] warning and refusal.

preflight.original.pl is the exact frozen extraction. The instrumented copy
changes only error/arity reporting; its retained stack trace shows arity 1 at
rel("bench/compiler-profile-copy-sites-release-20261116.json"). The instrumented
read-only reproduction exited 255. It did not invoke the launcher or compiler.

argument-context.pl independently reproduced the original missing argument
for two valid paths and checked a scalar-expression alternative against six
invalid-path cases. Its exit was zero. repair-proposal.md records that initial
alternative; root selected the API-level prototype repair below instead.

Corrected candidate:
compiler-profile-copy-sites.prototype-fix-v2.sh
SHA-256 abf52e1d5f22370a999e6e7765c4b28fe34671ff16430aa93da8d7112fd71bd6.

Only die_if changes: a two-scalar ($$) prototype plus an explicit arity/message
diagnostic. All rel() predicates and every policy/pin/bound remain unchanged.
The prototype consistently gives the condition scalar context. Existing callers
are predicates: comparisons, file tests, explicit scalar grep, and system exit
checks retain their intended semantics; none is an intentional list supplier.

Bash syntax and the corrected extracted Perl syntax pass. prototype-controls.pl
passes twelve controls: two valid relative paths, six unchanged invalid-path
refusals, three explicit arity/message refusals, and a direct failed-regex
predicate. Controls have no file/Git/compiler/profile calls.

Exactly one corrected full read-only extraction ran against the exact original
release and evidence, with GIT_OPTIONAL_LOCKS=0 and all output in this directory.
corrected-readonly.status is zero; stderr is empty. The report binds release
7440f30b, HEAD be0b6ad9 and producer
6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc.
It validates the original frozen launcher identity as an input; it does not
claim that the corrected candidate was released for execution.

A candidate-generation escape error was caught before that invocation:
JavaScript replacement-string rules reduced the intended $$ prototype to $.
The v1 candidate, extracted syntax failure, exact gateway result and
candidate-generation-v1.failure.json remain preserved. A literal replacement
callback produced v2; the v1 failed syntax check is not a cohort attempt.

No original launcher, release or cohort evidence was modified by this diagnosis.
No compiler, measured image, profile reader or accounting helper ran. No launcher
retry or cleanup occurred. The next cohort requires a new committed prospective
record and separate root release.
