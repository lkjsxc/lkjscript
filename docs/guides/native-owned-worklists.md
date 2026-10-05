# Compose parameterized owned worklists

A parameterized owned contract can describe a storage owner and its element owner
independently. Its methods may return structural owned products and choices. The
same generic build, transfer and drain functions can then use different storage
representations without inspecting their layouts.

[Status](../status.md) owns exact source acceptance and public binary availability.
Use the selected executable's `capabilities --section change` to discover its
surface. The [semantic contract](../spec/owned-contract-parameters.md) defines
parameter scope, structural substitution and exact witness matching.

## Contract and storage witnesses

The maintained [example](../../examples/owned-worklists/README.md) separates a
[generic library](../../examples/owned-worklists/library.lkjc),
[concrete carriers](../../examples/owned-worklists/carriers.lkjc) and
[consumer](../../examples/owned-worklists/application.lkjc). The library is checked
and exported before concrete implementations are authored. `Worklist<Item: Owned>`
retains its distinguished `Self: Owned` and describes these operations:

| Method | Inputs in order | Result |
| --- | --- | --- |
| `empty` | None | Self |
| `length` | Self borrow | I64 |
| `push` | Item consume, Self consume | Self |
| `pop` | Self consume | Owned choice: `empty Self` or `item {rest: Self, value: Item}` |

The `item` payload is an owned product. Both pop outcomes preserve an explicit
storage owner; the caller exhaustively matches the choice and completely unpacks
the item product before using either binding. An empty result can be reused.

Declare Self and Item as separate Owned contract parameters. The `types` vector
supplies only Item, because Self has its own operand. A function with Owned
parameters W and T binds its storage witness as:

```lisp
(implementation-parameter implparam_77000000000000000000000000000001
  work Worklist W (types T))
```

Additional parameter order follows the contract's declaration order with Self
removed. Witness selection compares the exact nominal contract, Self and every
ordered argument. Matching Self alone is insufficient. A generic forwarding call
retains the complete application in its exact enclosing function scope.

The concrete storage choices are:

| Representation | Closed Self type | Invariant |
| --- | --- | --- |
| Flat | `(owned-sequence Item)` | One sequence holds every element. |
| Chunked | Owned product containing `(owned-sequence (owned-sequence Item))` and I64 length metadata | Every full chunk contains 32 elements; the last chunk may be shorter. |

Each has closed implementations for OwnedI64Cell and ByteBuffer. Concrete method
targets are monomorphic graph functions with the exact contract signatures after
substitution. The generic algorithms use only Worklist methods and exact element
witnesses. They do not inspect storage fields or choose an implementation by name.

[Generic implementation schemes](native-generic-owned-implementations.md) provide
the reusable successor: one Flat<T> or Chunked<T> declaration explicitly maps the
generic functions and supports later independent items without storage wrappers.

Push consumes Item before Self in authored argument order. Both pure read methods
and consuming methods retain the existing affine and synchronous-loan rules.
Internal chunk moves transfer owners; they do not copy element storage. Vector
growth and allocation costs still require measurement.

## Build, move, drain and reuse

The generic `build` creates one element per ordinary I64 input. `move-all` consumes
a source storage owner and transfers its elements into another storage owner; its
source and destination Self types are independent. `drain` finishes every element
in LIFO order and returns ordinary values with the reusable empty owner.

With the example dependencies imported, construct flat cell storage explicitly:

```lisp
(implementation-call owned-worklists::build
  (types OwnedI64Cell (owned-sequence OwnedI64Cell))
  (implementations concrete@worklist-carriers::Scalar
    concrete@worklist-carriers::FlatCells)
  (local inputs))
```

The type argument order is element T followed by storage W. The witness order is
Element followed by Worklist. Bind the returned owner to an exact typed local
before borrowing or consuming it. `move-all` uses type arguments T, source W,
destination V and corresponding source/destination Worklist witnesses.

The `owned-worklists` command uses both transfer directions. Cell and buffer flat
paths build flat storage and transfer into chunked storage before draining;
chunked paths build chunked storage and transfer into flat storage. Moving and
then draining reverses twice, preserving the original input order in `moved`.

Its one argument is an ordinary list of octet-range I64 values. An argument file
containing `[[2,3,7]]` yields:

```json
{
  "cell-flat": {"length": 3, "moved": [2,3,7], "empty-length": 0, "reused": [17]},
  "cell-chunked": {"length": 3, "moved": [2,3,7], "empty-length": 0, "reused": [17]},
  "buffer-flat": {"length": 3, "moved": [2,3,7], "empty-length": 0, "reused": [17]},
  "buffer-chunked": {"length": 3, "moved": [2,3,7], "empty-length": 0, "reused": [17]}
}
```

ByteBuffer creation constrains this combined example to `0..255`; the cell carrier
itself retains signed I64. The example includes empty inputs, chunk boundaries and
larger runtime inputs. Compare complete outputs before comparing performance.

## Compare one representation at a time

Four additional targets run one generic build, length, drain and empty-owner reuse
algorithm independently. Each target creates one element per input, drains in LIFO
order, observes the empty owner, pushes 17 and drains again. For `[[2,3,7]]`, all
four return:

