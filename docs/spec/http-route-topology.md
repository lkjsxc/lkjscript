# Exact inbound HTTP route topology

This specification defines graph-owned exact HTTP route topology. It is the only supported route
selection model for an `http` target.

## Authority and identity

Accepted graph meaning owns a finite set of HTTP route owners. Each route has one stable durable
identity, belongs to one exact root-package HTTP target, and stores one method, path, and exact port
reference. Changing any of those three stored values preserves the route identity. Deployment may
select listener coordinates, grants, secrets, and resource limits, but cannot add, remove, replace,
or reorder routes.

An HTTP target names one component and no universal port. It owns 1 through 4,096 routes whose
aggregate method-plus-path bytes do not exceed 4 MiB. Every route port belongs to that component,
is function-backed, and has the exact current HTTP request-to-response function type. Non-HTTP
targets retain exactly one port and own no HTTP routes.

The canonical route order is unsigned method bytes followed by unsigned path bytes. Duplicate
method/path pairs reject even when they select the same port. The canonical route-set digest binds
the ordered stable route, method, path, and exact port identities; it does not replace stable route
identity.

## Keys and exact matching

A route method is a nonempty HTTP token containing at most 32 ASCII bytes. A route path contains 1
through 16,384 bytes, begins with `/`, and contains neither `?` nor `#`. Route and target bounds are
checked with non-wrapping arithmetic before accepted publication, compilation, artifact loading,
or listener readiness.

Transport validation precedes exactly one route lookup. Method and path equality are byte-exact and
case-sensitive. The lookup uses the adapter's validated path spelling without percent-decoding,
Unicode normalization, case folding, slash collapsing, dot-segment resolution, trailing-slash
equivalence, wildcard matching, or parameter extraction. Query text and decoded query parameters
remain handler inputs but never select a route; query decoding therefore follows a successful
lookup. `HEAD` selects only an exact `HEAD` route; a matched
HEAD response retains transport body suppression and is never mapped to `GET`.

An unmatched valid pair returns status 404, no application headers, and an empty body. It invokes
no graph function or capability and creates no resident task. The transport still drains or closes
the request body through its bounded ownership. A selected handler runs at most once and visible
effects are never replayed.

## Authoring, inspection, and derived execution

`create.target` forbids `port` for runner `http` and requires it for every other supported runner.
The only route mutations are:

```text
add.http-route as=$ROUTE target=TARGET method=METHOD path=PATH port=PORT
set.http-route route=HTTP_ROUTE method=METHOD path=PATH port=PORT
```

Creation accepts request-local target and port symbols. `delete.owner` removes a route under the
normal exact-base and dependency rules. Planning and applying use the same normalization,
validation, impact, and publication path; stale, altered, invalid, or exhausted work advances
nothing.

Exact route inspection exposes route identity, target, method, path, component, and port. Exact
target inspection exposes its component, route count, and canonical route-set digest. Bounded
context traversal follows route ownership, target/component, route/port, port/function, and effect
dependencies without introducing a second route projection or editable authority.

Compilation derives one canonical route table for each HTTP target. Strict artifact loading and
runtime preparation independently bind every row to its route owner, target, component, port,
function implementation, and HTTP type. In-memory and live dispatch use the same prepared table.
Zero, excess, malformed, duplicate, foreign, drifting, wrong-component, wrong-shape, non-function,
route-on-non-HTTP, HTTP-with-port, and non-HTTP-without-port forms reject at every owning boundary.
Predecessor graph, authored-request, package, compiler-unit, and artifact encodings are not read or
migrated.

## Scope and evolution

Handlers continue to own authentication, authorization, request and domain validation, data,
object and queue transitions, and response construction. Exact routing does not add wildcard or
parameterized paths, precedence, hosts, middleware, aliases, implicit methods, automatic OPTIONS,
static files, multipart, templates, response streaming, TLS, or HTTP/2 or HTTP/3 semantics.

Parameterized paths or middleware require a later maintained workload, explicit deterministic
precedence and effect semantics, finite authoring and resource bounds, dependency-closed migration,
and an implementation-disjoint oracle. They must not be introduced by weakening exact matching or
restoring a universal fallback port.
