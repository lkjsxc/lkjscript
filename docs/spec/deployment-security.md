# Deployment and security capabilities

This specification owns deployment binding for configuration, secrets, clocks, randomness,
identifiers, password hashing, adapters, listener/topology limits, and redacted inspection. It does
not own application authentication/authorization policy, semantic identity, or operator secrets.

## Descriptor and preparation

A deployment descriptor is strict JSON, at most 1 MiB and 1,024 grants. It has no public version
discriminator and names a
relative component artifact, exact target, optional listener,
resident/execution/HTTP/session/worker/stream limits, typed configuration map, secret environment
bindings, and grants. Outbound-client limits
are owned by each exact `http_client` adapter rather than folded into the inbound HTTP limit record.
Relative paths reject
absolute, backslash, empty, `.` and `..` components. Artifact paths and local object roots reject
symbolic-link components; artifacts must be regular files and local roots must be existing real
directories. The deployment directory is a trusted operator boundary.

All JSON objects require unique decoded keys, including the configuration map.
Repeated identical values are still ambiguous and reject; an escaped spelling of
the same key is not distinct. There is no first-wins or last-wins policy. Admission
uses the shared bounded strict JSON reader and retains the full signed/unsigned
64-bit range required by configuration and execution limits. Duplicate keys fail
before artifact access, secret reads, adapter creation or snapshot publication.
Previously accepted duplicate-key descriptors require an explicit operator choice
of one value, not automatic rewriting. Valid descriptor encodings are unchanged.

Preparation bounds and decodes the descriptor and artifact bundle, validates the bundle digest
and exact root target/component/runner/requirement closure, then loads named secrets, constructs
and preflights adapters, computes redacted descriptor digests, and only then permits readiness or
listener/worker admission. A failure closes already-created adapters and emits no ready event or
application work. Artifacts contain none of these deployment facts.

Foreground `run --deployment` uses this same descriptor and loader. Only this route permits
omitting `execution` and `runtime`; explicit null rejects. The raw schema preserves omission
until route validation. Resident routes require both policy objects, with positive live limits
and explicit numeric or null cumulative quota choices. Command topology
requires null listener, HTTP, session and worker fields. Discovery marks these four
topology fields `required=true`: accepting null does not make a JSON field omittable.
Other field requirements are unchanged.
Exact Command runner, full component requirements, grant/interface mapping, bounded arguments
and every result-type encoding branch are admitted read-only before named secrets or live
adapters. Partial adapter preparation closes earlier owners on failure.

Absent execution selects trusted foreground work without cumulative instruction, allocated-byte,
collection-item or invocation capability-call quotas. Explicit execution has six fields:

| Field | Presence | Accepted value | Omitted-field behavior |
| --- | --- | --- | --- |
| `instruction_fuel` | Required | Positive u64 or null | Reject |
| `maximum_call_depth` | Required | Positive usize | Reject |
| `maximum_value_stack` | Required | Positive usize | Reject |
| `maximum_allocated_bytes` | Optional | Positive u64 or null | 268,435,456 bytes |
| `maximum_collection_items` | Optional | Positive u64 or null | 1,000,000 items |
| `maximum_capability_calls` | Optional | Positive u64 or null | 100,000 calls |

Each null disables only that cumulative quota. Numeric zero, negative, fractional,
excessive, duplicate and unknown fields reject before artifact/secret/adapter access.
Omission of the three added fields preserves the actual limits of older numeric
three-field descriptors. Encoding materializes those selections, so an encode/decode
round trip does not remove a previously implicit quota. Null instruction fuel alone
therefore does not remove allocation, collection or capability-call quotas.

The counters are per invocation, not a process-lifetime budget. Unbounded counters
remain observed and saturate at u64::MAX, where the value is a lower bound rather than
an exact total. Allocated bytes and collection items are cumulative work accounting,
not an RSS or live-heap ceiling. Depth, stack, admission/codec bounds, task concurrency,
body/stream limits and exact per-grant authority remain independently enforced.

New HTTP recipes explicitly set all four cumulative quotas to null. Existing authored
descriptors keep their previous numeric selections; no configuration file is implicitly
rewritten. Internal discovery contract 5 describes the expanded choice. Older executables
reject null fuel or the new fields; selecting this syntax requires a supporting executable,
while rollback retains the earlier numeric descriptor and its matching executable.
No graph, artifact, package or application-data migration is required.

Absent runtime removes only this route's invocation deadline; shutdown and cancellation grace
remain finite. Explicit runtime preserves its supplied deadline and cleanup policy. Neither
policy supplies capabilities, alters per-grant maxima, removes finite preparation/codec or
representation admission, nor creates a hostile-code sandbox. Older executables reject newly
omitted fields: retain matching executables and immutable bundles for recovery. No graph, type,
artifact or operational-data encoding change or migration follows from this syntax evolution.