```json
{"drained":[7,3,2],"empty-length":0,"length":3,"reused":[17]}
```

Use the corresponding descriptor beside the same `owned-worklists.lkja` artifact:

| Target | Grant-free descriptor |
| --- | --- |
| `cell-flat` | [cell-flat.deployment.json](../../examples/owned-worklists/cell-flat.deployment.json) |
| `cell-chunked` | [cell-chunked.deployment.json](../../examples/owned-worklists/cell-chunked.deployment.json) |
| `buffer-flat` | [buffer-flat.deployment.json](../../examples/owned-worklists/buffer-flat.deployment.json) |
| `buffer-chunked` | [buffer-chunked.deployment.json](../../examples/owned-worklists/buffer-chunked.deployment.json) |

Use identical octet-range inputs when comparing representations and carriers. The
cell targets also accept the carrier's full signed-I64 range. Detached `run`
reports preparation, invocation and result-encoding nanoseconds separately, plus
modeled cumulative allocation and work counters. These counters do not measure
allocator traffic, retained owned bytes or process RSS. Public tests compare the
complete independently expected output of every target for 513 inputs.
The [matched measurements](../performance.md#parameterized-owned-worklists-2026-10-05)
retain stage costs and unfavorable chunked results. Flat storage is the practical
baseline for those tested workloads.

## Provisional dependency-graph analysis

The [native tool](../../examples/owned-worklists/reachability.lkjc) uses the same
Worklist abstraction to schedule graph identities held in owned scalar cells.
Targets `provisional-reachability-flat` and `provisional-reachability-chunked`
accept the same ordinary input and return the same typed result.

The argument is one graph record, wrapped in the command argument array:

```json
[{
  "roots": [10],
  "nodes": [
    {"id": 10, "successors": [20,30]},
    {"id": 20, "successors": [40]},
    {"id": 30, "successors": [40]},
    {"id": 40, "successors": [20]},
    {"id": 50, "successors": [50]},
    {"id": 60, "successors": []}
  ]
}]
```

Both targets return:

```json
{"case":"valid","value":{"reachable":[10,20,30,40],"unreachable":[50,60]}}
```

Reachability does not depend on queue order: both output lists retain original
node order. Every discovered identity is scheduled at most once. Cycles,
self-edges and repeated roots or successor references are valid.

The tool validates all nodes and references before traversal, including unreachable
nodes. It diagnoses duplicate identities first, then missing roots, then missing
successors, each in authored order. Invalid input returns an `invalid` case with
ordinary `{code, owner, target}` diagnostic data. A missing root uses code
`missing-root` and owner `-1`; identities remain signed I64, so the diagnostic code
determines that field's role.
A duplicate reports its identity in both owner and target. A missing successor
reports its source node identity as owner and the missing referenced identity as
target.

Application bounds are 4,096 nodes, 4,096 roots and 16,384 total successor entries.
Repeated entries count toward capacity. Exceeding a bound returns a `capacity`
case whose text payload is `nodes`, `roots` or `edges`. This typed result is
separate from invalid graph structure and from runtime cancellation, traps or
storage refusal. A runtime failure does not become a graph outcome.
Capacity checks precede graph validation in nodes, roots, edges order. Incorrect
JSON or argument types reject at the ordinary runner boundary rather than returning
an `invalid` graph result.

This is a native tooling witness over candidate data. Its success does not replace
the production compiler's admission or publication boundary. Compiler integration
needs an explicit host boundary and independent validation of that use.

## Author and execute

Each `.lkjc` file contains literal native declarations. Stage the builtin standard
transport for ordinary list and arithmetic helpers, then add the observed exact
dependency locator. Prepend `request base=REVISION` from current project `status`,
plan the request, apply the returned review token and run `check`.

Export the library first with `package current export --kind transport`. Stage it
in the carriers project using the `owned-worklists` alias; check and export those
carriers. Stage both exact dependencies in the consumer project with aliases
`owned-worklists` and `worklist-carriers`. Author the application and reachability
modules there. The [library guide](native-library.md) gives the exact project,
export and staging operations; the example README owns its complete invocation
sequence and deployment descriptor.

Build `owned-worklists.lkja` beside
`owned-worklists.deployment.json`, then use `run --deployment` with argument and
result files. The public acceptance workflow executes through a copied product
outside the checkout and repeats execution after deleting authoring projects and
staged transports. Retain literal input files and independent expected results.
The representation descriptors select the same bundle. Build
`provisional-graph.lkja` for the graph targets and use the supplied
[flat](../../examples/owned-worklists/provisional-reachability-flat.deployment.json)
or [chunked](../../examples/owned-worklists/provisional-reachability-chunked.deployment.json)
descriptor with one graph argument and a fresh result path.

Use `change draft` to reconstruct canonical declarations. Owner inspection and
function-definition detail retain Self, ordered contract arguments and method
signatures. Planning an unchanged draft must produce an unchanged proposal.

Associated types, generic implementation schemes and borrowed results tied to a
storage lifetime are outside this contract. Traps, cancellation and reservation
refusal dispose of remaining storage and loans under existing cleanup rules.
No user-defined Worklist method runs implicitly during cleanup.
