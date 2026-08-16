# Retained release-channel policy replay

This retained black-box replay is the equal-task oracle used by the protocol-v5 authoring campaign. It was derived only after the before trial was sealed, from that trial's corrected accepted creation request.

The policy uses named `Version`, `Channel`, `Transport`, `Client`, `Policy`, `BlockReason`, and `Decision` data. Its entry function decides whether to serve a release and computes a channel-sensitive rollout weight with a counted loop. The checked-in `proposal.json` is direct protocol-v5 symbolic proposal material: every initial-construction reference uses a `DraftSymbol`; it is not a source language, generic rewrite input, or second authoritative program representation.

The driver uses only the public `lkjscript` CLI and local `lkjscriptd` service. It creates an incomplete immutable revision, obtains bounded repair context, proves invalid-repair publication and allocation rollback, refines the placeholder without changing identity or uses, exercises normal and low-fuel laziness cases, renames `rollout_steps` to `steps`, restarts, and verifies all three historical revisions. It prints one compact JSON summary containing semantic oracles and directly measured request, response, process, artifact, proposal, contract, and timing facts. Authoring interaction totals list discovery and typed lifecycle requests separately. Provider token telemetry is reported unavailable rather than estimated.

Run the retained production replay from the repository root:

```sh
./examples/release-channel/run.sh
```

The script builds optimized release binaries, creates a private temporary state directory through the driver, performs typed shutdown before restart and at completion, and removes only that temporary state. It requires a current stable Rust toolchain, a POSIX shell, and Python 3 from the standard library.
