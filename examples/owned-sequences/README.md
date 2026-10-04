# Build, inspect and drain runtime-sized owned collections

This example contains literal inputs for three native packages. `library.lkjc`
is checked and exported before the example's concrete carriers are authored. Its `Element` contract
supports creation from I64, pure borrowed observation and consuming completion.
Generic functions build collections from runtime lists, summarize indexed reads
and drain elements in LIFO order while retaining the reusable empty sequence.

`carriers.lkjc` independently supplies OwnedI64Cell and ByteBuffer implementations.
`Scalar` observes the stored cell; `Alternate` observes 99 from the same Self type.
Both finish by consuming the original cell. `Octets` stores each input as a single
octet and reads or finishes that octet. The combined application accepts inputs in
`0..255`; the underlying cell carrier also supports the full I64 range.

Consult [status](../../docs/status.md) for exact source acceptance and public
availability before selecting an executable.

## Independent expected result

The `owned-sequences` target takes one ordinary list. For argument file
`[[2,3,7]]`, its complete result is:

```json
{
  "scalar": {"first":12,"second":12,"popped":7,"drain":[11,3,2],"empty-length":0,"reused":[17]},
  "alternate": {"first":297,"second":297,"popped":7,"drain":[11,3,2],"empty-length":0,"reused":[17]},
  "bytes": {"first":12,"second":12,"popped":7,"drain":[11,3,2],"empty-length":0,"reused":[17]},
  "nested": 12,
  "marker": 41
}
```

Every path summarizes its original sequence twice, pops the last element, appends
11, drains the remaining values and reuses the empty owner for 17. The nested path
owns a sequence inside another sequence and summarizes its child through nested
loans. An empty input gives summaries zero, a popped sentinel of -1, draining
result `[11]` and reused result `[17]`.

## Author through the product

Copy the executable and files into an owned directory outside the checkout. Create
the packages:

```sh
./lkjscript new library --template minimal --name owned-sequences-library
./lkjscript new carriers --template minimal --name sequence-carriers
./lkjscript new application --template command --name owned-sequences-application
```

For each proposal, prepend `request base=BASE`, obtaining BASE from the project's
current `status`. The minimal library uses ordinary standard list and arithmetic
helpers: first export the builtin standard transport, stage it in library, and add
its observed exact dependency using the same export/stage/dependency operations
below. Plan `library.lkjc` with `change plan --input-file FILE`, apply
with `change apply --input-file FILE --plan TOKEN`, and run `check`. Export the
accepted library through `package current export --kind transport --output
library.lkjp`.

The export reports package id, semantic revision, package revision and transport
digest. Stage it in carriers and application using `package dependency stage
--transport DIGEST --input-file library.lkjp`. Prepend the following exact dependency
request and native import to `carriers.lkjc`, substituting observed identities:

```text
add.dependency package=LIBRARY_ID semantic-revision=LIBRARY_SEMANTIC package-revision=LIBRARY_PACKAGE_REVISION
declarations.begin
(units (use owned-sequences LIBRARY_ID LIBRARY_PACKAGE_REVISION))
declarations.end
```

Plan, apply, check and export carriers. Stage that transport in application.
Prepend both exact dependency requests and native imports with aliases
`owned-sequences` and `sequence-carriers` to `application.lkjc`; then plan, apply
and check. Untouched canonical drafts of each module must plan as `unchanged`.

## Detached execution

Build with `--project application build --output owned-sequences.lkja`. Keep the
artifact beside `owned-sequences.deployment.json`. This pure target requires no
grants, secrets or configuration. Write `[[2,3,7]]` to `arguments.json` and run:

```sh
./lkjscript run --deployment owned-sequences.deployment.json \
  --arguments-file arguments.json --result-file result.json
```

The public `native_owned_sequence_` suite checks independent full results for empty,
singleton, representative and large runtime inputs. It removes all authoring
projects and exported transports before repeating execution, and verifies zero live owned
handles and joined cleanup. Other cases reject invalid element types, unused bad
substitutions, wrong witnesses, consumption of views or their protected source,
capture, storage, unsupported data codecs and invalid untaken syntax without
changing HEAD. A reviewed marker literal edit preserves all canonical identities.
Invalid indices trap, and a subsequent valid invocation succeeds.

## Task consumer

In a separate command project with the same exact package imports, author
`tasks.lkjc` instead of `application.lkjc`. Its `owned-sequence-tasks` target takes
a nonempty octet list. A clock call in the index expression runs once, a borrowed
scope returns an unrelated new cell with value 77, and joined parallel workers
append 11 to transferred cell and buffer sequences. For `[[2,3,7]]`, it returns:

```json
{"read":2,"unrelated":77,"scalar":[11,7,3,2],"bytes":[11,7,3,2]}
```

Build its artifact and select this target in a deployment with one explicit
`clock` wall-clock grant. The public suite retains this descriptor, denies missing
grants before execution, and runs the accepted artifact after deleting sources.
Views cannot cross a task boundary; complete sequence owners can cross only when
their full closed element type satisfies the transfer contract.

The [native guide](../../docs/guides/native-owned-sequences.md) and
[semantic contract](../../docs/spec/owned-sequences.md) describe ownership,
borrowing, growth and all-exit cleanup. These inputs and expected results define
verification scenarios; they are not evidence of acceptance until their tests pass.
