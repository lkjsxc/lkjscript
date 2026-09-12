# Product identity surface

Status: normative.

## Version authority

The version of the root `lkjscript` package is the sole product-facing version. The library
projects it through `PRODUCT_VERSION`; the distributed executable uses that owner for its exact
version query, capability product record, logical-review provenance, and public release metadata.
No environment setting, generated copy, descriptor field, or subsystem table may override it.

The product version identifies one distributed snapshot. It is not semantic authority and does
not replace exact semantic revisions, content digests, package revisions, target triples, commit
SHAs, dependency versions, or external tool versions. Those values retain accurately labeled
identity domains.

The last verified immutable public release is product 0.1.32, from source
`67baaf0b081842e0e2e3745e8d5503e22cc791e4`; exact/latest public-pair acceptance is recorded in
the [iteration predecessor reconciliation](../campaigns/202609121214.md). The current
[foreground campaign](../campaigns/202609121842.md) selects a combined iteration/foreground release.
A source package version alone does not imply a tag or public release. Older tags, releases, assets,
and metadata continue to identify their original snapshots.

## Public projection

Default and focused capability discovery begins with product name/version and an opaque
capabilities digest. It then reports stable operations, grammar, fields, limits, diagnostics,
authority effects, project requirements, and security nonclaims. The digest binds the executable's
complete capability projection, is deterministic, and is cache evidence rather than program
meaning.

Focused inspection discovery includes one revision-pinned local-function definition contract. Its
grammar, exact record fields and forms, logical and physical admissions, `icont_` binding,
diagnostics, authority effect, and containment nonclaims are executable-owned capability records
and one generated guide. Projection version and digest identify derived transport; neither is a
product version or semantic revision, and neither is accepted as authored change input.

Logical review files retain exact base/result authority, semantic state, request and prepared-plan
commitments, effects, budgets, selected tests, and the review token. They may report product
version and capabilities digest as provenance. The opaque prepared-plan commitment is additionally
seeded by a digest of the complete independently owned internal compatibility state, so its binding
remains exact without rendering those identities as public review fields.

Deployment descriptors are strict unversioned operator policy. Removed version-discriminator
fields are unknown input and reject during bounded decoding before artifact, secret, adapter, or
listener access. Ready, stopped, and failure events preserve exact deployment/artifact observations,
diagnostics, receipts, and cleanup evidence without a subsystem version field.

Foreground `run --deployment` has no project discovery and returns production-only execution,
effective policy, typed result and completed cleanup records. An installed executable can execute
multiple runtime-dependent application bundles in separate processes. A bundle does not embed
the runtime, and this contract supplies no daemon, installer or automatic runtime selection.
Command policy omissions have deliberate meanings; explicit null rejects. Older executables
reject descriptors omitting their formerly required policy fields. Retain the matching executable,
immutable bundle and descriptor for recovery; executable updates do not migrate operational data.

Current public release metadata contains product name/version plus exact source, target, toolchain,
candidate, linkage, notice, archive, checksum, and integrity evidence. Private first-party handoffs
and receipts may retain independently owned compatibility state, but new public metadata cannot
copy it or select a historical writer.

## Internal and historical boundary

Typed storage, graph, request, plan-token, compiler, cache, artifact, deployment, runtime, adapter,
package, and contributor-evidence owners retain their own compatibility discriminants, magic
values, codecs, and digest domains. They continue to reject malformed, stale, foreign,
noncanonical, corrupt, and predecessor input. Hiding those values from public projections cannot
weaken their validation or binding.

Completed campaigns, durable decisions, structured evidence, immutable releases, and frozen
receipts retain terminology and identities that were true for their time. They are not current
product discovery. Source-owned internal constants and tests are likewise outside the product
projection.

## Current documentation ownership and audit

Current product prose consists of the root README; status, architecture, roadmap, security, and
release documents; all current specifications; generated public guides; the maintained standard
package and `lkjournal` guides; maintained deployment descriptors; and hosted release parsing.
The removed diff/merge specification is explicitly historical and excluded. Generated guides are
written and verified only by the executable.

`lkjscript-dev policy product-surface` owns the bounded audit for those files and the distributed
executable. It scans the two classified current documentation directories and an explicit file
set, executes the exact version query plus every maintained capability command/section focus, and
probes predecessor discovery requests. Historical, decision, evidence, campaign, and internal
source paths are excluded by classification. There are no content exceptions inside the current
boundary.

The cutover is direct: predecessor option, section, descriptor, event, review, generated-guide, and
public-metadata shapes reject or are absent. There is no compatibility alias, dual writer,
generation selector, fallback reader for current input, or replacement subsystem edition.
