# Complete map projection and traversal cost

`workload.lkjc` is an ordinary native program, not a host-generated graph. It builds
a persistent map from an ordered list of entries, performs repeated enumeration
or length reads, and returns all final ordered entries plus the accumulated length.
[The decision](../../docs/decisions/checked-map-entry-projection.md) owns the sealed
runtime boundary. [Status](../../docs/status.md) owns exact acceptance and delivery.

## Author through the copied product

Create a fresh minimal project, export and stage the selected executable's builtin
standard transport, and prefix the literal workload with the current
`request base=REVISION` and exact `add.dependency package=PACKAGE
semantic-revision=REVISION package-revision=PACKAGE_REVISION` binding. Apply only
through `change plan` and `change apply` with the returned plan token. The
[dependency-component guide](../dependency-components/README.md) describes this
public authoring protocol in detail.

Run `check`, then build `map-entry-projection.lkja` beside
`command.deployment.json`. The following detached invocation does not need the
source project or supplier transport:

```sh
./lkjscript run --deployment command.deployment.json \
  --arguments-file arguments.json --result-file result.json
```

Use an absent result path. Arguments have the shape
`[ENUMERATE, ENTRIES, REPETITIONS]`. For example:

```json
[true,[{"key":7,"value":{"items":[3,9],"marker":11}},
       {"key":-4,"value":{"items":[8],"marker":17}}],3]
```

The full result is:

```json
{"entries":[{"key":-4,"value":{"items":[8],"marker":17}},
            {"key":7,"value":{"items":[3,9],"marker":11}}],"total":6}
```

False selects header-only map length in the repeated part; true enumerates and
counts the entries. Both modes build the same map and return the same full final
projection. Later duplicate keys replace their values without moving key order.
The measured domain has nonnegative repetition counts; arithmetic retains ordinary
integer overflow semantics. This is a diagnostic workload, not a new public cursor
or host-computed result API.

## Maintained checks

Five fixed native tests cover empty input, zero repetitions, sorted complete
results, repeated enumeration, the length control and duplicate replacement. They
run through both evaluators as part of the ordinary project check.

`tests/native_map_entries.rs` performs fresh copied-product authoring and an
attached smoke invocation, then deletes its source project and standard transport.
Its detached matrix has 8 sizes (0, 1, 31, 32, 33, 256, 1,024 and 4,096), 4 insertion
orders/update patterns, 3 repetition counts (0, 1 and 8), and both modes: 192 full
result comparisons. These are production invocations, not 192 differential pairs.
A separate ordered-map oracle specifies every key and payload, including duplicate
replacement. The complete raw ingress count includes overwritten entries; unused
or physically aliased input is not exempted.

Each mode pair has identical construction and final projection. Subtracting the
length-control map visits from repeated enumeration bounds the extra traversal visits by `distinct_keys *
repetitions`. The current cursor observes that bound exactly; future valid reuse
may do less work without failing this public cost ceiling. Complete input admission remains required; raw-result
readmission, internal descendant checks, capture admission and key-buffer copying
stay absent from this closed checked path. A test-only cost predicate rejects
underchecking, excess work and arithmetic overflow rather than trusting log shape.

Four malformed-final-element invocations reject even with zero repetitions, each
followed by valid recovery. Three explicitly limited invocations separately test
instruction, allocated-byte and collection-item refusal, each without a result file
and followed by recovery. Successful calls leave no owned handles, locals, operands,
frames, type bindings, transactions, tasks or workers. Executable and artifact
identities are checked before and after the detached proof.

## Interpretation

The source-level work law isolates tree traversal; it does not remove persistent
map construction, entry-record allocation, list construction, preparation, startup
or result encoding. Shared nested payloads and keys are not copied deeply, but
boxed option/sum spines still reserve clone storage. Separate runtime tests cover
those spines, foreign origins, retained versions, exact quotas and cancellation.

A timing comparison must use the same artifact and literal inputs on copied
predecessor and candidate release-build executables, interleave sample order, keep
warmups and controls, verify complete results and avoid simultaneous owned builds.
Do not multiply tree-visit reductions into a claimed whole-language speedup.
