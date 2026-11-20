# Exact outbound TCP port authority

An owned TcpPort grants outbound TCP connection authority to one immutable,
nonzero port. TcpPort.connector(address) accepts a run-time numeric IPv4
address and derives the existing exact-destination Connector. That connector
retains ordinary connect and deadline-aware connect_deadline behavior.

A browser can receive separate http, https and dns parameters with grants
http=80, https=443 and dns=53, while receiving no World. The DNS grant
permits an ordinary source DNS-over-TCP client to choose a numeric server on
port 53; it does not add implicit name resolution. These values authorize
multiple hosts only on their own ports. They confer no listening or arbitrary
port authority, and do not widen an existing exact Connector.

World.tcp_port(port: u16) derives the same restriction for a full-authority
parent, returning errno 22 for port zero. Entry grants reject zero, overflow,
negative numbers, lists, addresses, environment selectors, missing values and
duplicates. A port grant does not open a socket. The capability is affine and
cannot be forged from a name, integer or cast, copied, or treated as Data.

## Complete checked stack

The implementation extends the checked type and intrinsic contracts, entry
validation, authority report, native lowering, independent interpreter, binary
and text IR, source queries, generators and reducers. Canonical unit interfaces
retain the exact capability kind and port authority. There is no new native
runtime feature or child packet kind: unsupported child transfer has a located
refusal and publishes neither an image nor an admission record.

The [boundary proof](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/tcp-port-data/boundary.log) grants only TcpPort, Arguments,
Clock and Stdout. It selects its destination from run-time argument bytes and
connects to an independent C listener. Native, interpreter and an independent
Rust host bridge all receive the same peer data. Its checked authority report
contains no World, listening operation, or destination-widening capability.

Cold and warm separately checked unit images use a network helper across an
authenticated interface. They execute the same exchange and are byte-identical.
Malformed grants refuse before execution or output-image publication.

## Validation and retained identities

Native SHA-256:

    38f8494e1a58fddbff8dc0af0950ea5056c80ec42cdf91d9793c569d17cb03ed

Seed gzip SHA-256:

    ec4e761be1e69b3cd5d590d0b71c15ac31b3ec4446da6d2700dafcda47074a94

The native bootstrap reproduces itself exactly. Nine focused cases pass on each
engine, including ownership and independent checked-IR refusals. The separate
Rust witness agrees on one output case and three verdicts. The complete Unix
message regression also passes with this producer, including its descriptor
cleanup and confined-path replacement controls.

The complete four-worker [gate](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/tcp-port-data/full-gate.log) passes in 875.318
seconds, with all 6,206 case requests rerun and none reused. The separate
[real-program proof](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/tcp-port-data/standalone-real-programs.log) passes the
original Levenshtein algorithm, eight tests per engine, eighteen provenance
controls and all 7,239 native corpus rows. Producer and seed identities remain
unchanged after the gate. Exact-head publication gate/witness CI is recorded
with the session report.

All twelve unchanged-source census observations match the prior frontend
frontier byte for byte: nom 17, DOM 63, and Sōzu 448 classified plus 34
prior-unclassified rows. This runtime capability does not establish large-root
or nom-test admission.

An initial optional Rust invocation used an unqualified compiler name outside
the witness's deliberately restricted PATH. That harness setup failure is
retained separately; the successful proof selects the exact absolute rustc
path. No language change was needed for that correction.
