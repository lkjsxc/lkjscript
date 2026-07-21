# XML Surface

## Context

Need an AI-friendly functional syntax aligned with lkjagent model actions.

## Decision

Use attribute-less XML-like tags; tag name is operator or special form.

## Consequences

Parser stays small; closing tags help weak models; humans rarely author by hand.

## Rejected

S-expressions (less aligned with existing agent grammar); ML-like surface (heavier parser).
