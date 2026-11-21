# Native cases

Run a case or directory with the complete compiler:

```sh
target/lkjscript test tests/conformance --jobs 8
target/lkjscript test tests/reference
```

The compiler discovers source and IR files recursively. Each case declares its
expectation in comments beside its program. Source comments use `//`;
IR comments use `#` after the mandatory `lkjscript-ir 1` first line.

| Comment | Meaning |
| --- | --- |
| `expect: value 42` | Scalar return text, with its trailing newline ignored |
| `expect: output 00ff0a` | Exact stdout bytes; an omitted hex value means empty |
| `expect: reject E_BORROW` | The checker must reject with this diagnostic code |
| `expect: trap R0003 12` | Runtime trap code and optional source line |
| `expect: check` | Accept without executing |
| `expect: compare` | Compare native and interpreter outcomes |
| `stdout: HEX`, `stderr: HEX` | Exact bytes; default empty |
| `status: 7` | Normal exit status; default zero |
| `entry: NAME` | Select an entry instead of main/probe |
| `reference: PATH` | Compare with an independent source expansion, on both routes |
| `pending: REASON` | An intended rule blocked only by E_PARSE/E_UNSUPPORTED |
| `witness: JSON` | Optional Rust comparison metadata |

Reference paths are relative to the command's working directory. Reference
files have no expectation header and are executed alongside the originating
case. Comparisons preserve exit status, stdout, stderr, and termination kind;
same-source engine pairs also agree on trap line. Diagnostic locations can differ
between independently written programs.
An explicit trap expectation checks its line when supplied. R0004 also includes
artifact-local panic message codes R0016 and above.

Workers are native compiler processes. They share each case's checked IR between
routes, round-trip serialized IR through the independent checker, and compare
source/IR executable bytes when the invocation supplies every required grant.
Check-only cases never create grants. Each execution has fresh owned state. Passing suites
print one line; failures identify the case and route. The shell gate applies a
generous suite watchdog and accounts for all workers within LKJSCRIPT_GATE_JOBS
(2..8). The HTTP suite reserves two workers for its server/client pair; individual
`lkjscript test --jobs 1` runs remain available.

Only borrowed_closure_escape remains pending. The former claim of three scoped-task
cases was stale; concurrency now uses isolated processes, without shared-address-space
thread rules. Unrelated rejection, wrong results and malformed worker output fail.

The mandatory gate bootstraps from the seed without Rust or Node and uses Node
for host-side fixture checks and real-program tooling. `bash witness/check.sh`
optionally builds the Rust witness, compares complete compiler bytes, and checks
every marked case with overflow checks enabled. Its native test coordinator
keeps program stderr and Rust trap/status records separate.

Use `// witness: {"root":true}` for a self-contained source case that needs
Rust crate-root resolution, such as a macro using `$crate`. Rust compiles the
case at crate root with a witness adapter appended. A value case defaults to
`probe`. An output or trap case uses a named `World` entry selected with
`// entry: NAME`. Reserve `main` for the adapter in runnable root cases. A reject
case uses the same `rustAccept`, `rustCode` and `world` metadata as other rejects.

`tools/world-test.lkj` checks binary arguments and streams, directory confinement,
closed and broken descriptors, process ownership, Listener descriptor promotion,
and source/IR identity. Listener conformance cases cover IPv4 parsing, raw socket
address bounds, failed-acquisition cleanup, affine ownership, and drop/rebind.
`tools/resource-test.sh` keeps one heap and one stack refusal case and verifies
startup/reservation reuse under explicit test limits.
`tools/http-test.sh --seconds 0` retains all functional HTTP requests in the gate:
large bodies/headers, binary bytes, GET/HEAD, framing and Directory confinement.
Its clients are native lkjscript. It starts a fresh local server with stdin
closed, profiles that server, and closes every child before returning. The full
hour load and focused interpreter route run use the same supervisor separately.
