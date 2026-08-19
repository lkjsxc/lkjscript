# Release-channel policy replay

This retained black-box replay is an equal-semantics control for explicit and inline proposal forms.
It models named version, channel, transport, client, policy, block-reason, and decision data. Its
entry function decides whether to serve a release and computes a channel-sensitive rollout weight
with a counted loop.

The checked-in `proposal.json` is a diagnostic protocol-v11 proposal. The driver can replay it or
derive an equal typed inline proposal by replacing only eligible contiguous one-use postorder values.
That helper is not source syntax or a second program representation.

Through direct Engine commands the driver creates an incomplete revision, obtains repair context,
proves invalid-repair rollback, repairs the durable hole anchor, exercises normal and low-fuel lazy
cases, renames `rollout_steps` to `steps`, reopens state, and verifies three historical revisions. It
prints exact request, response, process, artifact, contract, and timing measurements. Provider
telemetry is reported unavailable rather than estimated.

Run from the repository root:

```sh
./examples/release-channel/run.sh
```

Inline mode is the default. Set `LKJSCRIPT_AUTHORING_MODE=explicit` for the sealed explicit control.
The script uses production binaries and removes only its private temporary state.
