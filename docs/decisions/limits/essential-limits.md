# Essential Source Limits

## Purpose

Choose stable semantic budgets rather than misleading line-count rules.

## Status

**Current** for Edition 1 nest, form-child, token, and top-level limits.
The permanent-policy part of this record is **Superseded** by [Resource Budget
Profiles](../platform/resource-budget-profiles.md). No Current limit is weakened until
aggregate replacement bounds are Current.

## Decision

- Keep nest depth, form-child count, tokens per file, top-level forms, and
  source-directory width as hardcoded language-version constants.
- Treat token count as the primary structural file budget, while recognizing
  that raw text and aggregate imports require separate byte/resource limits.
- Do not expose user configuration or CLI overrides for syntax validity.
- Apply source-directory width only to lkjscript language trees.

## Current Migration Consequences

Edition 1 authors still split source to satisfy these exact limits. Changing a
Current limit requires docs, aggregate replacement boundaries, adversarial
tests, and edition migration rather than an unbounded local override. The
accepted destination reclassifies maintainability thresholds without making
source/compiler work unbounded.

## Supersedes

[../archive/tunable-limits.md](../archive/tunable-limits.md).
