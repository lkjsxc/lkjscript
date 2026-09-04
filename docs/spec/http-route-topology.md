# Signature-indexed inbound HTTP route topology

This specification defines the only supported route-selection model for an `http` target.

## Authority and identity

Accepted graph meaning owns a finite set of HTTP route owners. Each route has one stable durable
identity, belongs to one exact root-package HTTP target, and stores one method, one function-backed
port, and exactly one typed selector:

- `exact` stores one literal path; or
- `pattern` stores canonical literal and whole-segment capture values.

Changing the method, selector, or port preserves route identity. Deployment may select listener
coordinates, grants, secrets, and resource limits, but cannot add, remove, replace, prioritize, or
reorder routes. Display strings, matchers, capture vectors, and observations are derived.

An HTTP target names one component and no universal port. It owns 1 through 4,096 routes whose
aggregate method-plus-selector bytes do not exceed 4 MiB. It owns at most 65,536 stored pattern
segments. Every route port belongs to that component and is function-backed. Non-HTTP targets retain
exactly one port and own no HTTP routes.

Canonical route order is unsigned method bytes followed by selector kind and canonical selector
bytes. The canonical route-set digest binds the ordered stable route, method, selector kind,
canonical literal/capture segments, capture order, and exact port identity. It does not replace
stable route identity.

## Selectors, keys, and bounds

A method is a nonempty HTTP token containing at most 32 ASCII bytes. Every exact path contains 1
through 16,384 UTF-8 bytes, begins with `/`, and contains neither `?`, `#`, nor control bytes. Exact
paths retain their byte spelling and may contain braces, empty internal segments, or a trailing
slash.

A public pattern contains 1 through 16,384 UTF-8 bytes and begins with `/`. After the leading slash
it has 1 through 64 slash-separated, nonempty segments and no trailing slash. Each segment is either:

- one literal containing no `{`, `}`, `?`, `#`, control byte, or slash; or
- one whole capture `{name}`, where `name` is a valid graph `Name`.

A pattern contains 1 through 32 captures, and capture names are unique within it. Empty, mixed,
optional, regex, wildcard, and tail segments are invalid. Public parsing immediately constructs the
typed segment sequence; the template spelling is not stored as a second authority. All counts and
byte totals use checked arithmetic before allocation or accepted publication.

## Overlap, specificity, and matching

Methods are exact. Different pattern segment counts are disjoint. Two equal-length patterns overlap
when every position containing literals in both patterns uses the same literal bytes. An overlapping
selector `A` is strictly more specific than `B` when every literal position in `B` is the same
literal in `A` and `A` replaces at least one capture position with a literal. Exact selectors are
maximally specific and win before pattern selection.

Overlapping patterns are admitted only when one is strictly more specific. Identical match
languages, including capture-name-only variants, and incomparable overlaps reject. Authored order,
insertion order, source order, owner identity, hash iteration, enum order, and deployment data never
break ties.

Transport validation precedes selection. An exact selector compares the adapter's validated path
spelling byte-for-byte. A pattern capture matches exactly one nonempty segment. Matching performs no
percent or Unicode decoding, normalization, case folding, slash collapse, dot resolution,
trailing-slash equivalence, or query selection. A capture value is ordinary unrestricted `Text`
containing the exact matched segment spelling.

Compilation derives a deterministic matcher per target. Preparation materializes bounded exact
indexes and a segment trie whose node count is bounded by the target's stored route segments plus
one root per method. A request performs an exact point lookup, then literal-before-capture trie
traversal bounded by the prepared node count; it neither allocates a route-set candidate list nor
linearly scans the route table. The selected route is the unique most-specific match. Capture values
are constructed in route order only after selection.

## Signature-indexed captures

An exact route requires a function-backed port with type
`(HttpRequest) -> HttpResponse`. A pattern containing `{c1}` through `{cN}` requires a port with type
`(HttpRequest, Text, ..., Text) -> HttpResponse` and a backing function with exactly those `N`
trailing parameters. The trailing parameters are named `c1` through `cN` in left-to-right route
order, have type `Text`, use unrestricted ownership, and have no resource requirement. The request
parameter remains first. A pattern route cannot bind missing, extra, reordered, renamed, non-Text,
borrowed, consumed, or resource-bound capture parameters.

