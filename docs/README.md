# Documentation

## Purpose

Provide the authoritative, status-labeled contract for `lkjscript`.

## Status

**Current.** This index defines the documentation status vocabulary.

## Status Vocabulary

Every behavior that is not unambiguously current must use one of these labels:

- **Current**: implemented and supported by recorded evidence.
- **Accepted Target**: approved contract that implementation is expected to
  reach next; it must not be described as already available.
- **Experimental**: implemented or proposed for measurement without an adoption
  decision.
- **Deferred**: intentionally outside the active implementation cycle.
- **Placeholder**: intentionally incomplete. Code and user-facing behavior must
  also say `PLACEHOLDER` wherever the incomplete surface is observable.
- **Rejected**: explicitly not an implementation target; retain the reason and
  evidence boundary until a later accepted decision replaces it.
- **Superseded**: historical contract that must not guide new implementation.

A mixed-status document must label the affected sections. Aspirations are not
release claims, and an old passing command is historical evidence rather than a
permanent property.

## Map

- [current-state.md](current-state.md): observed baseline and accepted next contracts
- [operations/architecture.md](operations/architecture.md): Current crate flow and accepted repository-intelligence flow
- [bounded topology](decisions/platform/bounded-repository-topology.md): authored repository limits,
  provenance, manifests, and audit contract
- [repository graph/context](decisions/platform/repository-intelligence-graph.md): derived
  identities, edges, budgets, and context profiles
- [agent work state](decisions/platform/agent-work-state.md): atomic task scope, attempts, evidence, and publication
- [language/](language/README.md): source format, semantics, imports, and limits
- [runtime/](runtime/README.md): VM and explicitly labeled JIT placeholder
- [operations/](operations/README.md): verification and engineering handoff
- [product/](product/README.md): validation applications and capability boundaries
- [vision/](vision/README.md): long-term direction and experiment registry
- [decisions/](decisions/README.md): active and superseded decisions
