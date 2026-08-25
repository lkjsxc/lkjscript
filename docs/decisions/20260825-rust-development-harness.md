# Rust development harness

Date: 2026-08-25 UTC.

## Status

Accepted and implemented. Check and scale have fresh local acceptance evidence. The service
orchestration is implemented, but live Docker/PostgreSQL acceptance is unavailable on the
recorded host because the `docker` executable is absent.

## Decision

Contributor orchestration is owned by the non-published `lkjscript-dev` workspace package at
`tools/lkjscript-dev`. The released `lkjscript` package does not depend on it, and the harness
does not link private `lkjscript` library internals. It exercises Cargo commands and the released
CLI as child processes. This keeps contributor dependencies and orchestration out of the copied
application-development binary and makes scale and service checks prove the public boundary.

`process.rs` owns bounded child execution, independent stdout and stderr limits, timeouts,
status classification, synchronized logs, and child termination. Linux process-group control is
recorded as platform identity; portability of that control is not claimed. Environment evidence
contains approved variable names and derived identity, never raw secret values.

`evidence.rs` owns verification-domain digests, file proofs, and receipt publication. A receipt
is constructed from closed Rust types, encoded as operational JSON, written to a private sibling,
flushed and synchronized, renamed atomically, and followed by directory synchronization. A
success summary and receipt digest are printed only after publication succeeds. Referenced child
logs are bounded and synchronized first. These receipts are operational evidence, not semantic
authority.

The checker uses a validated typed DAG for the focused, changed, product, service, and full
profiles. It rejects duplicate gates, missing dependencies, cycles, duplicate commands, and
invalid paths. Gate cache keys bind the input snapshot, profile and command definition,
toolchain, executable identities, and required outputs. Cache records and restored logs are
verified under a lock; corruption cannot produce a pass. Active-run locks prevent evidence
rotation from removing another invocation.

The same package owns:

- `check`, including the predecessor profiles and gates, bounded parallel execution, causal
  skips, fresh-versus-reused evidence, and aggregate failure receipts;
- `scale`, including independent modules, distributed small functions, a wide module, a deep
  call chain, and wide fanout, generated and measured only through released CLI invocations; and
- `service`, including the owned PostgreSQL container lifecycle, migrations, service and worker
  readiness, HTTP acceptance, backup and restore, exact-name cleanup, redacted secrets, and an
  aggregate receipt for pass, failure, or unavailability.

The Python tools were deleted. `lkjscript-dev policy no-python` is a gate in every checker
profile and rejects first-party `.py` files and Python shebangs while excluding Git metadata,
build output, and retained evidence. It does not follow symlinks.

## Alternatives

- Retaining the three Python tools was rejected because it preserved a second contributor
  runtime and duplicated process, cache, and receipt policy.
- Translating each tool into a separate Rust binary was rejected because it would retain three
  orchestration foundations and three evidence contracts.
- Adding contributor commands to the released CLI was rejected because contributor process and
  Docker dependencies do not belong in the application-authoring binary.
- Linking private platform builders into scale or service was rejected because successful
  tooling must exercise the same released boundary available to external authors.
- Substantial shell orchestration was rejected because its status, bounds, redaction, and
  failure-evidence model would remain implicit.

## Evidence

| Evidence | Result |
|---|---|
| `e35623d8` | Added the workspace package and shared check, process, evidence, atomic-publication, and cache foundation. |
| `097bc145` | Replaced `tools/semantic-scale` with the Rust `scale` command. |
| `1da13c97` | Replaced `tools/service-acceptance` with the Rust `service` command. |
| `b033f0d5` | Deleted `tools/check`, completed checker parity, and added the Python-absence gate. |
| `.artifacts/lkjscript-dev/check/self-test-1787627725085257897-1067662-0/receipt.json` | Passed at `b033f0d5`; `verification_3aa071c52ea17b0c384af98e00d3c41f7006a276df108322151d4c3a43df74eb`. |
| `.artifacts/lkjscript-dev/check/1787627703368853691-1065682-0/receipt.json` | Fresh focused profile passed 6/6 at `b033f0d5`; `verification_adea479313a1b581c9f2ae8a581f3d0d8b12b93977d212b9087112bb79d09830`. |
| `.artifacts/lkjscript-dev/scale/1787627668828559346-1065111-0/receipt.json` | Three-item wide-fanout public-CLI run passed at `b033f0d5`; `verification_8f4fe5d77b93e3e362733c19f145a9b5213c7a74dfd017b4632fdc37c9b87a6e`. |
| `.artifacts/lkjscript-dev/service/1787627645153602169-1064760-0/receipt.json` | Correctly retained `unavailable` at `b033f0d5` because Docker was not found; `verification_e82db4bcdc4f6e716158fd22d55524d25f25f7ab0d42836f00ee20b1d51f645b`. |

The checker self-test covers child pass, failure, timeout, unavailable command, independent
stdout and stderr exhaustion, dependency skips, invalid DAGs, cache miss/hit/corruption, and
fail-closed atomic replacement. Unit tests additionally cover active-run retention and service
failure-receipt and cleanup behavior. The unavailable service receipt is evidence of honest
classification and durable failure evidence, not evidence that the live service workflow passed.

## Consequences and remaining work

Contributors need Rust and Cargo, while application authors still need only the released binary.
All maintained contributor orchestration now shares one bounded process and evidence model.
Operational receipt JSON remains justified as inspectable CI evidence and does not define graph,
repository, package, or artifact identity.

Live service and worker acceptance must be rerun on a host with Docker and the required image
before those workflows may be reported as passing under the Rust harness. Scale currently invokes
the released change representation that exists at its recorded commit; it must follow the direct
compact CLI cutover without acquiring a private builder.

## Reversal condition

Split a command into another contributor-only Rust package only if an acyclic dependency boundary
or independently maintained release cadence cannot be preserved in this package. Preserve one
shared bounded process and evidence contract unless measurements show that sharing itself causes
a material correctness or workflow cost. Do not restore Python or move contributor orchestration
into the released binary.
