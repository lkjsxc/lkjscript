# Native diamond dependencies with independent borrowed readers

This workload has four independently authored packages: one generic supplier,
two concrete readers, and one consumer. It exercises shared immutable interface
records without sharing implementation choice, mutable owners or execution grants.

## Literal inputs and graph

Use the [recursive generic library](../../examples/demanded-callable-proof/library.lkjc)
as the supplier. It publishes Reader, Pair and read0 through read24 before a
concrete item or reader exists. Follow the ordinary public new/status/change
plan/change apply/check and package current export operations from the
[concrete reader guide](../../examples/concrete-callable-proof/README.md).

The [left reader](../../tests/fixtures/shared-interface-left.lkjc) and
[right reader](../../tests/fixtures/shared-interface-right.lkjc) each declare the
supplier as an exact dependency and import its compact-proof module. Stage its
complete transport and prepend the current exact dependency and module-use records
before authoring each literal input. Both private SelectedReader implementations
satisfy Reader for OwnedI64Cell, but their package identities and method bodies are
different. The left observes the cell; the right returns 99. Each public read
function invokes the supplier's complete read24 stack.

Export both reader packages. The
[consumer](../../tests/fixtures/shared-interface-consumer.lkjc) declares and stages
all three exact dependencies and imports compact-proof, left-reader and
right-reader. Thus the generic supplier is a direct dependency of the consumer
and of both readers. Its records are not authored three times or made editable by
three owners; only their derived immutable in-memory representation is shared.

The consumer allocates one cell, borrows it through each reader in order, then
extracts the preserved original value. Input `[17]` yields the full record:

```json
{"left":17,"right":99,"restored":17}
```

There are no deployment grants, external effects, inherited credentials, semantic
generators or host-computed result substitutions. Parameter order, loan lifetime
and exact private implementation choice remain ordinary language contracts.

## Maintained public acceptance

The source-selected
`native_owned_shared_dependency_interfaces_preserve_diamond_readers_after_source_deletion`
case authors and exports the four packages with copied executables outside the
compiler checkout. It rejects a wrong imported-call arity without moving HEAD,
checks canonical draft re-entry, builds a standalone artifact and compares full
results for I64 extrema, negative input, zero and positive input.

It then deletes only its four disposable authoring projects and three transports
and runs the original artifact again. The complete result is checked independently
at each input. This case belongs to the mandatory native_owned_ finalized-archive
family, not a separate manual optional check. A local passing run alone is not
finalized-archive acceptance or proof of publication.

The behavior is not a new syntax or a promise that the predecessor could not run
a diamond. The implementation change removes repeated immutable record copies.
The [decision](../decisions/shared-dependency-interfaces.md) describes exact ownership
and remaining type-copy costs; [status](../status.md) identifies actual acceptance.
