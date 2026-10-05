# Native worklists over independent owned storage

These literal inputs author three ordinary native packages. The generic library is
checked and exported before its concrete implementations exist. `Worklist<Item:
Owned>` relates the exact item type to the storage Self through consuming push and
an owned empty/item pop result. Explicit witnesses select every operation.

`library.lkjc` owns generic construction, cross-representation movement, draining
and reusable empty owners. `carriers.lkjc` supplies flat sequences and nested
32-item chunks independently for OwnedI64Cell and ByteBuffer. Ordinary graph
functions implement the chunk algorithms; no worklist-specific intrinsic or
external semantic generator participates. The cell contains the signed I64 value;
the buffer contains one octet, so the combined demonstration accepts `0..255`.

Consult [status](../../docs/status.md) for tested source and public availability.
These inputs and expected outputs are acceptance witnesses until their public tests
have passed; they do not establish compiler self-hosting or production adoption.

## Independent complete result

The `owned-worklists` target takes one list. Arguments `[[2,3,7]]` return:

```json
{
  "cell-flat":{"length":3,"moved":[2,3,7],"empty-length":0,"reused":[17]},
  "cell-chunked":{"length":3,"moved":[2,3,7],"empty-length":0,"reused":[17]},
  "buffer-flat":{"length":3,"moved":[2,3,7],"empty-length":0,"reused":[17]},
  "buffer-chunked":{"length":3,"moved":[2,3,7],"empty-length":0,"reused":[17]}
}
```

The flat paths build flat storage and move every element into chunked storage.
The chunked paths build chunked storage and move into flat storage. Each movement
reverses the source stack; consuming destination pops restore the input order.
Both empty source and destination owners survive the move. The destination is
drained, observed empty, reused for 17 and drained again. Each child moves between
custodians without cloning its payload.

## Author and detach through the product

Copy the executable and literal inputs outside the checkout. Create `library` and
`carriers` with `new --template minimal`, and `application` with `new --template
command`. Each proposal begins with `request base=BASE`, where `status` supplies
the exact current revision.

Export the builtin standard using `package builtin export --kind transport
--output standard.lkjp`; observe its package, semantic revision, package revision
and transport digest. Stage it in each minimal package using `package dependency
stage --transport DIGEST --input-file standard.lkjp`, then add its exact
`add.dependency` request. Both library and carriers use `(use std builtin)`.

Plan and apply `library.lkjc`, run `check`, and export with `package current export
--kind transport --output library.lkjp`. Stage that transport in carriers and
application. Prefix `carriers.lkjc` with the exact observed dependency and import:

```text
add.dependency package=LIBRARY_ID semantic-revision=LIBRARY_SEMANTIC package-revision=LIBRARY_PACKAGE_REVISION
declarations.begin
(units (use owned-worklists LIBRARY_ID LIBRARY_PACKAGE_REVISION))
declarations.end
```

Plan, apply, check and export carriers. Stage its transport in application. Prefix
`application.lkjc` with both exact dependencies and the imports `owned-worklists`
and `worklist-carriers`. Plan with `change plan --input-file FILE`, apply its exact
token with `change apply --input-file FILE --plan TOKEN`, and check. An untouched
canonical draft of each module must plan as `unchanged`.

Build the application through `build --output owned-worklists.lkja`. The supplied
`owned-worklists.deployment.json` requires no grants, configuration or secrets.
Write `[[2,3,7]]` to `arguments.json` and run:

```sh
./lkjscript run --deployment owned-worklists.deployment.json \
  --arguments-file arguments.json --result-file result.json
```

The public witness removes all authoring projects and exported transports before
repeating detached execution. Its independent expected results include empty,
singleton, chunk-boundary and 513-item inputs, and it checks joined cleanup with
zero live owned handles. Invalid witness/type/ownership proposals must preserve
accepted HEAD.

## Individual representation targets

The same artifact also exposes `cell-flat`, `cell-chunked`, `buffer-flat` and
`buffer-chunked`. These targets instantiate one generic build, length, drain and
reuse algorithm. They retain a single storage representation throughout the
invocation, so per-invocation observations can be compared on matched behavior.
Arguments `[[2,3,7]]` return the same complete result from every target:

```json
{"drained":[7,3,2],"empty-length":0,"length":3,"reused":[17]}
```

The supplied descriptors named after each target select `owned-worklists.lkja`
with empty grants. For example:

```sh
./lkjscript run --deployment cell-flat.deployment.json \
  --arguments-file arguments.json --result-file cell-flat-result.json
./lkjscript run --deployment cell-chunked.deployment.json \
  --arguments-file arguments.json --result-file cell-chunked-result.json
```

Use absent result paths for every invocation. Compare cell and buffer routes using
the same inputs in `0..255`; cell-only routes also admit full signed I64 values.
Public tests independently check all four targets for 513 inputs, both attached
and after removal of source projects and transports.

Detached execution reports preparation, invocation and result-encoding nanoseconds
separately. Its `production-observation` includes instructions, modeled cumulative
allocated bytes and allocation charges, with cleanup counters checked afterward.
Those counters do not measure allocator traffic, retained owned bytes or OS RSS.
Repeat fresh-process measurements with alternating representation order and retain
all complete outputs before making comparative performance claims.

## Provisional graph tooling

Author `reachability.lkjc` in the application with the same imports, either instead
of or alongside `application.lkjc`. Its `provisional-reachability-flat` and
`provisional-reachability-chunked` targets use the same generic native pass with
independent cell worklists. Each queued cell owns the actual candidate node ID.
The accepted meaning graph remains under ordinary admission and publication.

One command argument is a typed record of roots and authored nodes:

```json
[{"roots":[10],"nodes":[
  {"id":10,"successors":[20,30]},
  {"id":20,"successors":[40]},
  {"id":30,"successors":[40]},
  {"id":40,"successors":[20]},
  {"id":50,"successors":[50]},
  {"id":60,"successors":[]}
]}]
```

Both implementations return exactly:

```json
{"case":"valid","value":{"reachable":[10,20,30,40],"unreachable":[50,60]}}
```

Build this application with `build --output provisional-graph.lkja`, then use the
supplied `provisional-reachability-flat.deployment.json` or
`provisional-reachability-chunked.deployment.json` with the same argument file and
fresh result paths. Both descriptors are grant-free and select the same artifact.

IDs are arbitrary signed I64 identities. Repeated roots/edges, cycles and self-edges
are admitted; visited marking at enqueue schedules each node once. Output lists
retain authored node order. Empty roots make every node unreachable.

The tool admits 4,096 nodes, 4,096 roots and 16,384 successor entries, inclusive.
Capacity results use `{"case":"capacity","value":"nodes|roots|edges"}` with
one exact dimension name, in that precedence. Complete validation precedes
traversal and includes unreachable nodes. Duplicate IDs take precedence, followed
by missing roots and missing successors in their authored order. Invalid outcomes
carry `{"code":TEXT,"owner":I64,"target":I64}`: `duplicate-node` reports the
duplicate ID twice, `missing-root` reports owner -1 and its target, and
`missing-successor` reports the source ID and missing target. Signed ID -1 remains
valid; the diagnostic code distinguishes the missing-root sentinel.

These are native candidate-analysis outcomes. Wrong JSON/types reject at the
ordinary runner boundary; the tool's `invalid` outcome is ordinary result data,
not a kernel publication rejection. The tool neither reads nor edits the accepted
meaning graph. Program-visible graph adaptation and an actual Rust compiler-pass
replacement remain later work with their own authority and independent admission.
