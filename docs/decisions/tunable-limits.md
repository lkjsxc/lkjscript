# Tunable Limits

## Context

Weak models fail on deep nests and giant files.

## Decision

Enforce nest depth, child count, and per-file token budgets via `Limits`, with
defaults in config and CLI overrides.

## Consequences

Ecosystem grows as many small files and shallow defs. Numbers can move later.

## Rejected

Hard-coded sacred constants; unlimited source size.
