# lkjscript-linux-host

Private Linux host-observation and worker-binding mechanism crate.

It owns bounded topology and scheduler discovery plus checked affinity changes.
Its public Rust facade is safe, and the sole unsafe ABI file remains registered
under the existing `linux-host-io` boundary. Non-Linux execution is not claimed.
