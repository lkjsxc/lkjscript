# Tunable Limits

## Purpose

Preserve the rejected configurable-limit proposal.

## Status

**Superseded** by
[../limits/essential-limits.md](../limits/essential-limits.md).

## Historical Decision

The proposal would have exposed nest, child, file-token, and directory limits
through defaults and CLI/configuration overrides.

## Reason For Rejection

Environment-dependent syntax validity weakens reproducibility and invites
policy drift. Limits are now language-version constants changed through an
explicit contract and migration.
