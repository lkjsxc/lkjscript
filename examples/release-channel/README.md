# Retained release-channel policy replay

This retained black-box replay is the equal-task oracle used by the protocol-v5 and inline-authoring campaigns. It was derived only after the before trial was sealed, from that trial's corrected accepted creation request.

The policy uses named `Version`, `Channel`, `Transport`, `Client`, `Policy`, `BlockReason`, and `Decision` data. Its entry function decides whether to serve a release and computes a channel-sensitive rollout weight with a counted loop. The checked-in `proposal.json` is the direct explicit proposal consumed by the current protocol-v8 driver. The driver can replay it unchanged or deterministically derive the equal-graph inline mode by replacing only eligible contiguous one-use postorder values. This measurement helper is not a source language, accepted service input, or second authoritative program representation.

The driver uses only the public `lkjscript` CLI and local `lkjscriptd` service. It creates an incomplete immutable revision, obtains bounded repair context, proves invalid-repair publication and allocation rollback, refines the placeholder without changing identity or uses, exercises normal and low-fuel laziness cases, renames `rollout_steps` to `steps`, restarts, and verifies all three historical revisions. It prints one compact JSON summary containing semantic oracles and directly measured request, response, process, artifact, proposal, contract, and timing facts. Authoring interaction totals list discovery and typed lifecycle requests separately. Provider token telemetry is reported unavailable rather than estimated.

Run the retained production replay from the repository root:

```sh
./examples/release-channel/run.sh
```

The default is inline mode. Set `LKJSCRIPT_AUTHORING_MODE=explicit` to replay the sealed explicit
proposal for equal-work comparison.

The script builds optimized release binaries, creates a private temporary state directory through the driver, performs typed shutdown before restart and at completion, and removes only that temporary state. It requires a current stable Rust toolchain, a POSIX shell, and Python 3 from the standard library.
