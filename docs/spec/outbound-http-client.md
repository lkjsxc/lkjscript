# Deployment-bound outbound HTTP client

Status: normative.

This specification owns the exact-endpoint outbound HTTP/1.1 capability, destination admission,
TLS trust, request/response collection, cancellation, and cleanup. It does not own application
routing or response policy, inbound HTTP, WebSocket, Nostr event semantics, deployment selection,
or accepted graph publication.

## Semantic authority

The built-in standard package exposes exactly one public `HttpClient` interface with one task
operation, `get`. Its sole argument is an ordered bounded list of structural headers
`{name: Text, value: Bytes}`. Its result is one structural value
`{status: I64, headers: List<{name: Text, value: Bytes}>, body: Bytes}` preserving received header
order and whole bounded body bytes. The operation is idempotent with possible external visibility.
That classification permits no automatic replay and does not assert that a remote implementation
is side-effect-free.

Graph meaning receives no scheme, host, port, path, URL, proxy, trust switch, root-secret name,
credential, redirect policy, timeout, or retry count. One exact component requirement admits the
operation, and one exact deployment grant binds it to one `http_client` adapter. There is no
ambient URL, DNS, socket, or generic host-call intrinsic. Runtime results and failures cannot
change semantic `HEAD` or local operational data.

## Endpoint and recipe normalization

The adapter endpoint is immutable for a prepared deployment. Its canonical form recognizes only
lowercase `http` and `https`, one bounded canonical ASCII DNS name or canonical IP literal, an
optional canonical nonzero port, and a path. It rejects user information, query, fragment,
backslashes, noncanonical escapes, whitespace/control bytes, ambiguous authority, malformed port,
and unsupported or differently cased schemes before readiness. A public destination requires
`https`; `http` is valid only with `loopback_only` and a lexical loopback host.

The closed `nostr-relay-info` recipe accepts an exact lowercase `wss`, `https`, `ws`, or `http`
locator. It normalizes `wss` to `https` and `ws` to `http`, preserving authority, explicit port,
and path. Plaintext `ws`/`http` is admitted only for a lexical loopback destination and selects
`loopback_only`; other locators select `public_only`. This conversion is deployment construction,
not WebSocket implementation.

## Address admission and SSRF policy

Address policy is exactly `public_only` or `loopback_only`; there is no private-LAN mode. DNS is
performed for the exact endpoint at request time through the operating system resolver and is
bounded independently by result count and the total request deadline. Literal addresses bypass
name resolution but undergo the same class check. IPv4-mapped IPv6 is normalized to IPv4 before
classification.

The entire resolved set is validated before connection. `loopback_only` accepts only loopback
addresses. `public_only` conservatively accepts global unicast addresses and rejects loopback,
private, link-local, unspecified, multicast, carrier-grade NAT, documentation, benchmarking,
protocol-assignment, transition, and other non-global or reserved ranges. Any forbidden member
rejects the whole set. The client connects only to the validated socket addresses, in canonical
order, without a hidden second resolution. Alternate address attempts share one connection
deadline and are establishment attempts, not request replay.

The adapter does not read proxy environment variables, proxy configuration, cookies, credential
stores, or resolver configuration beyond the system DNS/network service needed for the selected
endpoint. This policy reduces destination authority; it is not DNSSEC, a privacy proxy, or a
general SSRF sandbox.

## TLS trust

HTTPS uses pure-Rust TLS with HTTP/1.1 application protocol behavior. It verifies the canonical
endpoint hostname, certificate chain, and certificate validity. Trust is exactly one of:

- `webpki_roots`, the executable's locked public WebPKI roots; or
- `named_pem_root`, one bounded deployment secret containing 1 through 16 PEM certificates.

The named secret is loaded only during preparation and never appears in output, evidence, graph
meaning, or an artifact. A missing, empty, malformed, excessive, expired, hostname-mismatched, or
untrusted chain fails closed. There is no platform-native TLS backend, insecure-skip switch,
certificate pinning framework, client certificate, trust union, secret fallback, or TLS trust
selected by graph code. Plaintext loopback development does not consume trust material.

## HTTP request and response

Each operation sends one HTTP/1.1 `GET` to the exact canonical path with no request body. The
adapter owns `Host`, connection framing, and `Accept-Encoding: identity`. It rejects graph-supplied
`Host`, `Content-Length`, `Transfer-Encoding`, `Connection`, `Upgrade`, authorization,
proxy-authorization, cookie, and other transport- or credential-owned headers. Remaining header
names and values are validated and emitted in graph order.

The client does not follow redirects, retry a request, decompress a body, persist cookies, use a
proxy, inject credentials, negotiate WebSocket, or reuse the response as a stream. Every response,
including non-2xx, is a transport result after strict HTTP/1.1 status/header/framing validation.
Content-length, chunked, and connection-close framing are bounded and mutually unambiguous;
informational responses, malformed framing, prohibited transfer encodings, conflicting lengths,
trailing bytes, and partial EOF fail without returning a partial success.

Request header count/bytes, response header count/bytes, response body bytes, DNS results,
concurrent requests, connection time, total time, and cleanup time are separate positive
deployment limits. Their global maxima and exact descriptor fields are executable-generated in
`docs/generated/deployment.md`. Connection time may not exceed total time. The operation allocates
one live request resource only after admission and releases it on every result.

## Cancellation, failure, and shutdown

The effective deadline is the earlier of the owning task deadline and the deployment total
deadline. Resolution, all address attempts, TLS, write, and response collection share that bound;
connection establishment additionally uses its narrower limit. Task cancellation, inbound-client
disconnect, resident shutdown, deadline expiry, protocol error, and resource exhaustion close the
connection and return no partial response. A write failure after bytes may have left the process is
reported with possible external visibility. There is no automatic replay after any failure.

Resolution, destination, connect, TLS, timeout, cancellation, protocol, and limit failures use
stable redacted diagnostics. They contain no remote body, certificate bytes, secret, provider
detail, alternate address inventory, or credential. Repeated adapter shutdown is idempotent, stops
new request admission, waits only the configured cleanup bound, and reports an infrastructure
failure if owned resources remain. Readiness requires successful static endpoint, policy, limits,
and trust preparation but performs no DNS lookup or network request.

## Nostr relay-information application policy

The closed recipe exposes inbound `GET /relay-info`. Its graph sends exactly
`Accept: application/nostr+json`, calls its sole `HttpClient.get` requirement, and returns the exact
bounded remote body only when status is 200 and at least one response `Content-Type` has the
case-insensitive base media type `application/nostr+json` with valid optional parameters. Transport
failure, another status, or another media type becomes deterministic local status 502 with body
`bad gateway`; it never reflects a remote body or diagnostic. Other routes return 404. The graph
stores nothing and treats the NIP-11 document as bounded bytes because fields may be absent or
unknown.

This slice is not a browser, cache, proxy, Nostr protocol implementation, relay subscription,
event model, event signer, key store, privacy layer, DNSSEC validator, sandbox, or multi-tenant
network isolation boundary. It provides neither WebSocket/NIP-01 nor inbound TLS.