An interactive descriptor must provide the complete session limit record and no HTTP or worker
record. Preparation reconstructs the canonical relational port, checks each positive limit against
its global ceiling, checks cross-field stream/frame/message/mailbox/transition and lifetime
relations with overflow-safe arithmetic, and reserves process-wide accounting before readiness.
The independently bounded dimensions include pending and active sessions, header/frame/message
bytes, mailbox items and aggregate bytes, retained-state nodes and bytes, messages and bytes per
transition, tick interval, idle and total lifetime, close/cancellation grace, and process-wide
session-buffer bytes. The separately supplied stream and resident/execution records continue to
bound callback streams, work, deadlines, and cleanup.

`capabilities --section deployment` returns the public descriptor schema; runtime readiness returns the
domain-tagged artifact-bundle digest,
target, runner, listener, typed configuration observation, secret names, and adapter kinds. Secret
bytes, data/object authority internals, password inputs, and live handles are omitted. Startup failure
publishes no application work.

## Configuration and secrets

Configuration values are closed bool, i64, or text with at most 4,096 fields and 1 MiB per
value. Applications may request only accepted-source `StaticText` names through typed exists/bool/
i64/text operations; wrong/missing type is capability failure. The current boundary has one descriptor source
and no ambient merge, watch, or mutable precedence.

Secret bindings map a canonical application-independent name to a canonical environment variable.
Each value is limited to 64 KiB, stored behind a redacted `SecretValue`, and never converted to
ordinary bytes/text or artifact/history/diagnostic/evidence. Adapters receive only the exact named
secret they require. `SecretVerifier` exposes bounded constant-time equality rather than the secret.
Rotation requires a new prepared deployment/restart.

## Time, randomness, identifiers, and passwords

Wall clock exposes UTC Unix milliseconds as explicit capability output. Operational deadlines use a
separate runtime monotonic clock and are not serializable semantics. Tests bind deterministic clock
values.

Secure randomness exposes at most 1 MiB OS-generated bytes per operation, further limited by the
component grant. Deterministic test randomness is a distinct adapter. UUID generation returns
canonical lowercase hyphenated RFC 4122 version-4 spelling; parsing/formatting rejects noncanonical
input. Application entity domains and collision response remain application policy.

Password hashing accepts 1 through 1,024 bytes and an exact bounded Argon2id deployment policy for
memory, iterations, lanes, salt, and output. Operations hash with OS randomness, verify without
exposing the derived secret, and report whether encoded parameters need upgrade. Mismatch is a
normal false result; malformed/unsupported/excessive encodings and infrastructure failure remain
distinct. Session expiry, revocation, token scope, CSRF, actor roles, and authorization are library
or application policy, not this adapter.

## Adapter lifecycle and trust

Current descriptor adapters are configuration, wall clock, secure randomness, UUID identifier,
password hash, secret verifier, byte stream, exact-endpoint `http_client`, first-party `data`,
memory/local/S3 object, and first-party `durable_queue_data`. An HTTP client grant binds one
canonical immutable endpoint, `public_only` or `loopback_only` address policy, locked WebPKI roots
or one named PEM-root secret, and separate request-header, response-header/body, DNS, concurrency,
connection, total, and cleanup limits. Preparation validates endpoint, policy, limits, and trust
material without network I/O; request-time resolution and connection remain live effects governed
by [outbound-http-client.md](outbound-http-client.md). A data grant binds a confined relative root, strict namespace,
sharing domain, authority revision, and independent limits; it has no connection secret, host,
port, TLS, pool, or network timeout. Service and worker may use separate grants that resolve to the
same root. Authority revision and descriptor digest identify the exact operational binding. None
becomes program semantic identity.

Adapters acquire live resources during preparation or calls, retain them only within deployment or
task scope, reject use after close, and receive one recorded idempotent shutdown after admission stops. A
restart re-reads descriptor/artifact/secrets and reconstructs data/object/queue authority; streams,
task IDs, random state, locks, and compiled code are disposable.

The trust and denial model is normative in `docs/security.md`. First-party Rust forbids `unsafe`,
but dependencies and the operator/OS are trusted. The HTTP/RFC 6455 listener is plaintext; inbound TLS
termination, certificate management, and ACME are deliberately out of scope. Outbound HTTPS
authenticates the exact endpoint under its selected closed trust mode but is not a privacy layer,
browser trust UI, pinning system, or client-certificate facility. The local data root is neither
encrypted nor a tenant-isolation boundary. This boundary also does not claim credential rotation
without restart, a hostile-code sandbox, tenant resource isolation, provenance, artifact
signatures, replication, consensus, or distributed atomicity.
