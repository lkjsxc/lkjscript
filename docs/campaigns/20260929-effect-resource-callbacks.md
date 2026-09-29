# Effect-generic callbacks with exact resource authority

Date: 2026-09-29 (Asia/Tokyo).

## Request, authority and starting point

The user explicitly selected `https://github.com/lkjsxc/lkjscript`, delegated current
engineering/design judgments, permitted major style changes and requested a very-long-term
view with implementation. This is the language repository, not `lkjstr`.

Work started on local and independently fetched remote main
`a8bc93dcacd0d7b08bc8c4557744c4d7386df3c7` in the shared Home Coder workspace
`lkjsxc/tomato-ocelot-73`, checkout `/home/coder/workspace/lkjscript`. Existing unrelated
untracked files, retained worktrees, the historical prototype stash and running services
were not selected for modification. No public deployment or release was selected.

## Selected boundary and rationale

The language already combines ordinary type generics, explicit effect rows, direct
resource borrowing/consumption, recursion and exact package transport. However, a
resource-bearing helper still rejects effect parameters categorically. That prevents a
library from using an effectful decoder/processor while preserving a concrete resource
contract, even though callback effect substitution already exists elsewhere.

This increment admits ordered effect parameters on exact-resource helpers. An ordinary
callback may be a task whose effect row is parameterized by E. A direct call supplies E
explicitly; a recursive or public forwarder can pass its in-scope E onward. Every resource
parameter still binds a concrete requirement, and that requirement must appear explicitly
in the helper's task effect. E cannot supply, replace or alias the resource binding.

This deliberately does not change the parameter's requirement representation into a
symbolic requirement operand. Requirement-polymorphic resource signatures remain rejected.
Resource returns, captures, containers, indirect resource calls and asynchronous borrowing
remain outside the admitted boundary. Existing suffix ordering, duplicate-consume rejection,
per-body affine checking and call/resource limits remain in force. An empty-row task callback
is not implicitly interchangeable with a pure function.

This is a compositional language step toward ordinary user-authored libraries, not a region
allocator, general memory reference, trait system or evidence of zero-copy processing. It
avoids adding a parallel callback, ownership or runtime mechanism merely to demonstrate the
feature. The accepted typed graph remains semantic authority; prepared applications remain
disposable execution data.

## Implementation owners

- Canonical whole-snapshot and incremental affine validation admit effect parameters while
  retaining the requirement-parameter rejection and exact resource-binding checks.
- Strict artifact runtime-owner expectations retain and compare the complete ordered effect
  parameter list and full task effect, alongside ordinary type parameters and resource
  metadata. Admission does not simply drop the old guard.
- Normalized preparation retains exact canonical/compiled signature comparison and closes
  effect applications through the existing specialization path. VM/reference resource
  transfers and adapters are unchanged.
- The independent affine flow oracle reads local/imported signatures independently and
  distinguishes callback effects from concrete resource provenance.
- `affine_capability_resources` validator feature advances from 7 to 8, invalidating
  incompatible validation-witness reuse. Graph, package, artifact and data encodings are
  unchanged. Root development version advances to `0.1.60`.
- Native authoring guidance, language/CLI/verification contracts, diagnostic discovery,
  generated documentation, status and roadmap are aligned with the selected boundary.

## Authored witness and negative cases

The literal supplier `tests/fixtures/effect-resources-library.lkjc` exports
`read-lease<T,E>` and `finish<V,E>`. A private recursive reader forwards the callback and
borrowed resource. The literal consumer in `effect-resources-consumer.lkjc` forwards these
public contracts and instantiates both integer-list and Unicode-text callbacks. Each
callback writes to its own `app::audit` queue, while the resource retains the supplier's
`queue::jobs` authority. Separate deployment grants and durable roots make that distinction
observable rather than inferring it from returned values.

Public CLI tests cover normal planning/application/checking, unchanged canonical draft
re-entry, exact transport staging and detached artifact execution after both authoring
projects are removed. An additional case repeats one borrowed owner across a resource
suffix while forwarding E. Repeated completed jobs return their absent defaults; independent
job-byte inspection checks the original queue's completion and the audit queue's ready state.

Rejections cover omitted effect arguments, callback effect mismatch, an effect variable
standing in for concrete resource authority, rebinding a borrowed parameter to the callback's
requirement, consumption of a borrowed parameter, retained resource-bearing function values,
and pure/empty-task callback mismatch. Rejected plans preserve revision and project inventory.
The preexisting missing-effect-argument case remains rejected for arity, not for the removed
blanket prohibition. Requirement-generic and previous affine rejection cases remain retained.
A missing callback grant rejects before either initialized durable root changes.

Failure tests place the same trapping callback first inside a borrowed reader and then
inside a consuming helper after completion. Both retain the prior audit write. Only the
second retains a completed main job; the first retains the committed lease. Processing an
independent text job afterward must not rewrite the failed job. No implicit rollback, retry
or completion is inferred from local cleanup.

Internal tests additionally exercise empty and nonempty effect applications over the existing
scripted borrow/reborrow/consume, pending-return/tail, argument-failure, grant-limit and
cancellation workloads in both VM and independent canonical interpreter. The finite affine
oracle checks legal and illegal contracts across private/package/public visibility. Rehashed
hostile artifacts alter ordinary type parameters, effect parameter identity/presence, the
symbolic/concrete effect row and resource parameter binding/use/parent; exact admission must
reject them before execution.

## Development observations, not source acceptance

The original public-authored supplier test was first run against the unmodified implementation
at the starting source. It failed during `change plan` with
`kernel_affine_function_resource_generic`, specifically because of effect parameters, not
because of parsing or fixture syntax. Retained log:
`/tmp/lkjscript-effect-resource-before-retry.log` (one failing test, 1.62 seconds).

Two intermediate implementation attempts exposed independent downstream guards: whole-source
package revalidation still rejected effect parameters, and artifact runtime-owner matching
still assumed an empty effect-parameter list. Both were corrected at their actual owners;
artifact matching now checks complete exact metadata rather than accepting an unchecked row.
The earlier interrupted compilation log is not treated as a test result.

At intermediate working trees before the version/documentation update, three selected internal
resource-effect tests passed (2.37 seconds), and five public callback cases passed (17.39
seconds). Their logs are `/tmp/lkjscript-effect-resource-internal.log` and
`/tmp/lkjscript-effect-resource-public-expanded.log`. These observations are not a full-profile
receipt and do not validate later edits, generated assets or a release.

## Acceptance and delivery

The selected completion requires an identified implementation source, generated documentation
through product discovery, the unchanged full profile with fresh execution, and independent
remote main verification after ordinary integration. Results and exact source/receipt identities
are recorded below only after they are observed. No release tag, promotion, anonymous acquisition
or replacement of an existing service is implied by source integration.
