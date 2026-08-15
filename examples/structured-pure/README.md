# Structured pure program repair

This retained example drives the production `lkjscript` generic CLI and `lkjscriptd` daemon through
strict version-3 JSON. JSON is transport; the daemon's canonical `.lkjscript` revisions are the only
program authority.

The workflow creates `range_sum(n)`, `normalize_and_sum(n)`, and `main()` in one structured
transaction. A loop-body `i64` hole is inspected through bounded repair context, an invalid repair is
rejected atomically, and the hole is refined in place to `add_i64(loop_carried, loop_index)`. It then
checks the semantic diff, prints `5050`, `0`, and `55`, restarts the daemon, and verifies both the
incomplete and repaired retained revisions.

From the repository root, run:

```sh
./examples/structured-pure/run.sh
```

From another directory, invoke `run.sh` by its absolute repository path.

Requirements are Linux x86-64, a current stable Rust toolchain, a POSIX shell, and Python 3
from the standard library only. The driver builds release binaries, creates a private temporary state
directory, never uses a hard-coded workspace ID, and removes only its own daemon process and state on
success or failure.
