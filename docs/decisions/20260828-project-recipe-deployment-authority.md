# Atomic project recipe and separate deployment authority

Date: 2026-08-28 UTC.

## Status

Implemented for the closed `minimal`, `command`, and `http` project recipes.

## Problem

A copied executable could create command Graph 5 authority, while the maintained HTTP runtime
required topology that no complete distributed authoring workflow could create. The smallest
useful HTTP project also needs a deployment descriptor and future artifact output location. A
post-creation descriptor write would expose an incomplete destination; treating that descriptor as
graph meaning would merge operator policy with program authority.

## Decision

- Keep one closed executable-owned recipe set. A recipe constructs typed meaning and is neither a
  user-defined template language nor durable authority after publication.
- Build and fully validate Graph 5 meaning before publication. The accepted graph remains the sole
  editable program authority and later response edits use reviewed `change plan` / `change apply`.
- Permit a recipe to place a bounded, validated auxiliary inventory in the same private sibling as
  the repository. For `http`, that inventory is one typed deployment-contract-1 descriptor and one
  empty ordinary artifact-output directory.
- Synchronize canonical repository data and every auxiliary entry before the single destination
  visibility rename, then reopen and reconcile both inventories. A previsibility failure removes
  only the owned private stage.
- Keep the deployment descriptor separate mutable operator authority. Its listener, grant binding,
  operational identity, and resource limits do not enter semantic state, revision, or root merely
  because they share a destination directory with the repository.
- Generate no application artifact during creation. Artifact 10 remains a deterministic derived
  output of explicit `build`, and resident execution consumes only descriptor, artifact,
  environment, and named host resources.

## Consequences

One copied executable can create a complete editable HTTP topology and a safe loopback starter
deployment without Cargo, checkout assets, a registry, database, container, helper process, source
file, or frozen bootstrap artifact. Destination visibility remains atomic across the useful
project inventory, while graph and deployment mutations retain distinct contracts and identities.
Generic component/requirement/port/target authoring is still private and requires a separately
selected maintained workflow.

## Reversal conditions

Add another auxiliary kind only when a maintained recipe requires it, its authority owner and path
contract are explicit, and the complete inventory can preserve the same one-rename failure model.
Replace the closed recipe with broader public topology authoring only after a second real workflow
proves a dependency-closed contract and all maintained consumers move in one cutover. Never retain
both paths as editable authorities.

The reversal condition was met by the maintained stateful HTTP, relay-information, and `lkjournal`
workflows. [Public topology authority and unified recipe lowering](20260901-public-topology-authority.md)
records the dependency-closed cutover; this record retains the original project-creation and
deployment-separation rationale.
