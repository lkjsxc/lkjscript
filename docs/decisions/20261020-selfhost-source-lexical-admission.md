# Self-hosted source ownership and lexical admission

## Slice decision

- Boundary: extend the admitted `LKJSG001` source carrier from endpoint closure to
  kind-specific source membership, unique syntax ownership, containment cycles,
  and exact-identity lexical visibility for the currently decoded source profile.
- Priority: E1 is the next unimplemented semantic boundary at baseline
  `3bbd8a93196978e6467d362c20f16fe0c90d2f7c`. Initial inspection confirmed ample disk
  and memory headroom. A subsequent public-transaction proof-work refusal required
  the narrow dependency recorded in
  [call-scope index admission](20261020-call-scope-index-admission.md), without
  increasing the existing limit. The canonical checkout is the only writer's
  source; all Cargo operations use its exclusively leased `target` directory.
- Effect: a standalone lkjscript-written validator rejects rehashed, endpoint-valid
  ownership and lexical violations before semantic lowering. Complete inventories
  are checked, including unreachable declarations, untaken operands and unused
  syntax. The canonical graph remains authoritative; derived numeric tables carry
  no independent admission authority.
- Acceptance: positive full compiler and independent carriers; legal nested and
  sequential shadowing; exact-addressed references to earlier identities despite
  same-name shadowing; shared type descriptors; recursive calls; negative missing,
  duplicate and inconsistent containment, orphan syntax, disconnected cycles,
  initializer self/forward references, cross-function and same-function sibling
  escapes, and reference-variant/binder-kind mismatches. Bind all carrier
  bytes, byte length, semantic state and complete inventory. Compare independent
  expectations with VM, canonical reference and native execution. Work exhaustion
  returns resource refusal, never a semantic-invalid claim. Finish all source,
  proposal, specification and status edits before fresh full verification.
- Remaining obligations: complete type/custody/effect/witness checking,
  source-to-normalized lowering, broader source forms, native lexical region
  reclamation and full Tier 2 dual-output source fixed points. This slice supplies
  no certificate of type correctness or Tier 2 completion. Nominal recursion, call
  recursion and region data cycles retain their distinct contracts.

## Implementation contract

Containment is derived from original canonical object fields after carrier and
endpoint admission. Fixed-width rows use owner-directory indices rather than
names. Sorting and checking the complete child inventory establishes unique
syntax membership; reverse member fields must agree with their containing owner.
Modules and the other designated top-level roots are admitted by kind. Shared
content-addressed types are never subjected to syntax-parent uniqueness.

Lexical admission follows exact binder identities and ordered let initializers.
An initializer sees preceding bindings but not itself or subsequent bindings;
a let body sees every binding in that let. Nested bindings do not hide an explicitly
addressed earlier identity. A parameter is visible only in its own callable body.
Containment and lexical proof work are bounded independently of runtime allocation
and process RSS. No native reclamation or bounded-RSS claim follows from these
checks. Detailed admitted forms, diagnostics and limitations belong to
`docs/spec/selfhosting-source-input.md`.
