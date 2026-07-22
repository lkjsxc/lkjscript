# Essential Source Limits

## Purpose

Choose stable semantic budgets rather than misleading line-count rules.

## Status

**Current** for nest, form-child, token, and top-level limits.
**Accepted Target** for the 16-entry language source-directory rule.

## Decision

- Keep nest depth, form-child count, tokens per file, top-level forms, and
  source-directory width as hardcoded language-version constants.
- Treat token count as the primary structural file budget, while recognizing
  that raw text and aggregate imports require separate byte/resource limits.
- Do not expose user configuration or CLI overrides for syntax validity.
- Apply source-directory width only to lkjscript language trees.

## Consequences

Authors split source by meaning. Changing a limit changes the language contract
and requires docs, boundary tests, and migration rather than local config.

## Supersedes

[../archive/tunable-limits.md](../archive/tunable-limits.md).
