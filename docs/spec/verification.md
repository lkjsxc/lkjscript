# Verification command contract 2

`tools/check` is the repository-owned verification entry point. It owns gate orchestration and
evidence framing, not semantic authority or a substitute for tests.

Accepted profiles are `focused`, `changed`, `product`, `service`, and `full`. Unknown profiles or
options reject. `changed` derives paths from exact Git porcelain facts. Runtime/Cargo/test or any
unknown path widens to full; package/application changes select product plus service; documentation
alone selects diff validation. Selection is conservative convenience and never narrows `full`.

Gates run sequentially with `CARGO_NET_OFFLINE=true`, a 3,600-second default deadline, and a 64 MiB
combined stdout/stderr retained-byte maximum per gate. They use new process groups. Timeout,
cancellation, log excess, command absence, nonzero exit, and success remain distinct statuses.
After the first non-pass, remaining gates are explicitly `unrun`.

Every run has a new directory under `.artifacts/check/`; the newest eight are retained. Each gate
has separate stdout and stderr logs, exit status, command vector, elapsed nanoseconds, byte counts,
and a SHA-256 output fingerprint. Failure output contains bounded head/tail excerpts and exact log
locators. Default all-pass output is one aggregate line plus receipt. `--machine` emits one compact
JSON object; `--details` is an explicit human expansion.

Receipt contract 2 records profile, selected/completed/passed/unrun gates, Git head, SHA-256 over
Git status and every tracked/untracked nonignored input byte, Rust/Cargo/Python/platform facts,
offline policy, elapsed time, and gate receipts. No cross-run pass reuse exists. Full therefore runs
all gates even when an identical pass receipt exists; ordinary compiler caches may accelerate work
without becoming authority.

`--self-test` independently exercises success with separate streams, exit 7, unavailable command,
timeout cleanup, and log overflow. Full includes that self-test, format, clippy, all-target locked
tests, release build, both package differential suites, both deep authority doctors, deterministic
artifact reproduction, live PostgreSQL service acceptance, and `git diff --check HEAD`.

The service profile requires Docker and an already available `postgres:16-alpine` image. Absence is
a failed/unavailable verification fact, never a skip converted to success. Complete service logs,
PostgreSQL backup, runner events, timings, and receipt are retained separately under
`.artifacts/service/` with secrets redacted.