Graph validation, package validation, compiler lowering, compiler-unit validation, artifact loading,
and runtime preparation reconstruct the route-to-target-to-port-to-function relation and verify the
function type, parameter identities, names, order, types, use modes, HTTP result, and component
requirement closure. Pattern routes may share a port only when their capture-name sequences and
therefore derived handler contracts agree. No capture map, ambient router context, thread-local
state, host callback, or route-private type system exists.

Invocation receives the request first and capture values in route order. It occurs at most once, and
live effects are never replayed for comparison.

## Authoring, inspection, and derived execution

`create.target` forbids `port` for runner `http` and requires it for every other supported runner.
The only route mutations are:

```text
add.http-route as=$ROUTE target=TARGET method=METHOD path=EXACT_PATH port=PORT
add.http-route as=$ROUTE target=TARGET method=METHOD pattern=PATH_PATTERN port=PORT
set.http-route route=HTTP_ROUTE method=METHOD path=EXACT_PATH port=PORT
set.http-route route=HTTP_ROUTE method=METHOD pattern=PATH_PATTERN port=PORT
```

Each mutation requires exactly one of `path` or `pattern`. Creation accepts request-local target and
port symbols. `delete.owner` removes a route under normal exact-base and dependency rules. Planning
and applying use the same parsing, normalization, allocation, overlap analysis, signature checking,
impact, and publication path. Apply rederives repository-dependent facts under the publication lock;
stale, altered, malformed, ambiguous, foreign, exhausted, or signature-drifting work advances
nothing.

Route inspection exposes route identity, target, method, selector kind, canonical segments and
captures, component, port, backing function, and derived signature. Target inspection exposes its
component, exact and pattern counts, route-set digest, and maximum specificity chain. Bounded context
traversal follows ordinary route ownership, target/component, route/port, port/function,
function/parameter, and effect relations; there is no second route projection or editable index.

Compiler units carry canonical selectors and capture-parameter identities. Strict artifact loading
and runtime preparation independently bind every matcher leaf to its route owner, target, component,
port, backing function, capture names, parameter owners, HTTP types, and requirements. In-memory and
live dispatch use the same prepared matcher. Zero, excess, malformed, duplicate-language,
incomparable, foreign, drifting, wrong-component, wrong-shape, non-function, route-on-non-HTTP,
HTTP-with-port, and non-HTTP-without-port forms reject at every owning boundary. Predecessor graph,
authored-request, package, compiler-unit, artifact, and adapter encodings are not read or migrated.

## Failure, effects, and resources

Existing malformed or excessive transport requests fail before matching. An unmatched valid pair
returns status 404, no application headers, and an empty body. It invokes no graph function or
capability and creates no resident task. The transport still drains or closes the request body
through bounded ownership.

A match constructs every capture before one invocation. Capture construction, overload,
cancellation, disconnect, request-body, handler, response, and shutdown failures advance no semantic
authority and release matcher, stream, task, and permit resources. Prepared matcher nodes, match
steps, capture counts and bytes, task/permit peaks, and cleanup remain finite observations rather
than semantic authority.

## Scope and evolution

Handlers continue to own authentication, authorization, request and domain validation, data,
object and queue transitions, and response construction. Route captures infer no identifier,
integer, UUID, alias, filesystem, or media semantics.

This model does not add host or scheme routing, query routing, middleware, implicit methods,
automatic `OPTIONS`, reverse routing, aliases, optional/regex/wildcard/tail captures, static files,
multipart, templates, response streaming changes, inbound TLS, or HTTP/2 or HTTP/3. Any such
extension requires a maintained workload, bounded graph-native semantics, dependency-closed
migration and deletion, and an implementation-disjoint oracle. A universal fallback port is not an
extension or reversal path.
