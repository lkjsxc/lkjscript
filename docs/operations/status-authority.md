# Capability Status Authority

## Status

Accepted and binding for Current claims.

## Registry

`meta/config/capability-status.json` is the machine-readable authority. Its
stable schema is `lkjscript.capability-status`; `contract` must equal the full
digest of the descriptor returned by `lkjscript describe`. Capability IDs are
stable, unnumbered names. A changed interface changes its descriptor digest,
not its public name.

The registry records an ID, closed status, interface, authority, evidence, and
all Current claimants. `lkjscript-xtask` rejects duplicate, unsorted,
generation-suffixed, stale-contract, missing-link, and claimant mismatches.

## Closed statuses

- `current`: implemented and covered by the named evidence.
- `accepted-target`: binding destination not yet fully implemented.
- `accepted-contract`: accepted interface awaiting its complete implementation.
- `accepted-selection`: selected measured candidate awaiting final integration.
- `experimental`: implemented only as an experiment.
- `deferred`: explicitly not scheduled for the Current slice.
- `rejected`: evaluated and not accepted.
- `superseded`: retained only as decision history.
- `historical`: retained evidence, never a Current capability.

Historical and superseded records live under `docs/history/` or immutable
retained evidence. They do not occupy Current registry IDs and do not provide
fallback acceptance.

## Claim rule

Every `<!-- LKJ-STATUS id=... status=... -->` marker must appear under a
`## Status` heading, match one registry claimant exactly, and use the registered
status. Current prose must not use numbered language generations, schema or ABI
generation suffixes, or obsolete contract identities.
