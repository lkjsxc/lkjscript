# Inspect owned children, then consume the original

This example accepts three native packages through the public executable. The
`owned-borrows` library is checked and exported before concrete carriers exist.
Its `T: Owned` functions borrow a direct product child, inspect nested products,
reborrow an inspected parent, hold two sibling read views, and exhaustively read
a choice containing an ordinary I64 or an owned T. A pure borrowed helper and
exact `Storage` implementation witness carry the same rights across packages.

`carriers.lkjc` supplies an OwnedI64Cell and a ByteBuffer. Scalar reads the cell's
stored value; Alternate reads 99 from the same Self type. Octets reads buffer
length. `application.lkjc` selects these witnesses explicitly, reads each
container, and then consumes its original children. The buffer remains `[73]`
and freezes to `{"$bytes":"SQ=="}`. Reads neither replace a child nor authorize
its consumption inside the read scope.

For arguments `[42,false]`, the complete result is:

```json
{
  "scalar": {"first":42,"second":42,"tag":7},
  "alternate": {"first":99,"second":99,"tag":7},
  "consumed": 42,
  "bytes": {"first":1,"second":1,"tag":8},
  "frozen": {"$bytes":"SQ=="},
  "nested": {"child":42,"parent":{"first":42,"second":42,"tag":9},"marker":17},
  "nested-consumed": 42,
  "choice": {"first":42,"second":99,"consumed":42},
  "siblings": {"left":42,"right":42},
  "sibling-consumed": {"left":42,"right":42},
  "marker": 41
}
```

For `[42,true]`, the choice carries the ordinary value 42 and `choice.second`
also becomes 42. The other fields stay the same. Input values at both I64 limits
require no arithmetic or narrowing.

## Author through the product

Copy the public executable and these files into an owned directory outside the
checkout. Create the packages:

```sh
./lkjscript new library --template minimal --name owned-borrows-library
./lkjscript new carriers --template minimal --name owned-borrow-carriers
./lkjscript new application --template command --name owned-borrows-application
```

For each proposal, prepend `request base=BASE`, taking BASE from that project's
current `status` revision. Plan `library.lkjc` with `change plan --input-file FILE`,
apply the returned review token with `change apply --input-file FILE --plan TOKEN`,
and run `check`. Export the accepted library using `package current export --kind
transport --output library.lkjp`.

The export reports package id, semantic revision, package revision and transport
digest. Stage it in carriers and application with `package dependency stage
--transport DIGEST --input-file library.lkjp`. Before `carriers.lkjc`, add the
following exact request and native import, using the observed export identities:

```text
add.dependency package=LIBRARY_ID semantic-revision=LIBRARY_SEMANTIC package-revision=LIBRARY_PACKAGE_REVISION
declarations.begin
(units (use owned-borrows LIBRARY_ID LIBRARY_PACKAGE_REVISION))
declarations.end
```

Plan, apply, check and export carriers. Stage its transport in application, then
prepend both exact dependency requests and native imports named `owned-borrows`
and `owned-borrow-carriers` to `application.lkjc`. Plan, apply and check application.
For each module, an untouched `change draft --module MODULE --output FILE` plans
as `unchanged`.

## Execute a detached artifact

Build with `--project application build --output owned-borrows.lkja`. Place the
artifact beside `owned-borrows.deployment.json`, which selects `owned-borrows`
without grants, secrets or configuration. Write `[42,false]` to `arguments.json`
and run:

```sh
./lkjscript run --deployment owned-borrows.deployment.json \
  --arguments-file arguments.json --result-file result.json
```

The maintained public test removes all authoring projects and both package
transports before repeating execution. It checks the complete independent result
for both choice arms and extreme I64 inputs, then verifies zero remaining owned
handles and successful cleanup. Separate cases reject root or view consumption,
escape, storage, incorrect field or arm annotations, incomplete or duplicated
choices, invalid untaken syntax and wrong Self witnesses without changing HEAD.
Canonical drafts retain all source, view, arm and witness identities through a
reviewed literal edit. Function extraction containing a read scope rejects.

A further copied-executable task fixture in
`tests/fixtures/owned-borrows-task.lkjc` performs an authorized clock call inside a
read scope and returns an unrelated owner from another scope. It consumes the
original after both scopes close; borrowed task parameters remain unsupported.
See the [native guide](../../docs/guides/native-owned-borrows.md) and
[semantic contract](../../docs/spec/owned-borrows.md) for the read rights.
