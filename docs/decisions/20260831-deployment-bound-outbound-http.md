# Deployment-bound exact-endpoint outbound HTTP

Status: accepted on 2026-08-31.

## Decision

The first outbound network capability is one built-in `HttpClient.get` task operation. Graph
meaning supplies only an ordered bounded header list and consumes one structural whole response.
One strict deployment `http_client` grant owns one immutable canonical endpoint, one closed address
class, one TLS trust mode, and independent resource limits.

Public destinations require HTTPS. Plaintext is confined to explicit loopback-only development.
Resolution occurs per request, the complete bounded result set must satisfy `public_only` or
`loopback_only`, and connections use only those validated addresses. HTTPS verifies hostname,
chain, and validity against either the executable's locked WebPKI roots or one bounded named PEM
root secret. Requests are HTTP/1.1 GET without body, redirect following, decompression, cookies,
proxy, credential injection, or automatic retry. Cancellation and shutdown close owned resources.

The maintained proving consumer is the closed `nostr-relay-info` recipe and its independent local
raw HTTP/TLS relay oracle. `wss` is normalized to the NIP-11 `https` document endpoint; this does
not admit WebSocket or NIP-01.

## Rationale

Allowing graph code to choose a URL, trust toggle, secret, or retry count would turn a narrow
deployment grant into ambient network authority and make SSRF review depend on runtime data.
Validating every DNS answer before connecting prevents a mixed answer from silently crossing the
selected address boundary and avoids hidden re-resolution after policy checks. Exact endpoint and
trust ownership also make readiness, diagnostics, copied-binary proof, and later capability review
finite and discoverable.

GET is classified idempotent to describe graph policy, but remote visibility remains possible and
the adapter never replays it. This keeps retry policy explicit and avoids claiming behavior about
an arbitrary remote implementation.

## Rejected alternatives

- A graph-supplied generic URL or socket intrinsic: destination authority would be ambient.
- Redirect following: a response could select a destination outside the admitted endpoint.
- Platform trust, insecure-skip, or trust unions: behavior would depend on hidden host state or
  permit authentication bypass.
- First-answer-only DNS admission: mixed or reordered answers would weaken address policy.
- A private-LAN mode: no maintained consumer in this slice justifies that authority.
- Automatic retry: it would replay an externally observable request.
- A Nostr client dependency or WebSocket implementation: neither is required to transport NIP-11
  relay information.

## Reversal conditions

Broader methods, request bodies, streaming, redirect policy, private-network destinations, client
certificates, proxies, WebSocket, or per-request destination selection require a separately named
maintained workload, exact semantic and deployment authority, new independent protocol/security
oracles, resource and cancellation contracts, and dependency-closed migration. A measured need may
replace the connection mechanism, resolver implementation, or TLS library only if exact endpoint,
address-set validation, trust, no-hidden-replay, diagnostics, and copied-binary behavior remain
independently proved.
