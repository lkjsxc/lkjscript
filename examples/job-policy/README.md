# Deterministic job-admission policy

This example builds and runs a pure policy application through the real local background service
(`lkjscriptd`) and the production generic `lkjscript` CLI. The policy decides whether a job fits
CPU and memory limits, supports its target, and may perform a trusted release. Accepted jobs receive
a deterministic score; rejected jobs return a specific reason. It does not schedule a process or
access files, time, randomness, networking, environment variables, or any other host resource.

The following is **explanatory pseudocode, not lkjscript source syntax**:

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

The real program is created from closed typed operations. JSON carries those operations to the CLI;
it is not stored as another program representation. Immutable `.lkjscript` revisions managed by the
service remain the authoritative program.

## What the workflow proves

The initial transaction creates three named record types (`Resources`, `Limits`, and `Job`), four
variant types with fixed alternatives (`Target`, `Mode`, `RejectReason`, and `Decision`), and seven
functions. Their bodies exercise nested named data, field projection, calls, conditions, a counted
loop, checked addition, and complete handling of every `Target` and `Mode` alternative.

`score` initially ends at an `i64` typed placeholder. The driver:

1. discovers the machine-contract manifest, the current task-relevant sections, and the compact
   unchanged response for a known fingerprint;
2. saves and queries the incomplete revision;
3. obtains bounded repair context containing the expected type, required visible values, and legal
   addition constructor;
4. attempts to put a `Decision.accept` value into the `i64` placeholder and proves that the typed
   rejection publishes nothing and consumes no persistent identity;
5. fills the placeholder with the required addition without changing its Node ID, owner, body
   position, output, or incoming uses;
6. queries a semantic diff that reports an operation refinement rather than delete/create churn;
7. runs accepted Linux and WebAssembly jobs plus CPU, memory, target, and trust rejection outcomes;
   low-fuel runs with large scoring inputs prove unsupported-target and untrusted-release paths do
   not evaluate unselected accepted work;
8. renames `Resources.memory` to `Resources.memory_units` without changing field identity, layout,
   references, or behavior;
9. restarts the service, checks all three saved revisions and selected identities, preserves the old
   display name in old revisions, and repeats old and current runtime oracles.

Public input and output values identify every named type, field, and variant by persistent Node ID.
The driver fails if any response, revision, identity, type, member, payload, or result differs from
its oracle.

## Run it

From the repository root:

```sh
./examples/job-policy/run.sh
```

The script builds optimized release binaries, creates one private mode-0700 temporary state
directory, uses typed shutdown, restarts against that same private state, and removes only the state
it created. It requires a current stable Rust toolchain, a POSIX shell, and Python 3 from the standard
library; it uses no third-party Python package.
