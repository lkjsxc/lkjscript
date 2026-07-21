# Essential source limits

## Context

A fixed line-count gate poorly measures attribute-less XML density.
JSON-tunable limits invited ad-hoc policy drift.

## Decision

- Keep nest depth, children-per-element, tokens-per-file, directory fan-out,
  and top-level form caps as **hardcoded language constants**.
- Treat `MAX_TOKENS_PER_FILE` as the primary file-size budget.
- Do not expose a user-facing JSON `--limits` flag.

## Consequences

Authors split by meaning under fixed budgets. Changing numbers implies a
new language version, not a config tweak.
