# Public topology authority and unified recipe lowering

Date: 2026-09-01 UTC.

## Status

Accepted and implemented in unreleased product snapshot 0.1.14, CLI contract 16, and compact-change
records contract 8. Project creation remains contract 3.

## Problem

The typed authored-change model could create dependencies, components, ports, and targets, but the
public compact surface could not. Built-in recipes therefore assembled semantic owners and
snapshots in private Rust, and the maintained stateful HTTP workflow had to inherit topology from
the `http` recipe. That split made a copied executable unable to author a complete application from
`minimal` and left two construction mechanisms for the same graph meaning.

## Decision

- Public compact change owns exactly these additional records:

  ```text
  add.dependency package=PKG semantic-revision=REV package-revision=PACKAGE_REVISION
  create.component as=$COMPONENT module=MODULE name=NAME visibility=private|package|public
  add.port as=$PORT component=COMPONENT name=NAME type=TYPE function=DECLARATION
  create.target as=$TARGET name=NAME component=DECLARATION port=PORT runner=command|http
  ```

- Dependency addition accepts only the exact current built-in binding after its immutable transport
  is staged. It performs no registry, network, ambient-directory, or unchecked-file lookup.
- Components are created empty. Requirements and function-backed ports are separate bounded
  operations. A port's explicit function type must equal its implementation type. A target binds an
  exact component and one of its ports; component requirement closure and command/HTTP runner shape
  are validated before publication.
- Request-local forward references, canonical commitment, budgets, allocation, logical planning,
  relation/impact evidence, selected tests, full validation, stale-base/token handling, idempotent
  re-preparation, and atomic publication are the existing authored-change mechanisms. There is no
  `new`-only topology path or compatibility spelling.
- `minimal`, `command`, `http`, and `nostr-relay-info` remain the finite built-in recipe names. Every
  recipe's semantic content is typed authored intent made only from public-representable operations
  and lowered by the same engine as public plan/apply. Internal recipes need no compact-text round
  trip, but may not assign semantic IDs, construct owner records or snapshots, or validate through a
  second path.
- `new` still validates a complete private candidate, creates exactly one initial accepted graph
  revision, synchronizes its bounded auxiliary inventory, and exposes the destination by one rename.
  The HTTP and relay-information descriptors and empty generated directories remain separate
  operator authority; artifacts remain explicit derived build output.
- The maintained stateful HTTP oracle now begins at `minimal`, publicly stages the exact built-in
  transport, and submits one reviewed request containing its dependency, complete topology, and BBS
  meaning. It does not call a recipe builder or inherit the `http` recipe topology.

## Consequences

A copied current-source executable can author a complete command or HTTP application topology from
an empty package without source, Cargo, a registry, network resolution, or a private construction
fixture. Recipe behavior remains executable-owned and finite, while accepted expanded graphs remain
the sole editable program authority and are unaffected by later recipe changes. The former
recipe-specific owner/snapshot builder and direct semantic-ID allocation are deleted.

This does not expose expression-backed ports, target replacement, dependency removal, arbitrary
package transports, a general registry, user recipes, additional runners, WebSocket, or deployment
policy as graph meaning. Immutable public v0.1.13 remains unchanged; this source cutover is not a
release.

## Reversal conditions

Broaden dependency transport or runner kinds only from a maintained copied-binary workflow with an
exact authority owner, bounded discovery and failure behavior, independent live and pure oracles,
and a dependency-closed migration. Reintroducing a recipe-specific semantic builder requires proof
that the public authored model cannot express a necessary invariant and a simultaneous removal of
the displaced path; convenience or compact-text avoidance is insufficient. Project-creation
atomicity and deployment separation may change only through a separately versioned observable
contract with crash/recovery proof.
