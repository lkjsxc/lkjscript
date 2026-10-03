# Native parallel work

This maintained example authors three exact packages through an ordinary copied
lkjscript executable. One generic contract selects two different owned carriers.
The same method implementations run serially, in a parallel pair, and in nested
groups. Their complete results agree with an independent Rust byte oracle in
`tests/public_cli/parallel_work.rs`. These are application/library programs, not
compiler self-hosting or adoption by lkjournal.

`contract.lkjc` owns `Work`, exact implementation forwarding and the three generic
builders. It can be accepted before any concrete implementation or caller exists.
`carriers.lkjc` supplies `Octets` for ByteBuffer and `Scalar` for OwnedI64Cell.
`application.lkjc` selects those exact implementations and publishes command
targets `serial`, `parallel` and `nested`. `http.lkjc` adds the fixed CPU endpoint.
All proposals enter through public plan/apply. Proposals and drafts are derived
inputs; each newly accepted graph is that program's editable meaning authority.

For an input length N, rounds R and seed S, generate original octets
`x[i] = (S + i) % 256`. Apply `y = (37 * y + 11) % 256` R times to each octet,
then reduce `sum((i + 1) * y[i])`. Native arithmetic uses ordinary multiply, add,
divide and subtract; the independent Rust oracle uses remainder and a direct loop.
The maintained workload uses N from 0 through 4096, R from 0 through 32 and S from
0 through 255. These are workload bounds, not new bounded language types.

Octets reads its original prefix through synchronous loans and appends transformed
octets to its consumed buffer. Its borrowed read method reduces the appended half.
Scalar independently generates and transforms the same octets while replacing
its fixed cell accumulator. Both results return to the parent through owned
custody; the parent reads them, freezes the buffer and extracts the cell. Buffer
growth may reallocate. Transfer preserves the allocation present at the transfer
boundary; this example makes no whole-algorithm zero-copy claim.

The command result contains the complete original-plus-transformed payload and
both independently computed checksums. The nested builder places each generic
worker in a further joined group with a task returning ordinary metadata. It
preserves the same payload and checksum while exercising three parallel scopes.

| N | R | S | Transformed prefix | Checksum | Payload bytes |
| ---: | ---: | ---: | --- | ---: | ---: |
| 0 | 3 | 0 | empty | 0 | 0 |
| 1 | 0 | 255 | 255 | 255 | 2 |
| 5 | 3 | 0 | 117,82,47,12,233 | 1635 | 10 |
| 5 | 3 | 73 | 122,87,52,17,238 | 1710 | 10 |
| 256 | 3 | 0 | 117,82,47,12,233,198,163,128 | 4184448 | 512 |
| 4096 | 32 | 0 | 160,33,162,35,164,37,166,39 | 1069109248 | 8192 |

## Author through the product

Work outside the compiler checkout with a copied compatible executable:

```sh
./lkjscript new contract --template minimal --name work-contract
./lkjscript new carriers --template command --name work-carriers
./lkjscript new application --template command --name work-application
```

The command starter selects an exact built-in supplier for the two packages using
standard operations. A `use std builtin` alias alone does not select a dependency.
For each proposal, prepend `request base=BASE` using that project's current
`status` revision. Save the resulting request under an owned local filename,
plan it, then apply that same request with the returned exact token:

```sh
./lkjscript --project contract status
./lkjscript --project contract change plan --input-file contract.request.lkjc
./lkjscript --project contract change apply --input-file contract.request.lkjc --plan CONTRACT_PLAN
./lkjscript --project contract check
./lkjscript --project contract package current export --kind transport --output contract.lkjp
```

Map that export to `CONTRACT_PACKAGE` (package id), `CONTRACT_REVISION` (semantic
revision), `CONTRACT_PACKAGE_REVISION` and its transport digest. Stage the exact
transport in both carriers and application with `package dependency stage
--transport DIGEST --input-file contract.lkjp`. Substitute only those observed
identities into the carriers proposal, then plan/apply/check/export it in the same
way. Stage its transport in application and substitute its export into
`CARRIERS_PACKAGE`, `CARRIERS_REVISION` and `CARRIERS_PACKAGE_REVISION` before
authoring the application proposal. Finally plan/apply/check the HTTP extension
against the application's newly observed base.

An untouched module draft must plan as unchanged. The public test also submits an
incorrect carrier witness and confirms rejection preserves accepted HEAD.

## Execute the same program

Build `work.lkja` and copy it, the executable and selected descriptors to a fresh
runtime directory. Authoring projects and package transports can then be removed.

```sh
./lkjscript --project application build --output work.lkja
./lkjscript run --deployment command.deployment.json --arguments '[4096,32,0]' --result-file result.json
./lkjscript serve --deployment service.deployment.json
```

The command descriptor selects `parallel`. A separately named descriptor changing
only the target to `serial` or `nested` executes the matched alternatives. Results
include all 8192 payload bytes and both sums 1069109248. Execution receipts report
joined cleanup and executor lifecycle separately from invocation work counters.

GET `/work` runs the same default computation and returns its complete JSON result.
The public test serves it beside a separately authored HTTP starter in one host,
sends four concurrent CPU requests while probing the independent instance, joins
the batch, then executes two sequential CPU requests before stopping the host and
repeating with a fresh host. With started workers, completed dispatches exceed
physical starts, proving reuse. A host unable to start workers explicitly records
inline execution instead. The test asserts source-free results, closed dispatch
and joined workers. This establishes useful sibling
service and lifecycle behavior; it does not establish preemption, general CPU
fairness, speedup or hostile-code isolation. Matched timings belong to
`docs/performance.md`, with original observations under `.artifacts/`.
