# Built-in bootstrap ownership and recipe authority

Date: 2026-08-22 UTC.

## Status

Accepted and implemented for bootstrap contract 2 and meaning graph contract 4. Copied-binary,
exact reproduction, conflict, and complete-workflow coverage lives in `tests/public_cli.rs`.

## Decision

The executable embeds one exact graph-native standard-package artifact and a small manifest that
binds its bootstrap contract, graph contract, package identity, semantic revision, artifact digest,
bundle digest, and byte length. Repository verification compares the embedded bytes with maintained
standard authority, while runtime loading verifies their artifact bindings. `package builtin
inspect` exposes the manifest and `package builtin export --output PATH` writes the exact embedded
bytes without permitting ambient replacement.

`new` supports `minimal` and `command` bootstrap recipes. The executable owns these versioned,
deterministic recipes. The command recipe lowers through the same change-v3 implementation used
after project creation and uses typed request-local symbols. The zero-base publication path
allocates repository and package identities, resolves the exact built-in dependency, fully
validates the graph, writes private durable state, and makes the project visible at one atomic
rename.

The minimal recipe creates an ordinary package with one empty `app` module and no standard
dependency. The command recipe creates a graph-owned function, test, component, port, command
target, and exact standard dependency. The receipt returns the template, project, repository,
package, revision, root, optional built-in dependency manifest, allocated-identity map, and
publication receipt. The publication receipt binds the exact transaction digest; `new` does not
return a separate normalized-change document or recipe digest.

## Authority

The maintained accepted standard graph under `packages/standard/.lkjscript/meaning` is the sole
authored authority for standard meaning. The embedded artifact is an immutable derived replica for
offline distribution. Its build input must reproduce the embedded bytes exactly; copying the same
artifact into an application dependency directory does not create another authority.

Built-in recipes are executable-owned, versioned authoring behavior. They are not accepted program
meaning, source code, storage authority, or a hidden mutable template store. After expansion, only
the validated declarations published in the project's meaning graph are program authority. Later
recipe changes do not alter existing projects.

Repository identity, package identity, semantic revision, artifact digest, bundle digest,
transaction digest, template name, and filesystem location remain separate domains.

This decision record is evidence. The accepted project graph and exact revision records remain
program authority.

The current embedded package is standard revision
`rev_1af582dbebc01b43cd1050349f208b7c71c92ca4efd3f6b65624745f7d9c988e`, package artifact
`artifact_6ea73654d153ac4410ff4aaad329373dce27a58bb0d8c61eaa31cd6d66bcb3f6`, and bundle digest
`artifact_3648f87daea0164ef6e94ea6e731dd687db590b8889583f63cac6587f5e7a4d1` (22,264 bytes).

## Invariants

- A copied executable and an empty directory are sufficient; bootstrap performs no network lookup
  and reads no ambient package, recipe, registry, repository checkout, or Rust toolchain.
- The embedded artifact is strict, bounded, integrity checked, inspectable, exportable, and bound to
  the executable's declared bootstrap and graph contracts.
- Build reproduction fails on byte, identity, revision, manifest, or digest drift.
- Runtime corruption fails before dependency staging or project visibility.
- Recipe structure and lowering are deterministic for the executable/bootstrap contract, selected
  template, package name, built-in manifest, and change schema. Production repository/package IDs
  are deliberately fresh, and created stable IDs are returned in the receipt.
- A recipe uses typed local symbols; it cannot prescribe physical storage keys or smuggle accepted
  graph records around validation.
- The command recipe uses the ordinary change normalization and commit path inside the private
  stage; `new` has no separate public preview mode. Publication rechecks every exact input before
  the single visibility point.
- A missing, invalid, stale, or incompatible built-in dependency publishes nothing.
- Nonempty, symlink, non-directory, or unsafe destinations reject without changing user-owned
  contents. Failed private stages remain unreachable and removable.
- The initial revision satisfies the same canonicalization, semantic validation, immutable-object,
  durability, receipt, and HEAD invariants as later publication.

## Complete consumers

- `new --template minimal` creates an inspectable package that can be changed, tested, built,
  backed up, restored, and checked by deep doctor.
- `new --template command` creates, tests, builds, and runs a nontrivial command application, then
  survives a public rename/refactor, backup, blank-directory restore, and deterministic artifact
  comparison.
- The standard package build reproduces the embedded bytes, and both the command application and
  `lkjournal` consume the same exact package-object contract.

These consumers do not establish a general user-defined template language. Such a language needs
multiple independent authoring workflows and its own security and lifecycle evidence.

## Rejected alternatives

- Requiring an external `.lkja`, repository checkout, Cargo, registry, or network for first use is
  rejected.
- Deterministically reconstructing the entire standard package in private Rust builders is rejected
  because it duplicates maintained graph meaning and public authoring semantics.
- Treating the embedded bytes as a mutable or independently edited authority is rejected.
- Loading an ambient file as a silent built-in override is rejected.
- Storing recipe definitions in accepted application graphs is rejected because templates affect
  future authoring, not the meaning already produced.
- Recursive transaction JSON that mirrors Rust enums is rejected as the only practical recipe
  form. Recipes lower from compact semantic intent to the exact public change protocol.
- Private builders for maintained sample applications are rejected after public recipe/change
  parity exists.
- A general macro language, natural-language generator, online package registry, mandatory daemon,
  or compatibility reader is not introduced.

## Incremental invalidation

- A standard-graph change invalidates standard reproduction evidence, embedded bytes and manifest,
  binary identity, and new-project dependency bindings. Existing projects remain pinned to their
  old exact artifact until an explicit dependency change.
- A recipe-definition change invalidates discovery examples, new-project receipts, embedded
  bootstrap expectations, and recipe acceptance tests. Previously accepted expanded graphs are
  unchanged.
- A recipe-parameter change invalidates only that expansion and its prepared publication.
- A bootstrap-, graph-, validator-, artifact-, package-, storage-, or public-change-contract change
  invalidates all affected preparation and reproduction evidence.
- Any future prepared expansion cache key must bind binary identity, bootstrap contract, template
  definition, parameters, explicit replay seed, built-in manifest, graph contract, validator
  contract, and public-change schema. Current publication binds and rechecks the zero base and
  destination.

Initial creation deliberately uses the full validator. Incremental reuse begins only after the
accepted initial revision exists, and loss of recipe or bootstrap caches cannot alter authority.

## Reversal gates

- Replace embedded artifact bytes with deterministic construction only if construction uses the
  public change contract, remains fully offline, materially reduces complete-workflow cost or binary
  size, and reproduces the same exact standard authority without a second builder model.
- Add another built-in package only when at least two complete offline consumers require it and the
  added binary size, startup cost, update policy, and package identity are measured.
- Add a user-defined recipe contract only when multiple independent applications need reusable
  creation behavior, expansion remains inspectable and deterministic, and accepted graphs remain
  readable after recipe removal.
- Change the visibility mechanism only when crash evidence shows a stronger portable atomic
  publication design.

No reversal may permit an ambient override, dual authority, or compatibility fallback.

## Security and non-goals

Embedded digest verification proves integrity within the executable and artifact contracts; it does
not independently prove build provenance or publisher trust. Bootstrap artifacts and recipes
contain no secrets, credentials, deployment grants, live resources, host paths, or network-derived
state. Artifacts declare capability requirements only; deployment remains external authority.

This decision does not add a registry, dependency solver, remote updater, package signing system,
dynamic evaluation, textual source, hostile-code sandbox, multi-tenant isolation, TLS, certificate
management, ACME, or PostgreSQL TLS. Plaintext transport and deployment trust boundaries remain
separate runtime concerns.
