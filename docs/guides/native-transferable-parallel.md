# Reusable generic parallel libraries

A generic task can form a joined group using its own type parameters when those
parameters explicitly promise transferability. Check the selected executable's
`capabilities` and [current availability](../status.md); older executables reject
these new constraints. The [parallel specification](../spec/structured-parallel.md)
owns the exact semantic contract.

`(constraint transferable)` describes ordinary transferable data and also satisfies
capture safety. `(constraint capture-safe transferable)` states both properties
explicitly. `(constraint owned transferable)` describes an affine owner that may
move into a child. An `owned` or `capture-safe` bound alone does not establish
transferability. Every nominal argument, field and alternative must be safe,
including phantom arguments and inactive cases. A callable, capability or borrowed
owner cannot gain permission merely by appearing inside an aggregate.

The following complete example uses three independently authored packages. Copy
the selected executable to `./lkjscript` in a disposable directory outside any
compiler checkout. No host-language program generator is needed. As in the
[native library guide](native-library.md), replace each `*_BASE` with that project's
observed `status` revision; apply each request using its own observed `plan_...`
token. Keep all four export fields: `id`, `revision`, `package-revision`, `transport`.
Names below identify modules; exact package identities select dependencies.

## Publish worker meaning

```sh
./lkjscript new ./workers --template minimal --name workers
./lkjscript --project ./workers status
```

Save `workers.lkjc`:

```text
request base=WORKERS_BASE
declarations.begin
(units (module create workers
  (function create keep (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (parameter create value (type T)) (returns T) (body (local value)))
  (function create move (visibility public) (effect (task))
    (type-parameter create T (constraint owned transferable))
    (parameter create value (type T) (use consume))
    (returns T) (body (local value)))))
declarations.end
```

```sh
./lkjscript --project ./workers change plan --input-file ./workers.lkjc
./lkjscript --project ./workers change apply --input-file ./workers.lkjc --plan WORKERS_PLAN
./lkjscript --project ./workers check
./lkjscript --project ./workers package current export --kind transport --output ./workers.lkjp
./lkjscript new ./groups --template minimal --name groups
./lkjscript --project ./groups package dependency stage --transport WORKERS_TRANSPORT --input-file ./workers.lkjp
./lkjscript --project ./groups status
```

Map the workers export to `WORKERS_PACKAGE`, `WORKERS_REVISION`,
`WORKERS_PACKAGE_REVISION` and `WORKERS_TRANSPORT`.

## Accept the generic builder before its callers

Save `groups.lkjc`:

```text
request base=GROUPS_BASE
add.dependency package=WORKERS_PACKAGE semantic-revision=WORKERS_REVISION package-revision=WORKERS_PACKAGE_REVISION
declarations.begin
(units (use workers WORKERS_PACKAGE WORKERS_PACKAGE_REVISION)
  (module create groups
    (function create ordinary (visibility public) (effect (task))
      (type-parameter create Left (constraint capture-safe transferable))
      (type-parameter create Right (constraint transferable))
      (parameter create left (type Left)) (parameter create right (type Right))
      (returns (record (left Left) (right Right)))
      (body (parallel
        (call workers::keep (types Left) (local left))
        (call workers::keep (types Right) (local right)))))
    (function create mixed (visibility public) (effect (task))
      (type-parameter create Owner (constraint owned transferable))
      (type-parameter create Data (constraint transferable))
      (parameter create data (type Data))
      (parameter create owner (type Owner) (use consume))
      (returns (owned-product (field left Owner) (field right Data)))
      (body (parallel
        (call workers::move (types Owner) (local owner))
        (call workers::keep (types Data) (local data)))))))
declarations.end
```

```sh
./lkjscript --project ./groups change plan --input-file ./groups.lkjc
./lkjscript --project ./groups change apply --input-file ./groups.lkjc --plan GROUPS_PLAN
./lkjscript --project ./groups check
./lkjscript --project ./groups package current export --kind transport --output ./groups.lkjp
```

This acceptance establishes the generic body without any concrete caller. An
ordinary pair is a structural record. If either result owns data, the pair is an
owned product and must be consumed or unpacked. Both mixed orientations and nested
products/choices obey the same rule. Declare ordinary parameters before consuming
owners so the builder can itself become a child of another group.
Weakening `Owner` to `owned`, or `Data` to
`capture-safe`, makes the generic proposal invalid before publication.

Map the groups export to the corresponding `GROUPS_*` placeholders. Create a
consumer and stage both exact transports:

```sh
./lkjscript new ./consumer --template minimal --name consumer
./lkjscript --project ./consumer package dependency stage --transport WORKERS_TRANSPORT --input-file ./workers.lkjp
./lkjscript --project ./consumer package dependency stage --transport GROUPS_TRANSPORT --input-file ./groups.lkjp
./lkjscript --project ./consumer status
```

## Instantiate and execute

Save `consumer.lkjc`:

```text
request base=CONSUMER_BASE
add.dependency package=WORKERS_PACKAGE semantic-revision=WORKERS_REVISION package-revision=WORKERS_PACKAGE_REVISION
add.dependency package=GROUPS_PACKAGE semantic-revision=GROUPS_REVISION package-revision=GROUPS_PACKAGE_REVISION
declarations.begin
(units (use groups GROUPS_PACKAGE GROUPS_PACKAGE_REVISION)
  (module create consumer
    (external create empty (visibility private) (implementation core.buffer.empty)
      (returns ByteBuffer))
    (external create push (visibility private) (implementation core.buffer.push)
      (parameter create n (type I64))
      (parameter create value (type ByteBuffer) (use consume)) (returns ByteBuffer))
    (external create freeze (visibility private) (implementation core.buffer.freeze)
      (parameter create value (type ByteBuffer) (use consume)) (returns Bytes))
    (type-alias Result (record
      (ordinary (record (left (list I64)) (right (list I64))))
      (bytes Bytes) (number I64)))
    (function create main (visibility public) (effect (task))
      (parameter create n (type I64)) (returns Result)
      (body (let
        (binding data (type ByteBuffer) (call empty))
        (binding data (type ByteBuffer) (call push (i64 0) (local data)))
        (binding data (type ByteBuffer) (call push (i64 255) (local data)))
        (binding data (type ByteBuffer) (call push (i64 128) (local data)))
        (binding pair (type (owned-product (field left ByteBuffer) (field right I64)))
          (call groups::mixed (types ByteBuffer I64) (local n) (local data)))
        (in (unpack-owned (type (owned-product (field left ByteBuffer) (field right I64)))
          (local pair)
          (field left (binding data (type ByteBuffer)))
          (field right (binding number (type I64)))
          (in (record structural
            (field ordinary (call groups::ordinary (types (list I64) (list I64))
              (list I64 (local n) (i64 42)) (list I64 (i64 -3))))
            (field bytes (call freeze (local data))) (field number (local number)))))))))
    (component create command (visibility private)
      (port create main (type (task-function (I64) Result (row))) (function main))))
  (target create transfer (component consumer::command)
    (runner command) (port consumer::command::main)))
declarations.end
```

```sh
./lkjscript --project ./consumer change plan --input-file ./consumer.lkjc
./lkjscript --project ./consumer change apply --input-file ./consumer.lkjc --plan CONSUMER_PLAN
./lkjscript --project ./consumer check
./lkjscript --project ./consumer build --output ./transfer.lkja
```

Save `transfer.deployment.json` and `arguments.json`:

```json
{"artifact":"transfer.lkja","target":"transfer","listen":null,"http":null,"session":null,"worker":null,"streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},"grants":[],"secrets":[],"configuration":{}}
```

```json
[-257]
```

```sh
./lkjscript run --deployment ./transfer.deployment.json --arguments-file ./arguments.json --result-file ./result.json
```

The complete result is:

```json
{"ordinary":{"left":[-257,42],"right":[-3]},"bytes":{"$bytes":"AP+A"},"number":-257}
```

Use the deployment runner for these tasks. An empty task effect row permits CPU
work; plain `run TARGET` selects the pure differential runner and rejects tasks.
Child calls are direct named empty-effect tasks. Ordinary argument expressions
run once in the parent, in authored order; consuming owner arguments name live
locals. Joined cleanup runs on success, failure and cancellation.

## Reentry, witnesses and detached use

`change draft --module groups --output groups-draft.lkjc` exports the accepted
builder. Planning that untouched draft returns `unchanged`. Edit the draft to
preserve graph identities when changing an accepted child. Rebuild consumers when
their exact dependency selection changes.

A generic group can also pass its lexical implementation parameters directly to
generic children. Use `parameter@FUNCTION@implparam_ID` in an
`implementation-call`; concrete consumers select `concrete@MODULE::IMPLEMENTATION`.
The selected implementation's exact identity is retained through forwarding, even
when two implementations have the same Self type. See the complete literal
[worker](../../tests/fixtures/parallel-transfer-workers.lkjc),
[generic combinator](../../tests/fixtures/parallel-transfer-combinator.lkjc) and
[consumer](../../tests/fixtures/parallel-transfer-consumer.lkjc) for that variant,
all four result orientations, nested choices and a group whose intermediate pair
is absent from the function's declared return.

Copy `lkjscript`, `transfer.lkja`, the deployment and arguments to a new runtime
directory, then run the same deployment there. The artifact carries the admitted
meaning and exact dependency closure. Authoring projects and package transports
are unnecessary at execution time. The [maintained public proof](../../tests/public_cli/native_parallel.rs)
executes both signed I64 extremes and both choice outcomes before and after removing
all three disposable source projects and both transports, and checks the complete
byte results and joined cleanup. It also checks rejected weakened bounds,
incompatible witnesses and duplicate ownership leave the accepted HEAD unchanged.
