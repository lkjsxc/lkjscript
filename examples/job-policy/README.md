# Deterministic job-admission policy

This public-path example builds and runs a pure policy through the production `lkjscript` CLI and
direct Engine. It decides whether a job fits CPU and memory limits, supports its target, and may use
a trusted release mode. Accepted jobs receive a deterministic score; rejected jobs return a named
reason. It has no host access or ambient authority.

The following is explanatory pseudocode, not lkjscript syntax:

```text
record Resources { cpu: i64, memory: i64, trusted: bool }
record Job { resources: Resources, target: Target, mode: Mode }
variant Decision { accept(i64), reject(RejectReason) }

decide(job, limits):
    reject jobs over CPU or memory limits
    reject unsupported targets
    reject untrusted release jobs
    otherwise accept(triangular(cpu) + memory + target_bonus + mode_bonus)
```

The driver creates seven named types and seven functions using closed typed operations. It publishes
an incomplete body, inspects repair context, proves a wrong-type repair publishes nothing and
consumes no durable identity, repairs the same durable hole anchor, runs accepted/rejected and lazy
low-fuel cases, renames a durable field, reopens the workspace, and verifies historical and current
revisions. It checks every identity, revision, value, trap, and result against an independent oracle.

JSON and context packets are proposals or observations; canonical `.lkjscript` revisions are the
authority. The current workload contains 189 semantic items but only 48 durable identities; body
scaffolding uses function-local references.

Run from the repository root:

```sh
./examples/job-policy/run.sh
```

The script builds release binaries, creates a private mode-0700 temporary state directory, reopens
that same state through direct commands, and removes only what it created. It requires stable Rust, a
POSIX shell, and Python 3 without third-party Python packages.
