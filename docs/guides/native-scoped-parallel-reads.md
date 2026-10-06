# Read one owner through joined task groups

A graph task can synchronously borrow an owned input and return ordinary data.
With a Shareable type obligation, a joined parallel group can lend that same owner
to both children, including recursive child groups. The parent regains consumption
rights after the complete group joins. [Status](../status.md) owns executable
availability; [the parallel specification](../spec/structured-parallel.md) owns
admission, custody and cleanup.

## Declare the required read rights

Owned task parameters explicitly select `borrow` or `consume`. A synchronous
borrow needs only Owned, including an exact task method whose Self input borrows.
Callable kind and exact effects remain part of admission. Graph-function calls
retain their final capability-resource suffix. A borrowed task result remains unsupported.

Borrowing across fresh joined child invocations needs a stronger assumption:

```lisp
(type-parameter create W (constraint owned shareable))
(parameter create storage (type W) (use borrow))
```

Use `(constraint owned transferable shareable)` when the algorithm also moves
owners across a child boundary. Transferable and Shareable are independent:
`owned transferable` does not prove shared-read safety. Concrete cells, buffers,
products, choices and sequences prove the relevant property structurally, including
inactive cases and unused type arguments.

Both child calls remain direct named graph tasks with closed empty effect rows.
Their ordinary parameters precede owned inputs. Evaluate left arguments, then
right arguments, once in the parent. Preparation retains both loan footprints
until join. Borrowing the same root twice is valid; borrowing one alias while
consuming that root or an ancestor rejects in either order. Every started child
finishes cleanup before the source guard or owner can end.

## Compose the generic library before concrete items

The literal [library](../../examples/scoped-parallel-reads/library.lkjc) adds module
`scoped-parallel-reads` to the existing storage library. Its `range-sum` task takes
an indexed half-open range and borrowed `W`, with `T: owned shareable` and
`W: owned shareable`. Exact `Element<T>` and `IndexRead<W,T>` prerequisites provide
observation and source-tied selection. Both low-level `range-sum` and
`serial-range-sum` callers supply `0 <= start <= end <= length`; the full-storage
sum tasks obtain length through their exact reader and supply that valid range.

Ranges of at most 32 elements use `serial-range-sum`, an indexed loop task with an
ordinary I64 accumulator and borrowed storage. Each iteration uses `IndexRead.at`
inside `borrow-call`, observes the selected `T` view through its exact Element
witness, and ends that view before advancing. An empty range returns the accumulator.
A larger range divides at its midpoint, starts two recursive range-sum children
borrowing the same storage, joins them, and adds their ordinary results. The cutoff
is authored library behavior; index intervals introduce no slice or cursor type.

`sum` obtains the storage length and reads the full range. `parallel-sums` starts
two sum tasks with different explicit Element witnesses: ordinary observation and
modulo-four keys. Reader and element identities remain exact through every nested
group. Equal Self types do not merge witnesses.
`serial-sum` reads the full range through the same serial loop without forming a group.

The `Sum` contract declares `sum(Self borrow) -> I64` as a task. Its generic
`RecursiveSum<T,W>` scheme maps that method to the task above with explicit Element
and IndexRead prerequisites. The contract's Self remains Owned; the scheme retains
the stronger Shareable obligations needed by its selected recursive implementation.
An ordinary synchronous borrowed task method does not itself introduce a Shareable
boundary requirement.

## Author three packages through the product

Copy a compatible executable and the literal proposals to a disposable directory
outside the compiler checkout. Follow the exact staging and export operations in
the [native library guide](native-library.md). Preserve all export fields: `id`,
`revision`, `package-revision` and `transport`.

Create a minimal library project, stage the exact builtin standard transport, add
its dependency, and author these proposals in order:

1. [Worklist algorithms](../../examples/owned-worklists/library.lkjc).
2. [IndexRead selectors](../../examples/owned-read-results/library.lkjc).
3. [Generic storage](../../examples/generic-owned-implementations/storage.lkjc).
4. [Scoped parallel reads](../../examples/scoped-parallel-reads/library.lkjc).

Each request begins with `request base=BASE` using the current project `status`.
Plan its literal input, apply the same input with the exact returned plan token,
and check the project. Export the completed library before authoring concrete items.

Create a minimal item project, stage the standard and library transports, and add
both exact dependencies. Import `owned-worklists` from the library's exact package
and package revision. Author the independent
[element proposal](../../examples/generic-owned-implementations/elements.lkjc),
check and export it. It supplies cells, one-octet buffers and a nested product;
it does not specialize the recursive read algorithm.

Create a command consumer project, stage both completed transports and add both
exact dependencies. Import `owned-worklists`, `owned-read-results`, `generic-storage`
and `scoped-parallel-reads` from the library locator, and `generic-elements` from
the item locator. Author the literal
[application](../../examples/scoped-parallel-reads/application.lkjc), then check and
build `scoped-parallel-reads.lkja`.

Use `change draft` to reconstruct accepted declarations and require unchanged
re-entry. Stronger constraints, task kind, parameter modes and method maps must
survive package export and canonical drafts.

## Observe the join, then drain and reuse

The `scoped-parallel-reads` command accepts one I64 list in the shared `0..255`
cell/buffer/product domain. It builds flat and chunked storage for each item type.
Each scenario obtains the nested parallel sums and an exact borrowed Sum method
result, then drains the original storage, observes its empty length, pushes 17
and drains again. The product stores additional tag 5 and stamp 7, so ordinary
observation and finishing add 12 to each input.

For `[[2,7,11,3]]`, each scenario has length 4 and empty-length 0:

| Item in flat or chunked storage | sum-left | sum-right | method-sum | drained | reused |
| --- | --- | --- | --- | --- | --- |
| Cell or buffer | 23 | 11 | 23 | [3,11,7,2] | [17] |
| Nested product | 71 | 11 | 71 | [15,23,19,14] | [29] |

`sum-right` observes each element modulo four; it is not the left sum modulo four.
Empty input produces zero sums and an empty drained list, then the same reuse
result. Individual `scoped-read-cell-flat`, `scoped-read-cell-chunked`,
`scoped-read-buffer-flat`, `scoped-read-buffer-chunked`, `scoped-read-product-flat`
and `scoped-read-product-chunked` targets select one scenario.

The matched `scoped-read-cell-flat-serial` target selects serial-sum for both
observers and the equivalent method result. It uses the same cells, reader and
Element witnesses, flat storage, drain and reuse steps, and returns the same
complete Scenario. Its executed path has zero parallel scopes, even though the
artifact also contains the parallel tasks. `scoped-read-cell-flat` executes one
outer observer group for inputs of at most 32 elements; 33 and 513 elements also
exercise nested range groups.

Run with the supplied
[grant-free descriptor](../../examples/scoped-parallel-reads/scoped-parallel-reads.deployment.json);
an empty task effect row still requires task execution. Copy the executable, artifact, descriptor and
literal arguments to a fresh runtime directory and repeat execution there.
Authoring projects and staged transports are unnecessary at execution time.

Compare the
[parallel cell-flat](../../examples/scoped-parallel-reads/scoped-read-cell-flat.deployment.json)
and [serial cell-flat](../../examples/scoped-parallel-reads/scoped-read-cell-flat-serial.deployment.json)
descriptors
with identical literal inputs in fresh processes. Retain complete outputs alongside
preparation, invocation and result-encoding durations and runtime observations.
Keep all matching witnesses and storage behavior fixed. These measurements describe
the selected workload and environment; the example makes no speedup claim.

Independent checks should cover empty and singleton input, 31/32/33-item chunk
boundaries, the complete outputs, exact witness selection and owner reuse after
the join. Weakened Shareable bounds, wrong task kind, read/consume aliases and
borrowed task results must reject without changing accepted HEAD. Cancellation,
traps and quota refusal must join started children before releasing their source
loans. These are acceptance obligations; a designed example alone does not prove
overlapping execution, cleanup or published availability.
